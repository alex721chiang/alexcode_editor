//! macOS：在 Finder 雙擊或用「打開方式」開檔時，系統不是把檔案放在命令列參數，
//! 而是送一個「開啟文件」的 Apple 事件（kAEOpenDocuments，`'odoc'`）給程式。
//! winit 0.30 沒有處理這個事件，所以這裡自己接：在 `NSApplicationWillFinishLaunchingNotification`
//! 時（AppKit 已裝好預設處理程式、但還沒處理任何事件）把 `'odoc'` 換成自己的處理程式，
//! 取出檔案路徑送進通道，再叫醒畫面，由 `AlexCodeApp` 在下一幀開啟。
//! 程式已經開著時再從 Finder 開檔，系統同樣送這個事件給現有的程式，不會另外啟動一個。

use crossbeam_channel::Sender;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{define_class, msg_send, sel, AllocAnyThread, DefinedClass};
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter, NSString};
use std::path::PathBuf;

const K_CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
const K_AE_OPEN_DOCUMENTS: u32 = u32::from_be_bytes(*b"odoc");
const KEY_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

pub struct Ivars {
    tx: Sender<Vec<PathBuf>>,
    wake: Box<dyn Fn()>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "AlexCodeOpenDocumentsHandler"]
    #[ivars = Ivars]
    pub struct OpenDocuments;

    impl OpenDocuments {
        #[unsafe(method(applicationWillFinishLaunching:))]
        fn will_finish_launching(&self, _note: &NSNotification) {
            self.install_event_handler();
        }

        #[unsafe(method(handleOpenDocuments:withReplyEvent:))]
        fn handle_open_documents(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            let files = paths_from_open_event(event);
            if !files.is_empty() && self.ivars().tx.send(files).is_ok() {
                (self.ivars().wake)();
            }
        }
    }
);

impl OpenDocuments {
    fn new(tx: Sender<Vec<PathBuf>>, wake: Box<dyn Fn()>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars { tx, wake });
        unsafe { msg_send![super(this), init] }
    }

    fn install_event_handler(&self) {
        let manager = NSAppleEventManager::sharedAppleEventManager();
        // objc2-foundation 的對應方法要另外開 objc2-core-services（只為了 AEEventClass 等 u32 型別），直接送訊息
        let _: () = unsafe {
            msg_send![
                &manager,
                setEventHandler: self,
                andSelector: sel!(handleOpenDocuments:withReplyEvent:),
                forEventClass: K_CORE_EVENT_CLASS,
                andEventID: K_AE_OPEN_DOCUMENTS
            ]
        };
    }
}

/// 在 `eframe::run_native` 之前（主執行緒）呼叫：之後 Finder 交來的檔案會送進 `tx`，並呼叫 `wake` 叫醒畫面
pub fn install(tx: Sender<Vec<PathBuf>>, wake: impl Fn() + 'static) {
    let handler = OpenDocuments::new(tx, Box::new(wake));
    let center = NSNotificationCenter::defaultCenter();
    let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
    unsafe {
        center.addObserver_selector_name_object(&handler, sel!(applicationWillFinishLaunching:), Some(&name), None);
    }
    // 通知中心與 Apple 事件管理員都不會保留這個物件：讓它一直存在到程式結束
    std::mem::forget(handler);
}

/// 「開啟文件」事件裡的檔案路徑（直接參數是檔案 URL 的清單；只有一個檔案時也可能不是清單）。資料夾與不存在的路徑略過
pub fn paths_from_open_event(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
    let direct: Option<Retained<NSAppleEventDescriptor>> =
        unsafe { msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT] };
    let Some(direct) = direct else {
        return Vec::new();
    };
    let n = direct.numberOfItems();
    let items: Vec<Retained<NSAppleEventDescriptor>> = if n <= 0 {
        vec![direct]
    } else {
        (1..=n).filter_map(|i| direct.descriptorAtIndex(i)).collect()
    };
    items
        .iter()
        .filter_map(|d| d.fileURLValue())
        .filter_map(|url| url.path())
        .map(|s| PathBuf::from(s.to_string()))
        .filter(|p| p.is_file())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2::ClassType;
    use objc2_foundation::NSURL;
    use std::cell::Cell;
    use std::rc::Rc;

    fn file_desc(p: &std::path::Path) -> Retained<NSAppleEventDescriptor> {
        let url = NSURL::fileURLWithPath(&NSString::from_str(p.to_str().unwrap()));
        NSAppleEventDescriptor::descriptorWithFileURL(&url)
    }

    /// 與 Finder 送來的一樣：aevt/odoc，直接參數是檔案 URL 清單
    fn open_event(direct: &NSAppleEventDescriptor) -> Retained<NSAppleEventDescriptor> {
        let event: Retained<NSAppleEventDescriptor> = unsafe {
            msg_send![
                NSAppleEventDescriptor::class(),
                appleEventWithEventClass: K_CORE_EVENT_CLASS,
                eventID: K_AE_OPEN_DOCUMENTS,
                targetDescriptor: None::<&NSAppleEventDescriptor>,
                returnID: -1i16, // kAutoGenerateReturnID
                transactionID: 0i32 // kAnyTransactionID
            ]
        };
        let _: () = unsafe { msg_send![&event, setParamDescriptor: direct, forKeyword: KEY_DIRECT_OBJECT] };
        event
    }

    fn canon(v: Vec<PathBuf>) -> Vec<PathBuf> {
        v.into_iter().map(|p| p.canonicalize().unwrap()).collect()
    }

    #[test]
    fn open_documents_event_yields_the_file_paths() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a b.txt");
        let b = dir.path().join("中文.md");
        std::fs::write(&a, "a").unwrap();
        std::fs::write(&b, "b").unwrap();
        let list = NSAppleEventDescriptor::listDescriptor();
        for p in [&a, dir.path(), &b, &dir.path().join("missing.txt")] {
            list.insertDescriptor_atIndex(&file_desc(p), 0);
        }
        let got = paths_from_open_event(&open_event(&list));
        assert_eq!(canon(got), canon(vec![a.clone(), b]), "files in order; the folder and the missing path are skipped");

        // 只有一個檔案、直接參數不是清單
        let got = paths_from_open_event(&open_event(&file_desc(&a)));
        assert_eq!(canon(got), canon(vec![a]));
    }

    #[test]
    fn handler_sends_the_files_and_wakes_the_ui() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        std::fs::write(&a, "a").unwrap();
        let (tx, rx) = crossbeam_channel::unbounded();
        let woke = Rc::new(Cell::new(0));
        let w = woke.clone();
        let handler = OpenDocuments::new(tx, Box::new(move || w.set(w.get() + 1)));
        let list = NSAppleEventDescriptor::listDescriptor();
        list.insertDescriptor_atIndex(&file_desc(&a), 0);
        let event = open_event(&list);
        let reply = NSAppleEventDescriptor::nullDescriptor();
        // 以 Apple 事件管理員會用的選擇器呼叫
        let _: () = unsafe { msg_send![&handler, handleOpenDocuments: &*event, withReplyEvent: &*reply] };
        assert_eq!(canon(rx.try_recv().unwrap()), canon(vec![a]));
        assert_eq!(woke.get(), 1);

        // 沒有檔案的事件不送、也不叫醒
        let empty = open_event(&NSAppleEventDescriptor::listDescriptor());
        let _: () = unsafe { msg_send![&handler, handleOpenDocuments: &*empty, withReplyEvent: &*reply] };
        assert!(rx.try_recv().is_err());
        assert_eq!(woke.get(), 1);
    }
}
