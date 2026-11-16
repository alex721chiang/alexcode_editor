//! 語言伺服器管理 (LspManager)
//!
//! 依檔案類型啟動語言伺服器（每一種一個：`.rs` → rust-analyzer、C / C++ → clangd、`.py` → pylsp），
//! 完成 initialize 握手，並把開著的文件同步給它：didOpen、內容變了就 didChange（送整份內容）、
//! 分頁關掉就 didClose。伺服器送來的請求（例如 `window/workDoneProgress/create`）一律回覆，
//! 免得伺服器一直等。介面每一幀呼叫 `sync`。

use super::client::LspClient;
use super::protocol::{make_notification, make_request, make_response, uri_from_path};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// 支援的語言伺服器
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerKind {
    RustAnalyzer,
    Clangd,
    Pylsp,
}

impl ServerKind {
    pub const ALL: [ServerKind; 3] = [ServerKind::RustAnalyzer, ServerKind::Clangd, ServerKind::Pylsp];

    fn ext(path: &str) -> String {
        Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
    }

    /// 這個檔案由哪一種伺服器處理（不支援的類型是 None）
    pub fn for_path(path: &str) -> Option<Self> {
        match Self::ext(path).as_str() {
            "rs" => Some(Self::RustAnalyzer),
            "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Some(Self::Clangd),
            "py" | "pyw" => Some(Self::Pylsp),
            _ => None,
        }
    }

    /// 伺服器的程式名稱（在 PATH 裡找）
    pub fn program(self) -> &'static str {
        match self {
            Self::RustAnalyzer => "rust-analyzer",
            Self::Clangd => "clangd",
            Self::Pylsp => "pylsp",
        }
    }

    /// didOpen 時告訴伺服器的語言代號
    pub fn language_id(path: &str) -> &'static str {
        match Self::ext(path).as_str() {
            "rs" => "rust",
            "c" | "h" => "c",
            "py" | "pyw" => "python",
            _ => "cpp",
        }
    }
}

/// 一個語言伺服器目前的狀態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspStatus {
    /// 已啟動、等 initialize 的回覆
    Starting,
    /// 握手完成，文件會同步給它
    Ready,
    /// 找不到伺服器的程式
    NotInstalled,
    /// 啟動或握手失敗
    Failed(String),
    /// 伺服器自己結束了（不會自動重新啟動）
    Exited,
}

impl LspStatus {
    /// 狀態列上的文字
    pub fn label(&self) -> String {
        match self {
            Self::Starting => "啟動中…".into(),
            Self::Ready => "已連線".into(),
            Self::NotInstalled => "未安裝".into(),
            Self::Failed(e) => format!("錯誤：{e}"),
            Self::Exited => "已結束".into(),
        }
    }
}

/// 啟動某一種伺服器的方式（預設是 `spawn_server`；測試接上假的伺服器）
pub type Launcher = Box<dyn Fn(ServerKind, &Path) -> std::io::Result<LspClient>>;

/// 預設的啟動方式：執行 PATH 裡的伺服器程式
pub fn spawn_server(kind: ServerKind, _root: &Path) -> std::io::Result<LspClient> {
    LspClient::spawn(kind.program(), &[])
}

/// `sync` 用的一份開著的文件
pub struct OpenDoc {
    pub path: String,
    /// 內容版本：變了才送 didChange
    pub revision: u64,
}

struct Server {
    client: Option<LspClient>,
    status: LspStatus,
    init_id: i64,
    /// 已經送 didOpen 的文件：uri → (送給伺服器的版本號, 那時的內容版本)
    docs: HashMap<String, (i64, u64)>,
}

impl Server {
    /// 讀進伺服器送來的訊息：initialize 的回覆、伺服器的請求；伺服器結束時改成 `Exited`
    fn poll(&mut self) {
        let Some(c) = &mut self.client else { return };
        while let Some(msg) = c.try_recv_message() {
            let method = msg.get("method").and_then(|m| m.as_str());
            match (method, msg.get("id")) {
                // 伺服器的請求：一律回覆（設定一律「沒有」，其他回 null）
                (Some(method), Some(id)) => {
                    let result = if method == "workspace/configuration" {
                        let n = msg["params"]["items"].as_array().map_or(0, |a| a.len());
                        Value::Array(vec![Value::Null; n])
                    } else {
                        Value::Null
                    };
                    let _ = c.send_message(&make_response(id.clone(), result));
                }
                (None, Some(id)) if id.as_i64() == Some(self.init_id) && self.status == LspStatus::Starting => {
                    if let Some(err) = msg.get("error") {
                        let text = err["message"].as_str().unwrap_or("initialize 失敗").to_string();
                        self.status = LspStatus::Failed(text);
                    } else if c.send_message(&make_notification("initialized", json!({}))).is_ok() {
                        self.status = LspStatus::Ready;
                    } else {
                        self.status = LspStatus::Failed("無法送出 initialized".into());
                    }
                }
                _ => {}
            }
        }
        if c.is_closed() {
            if matches!(self.status, LspStatus::Starting | LspStatus::Ready) {
                self.status = LspStatus::Exited;
            }
            self.client = None;
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        if let Some(c) = &mut self.client {
            if c.send_message(&make_notification(method, params)).is_err() {
                self.status = LspStatus::Exited;
                self.client = None;
            }
        }
    }
}

pub struct LspManager {
    root: PathBuf,
    launcher: Launcher,
    servers: HashMap<ServerKind, Server>,
}

impl LspManager {
    /// `root` 是專案資料夾（initialize 的 rootUri；之後才啟動的伺服器用 `set_root` 設的）
    pub fn new(root: PathBuf, launcher: Launcher) -> Self {
        Self { root, launcher, servers: HashMap::new() }
    }

    pub fn set_root(&mut self, root: PathBuf) {
        self.root = root;
    }

    /// 某一種伺服器的狀態（還沒需要用到、所以沒啟動過時是 None）
    pub fn status(&self, kind: ServerKind) -> Option<&LspStatus> {
        self.servers.get(&kind).map(|s| &s.status)
    }

    /// 有伺服器在等 initialize 的回覆（介面要繼續重畫，才會看到它就緒）
    pub fn is_starting(&self) -> bool {
        self.servers.values().any(|s| s.status == LspStatus::Starting)
    }

    fn start(&mut self, kind: ServerKind) {
        let server = match (self.launcher)(kind, &self.root) {
            Ok(mut c) => {
                let id = c.next_request_id();
                let root_uri = uri_from_path(&self.root.to_string_lossy());
                let name = self.root.file_name().map_or("workspace".to_string(), |n| n.to_string_lossy().into_owned());
                let params = json!({
                    "processId": std::process::id(),
                    "clientInfo": { "name": "AlexCode" },
                    "rootUri": root_uri,
                    "workspaceFolders": [{ "uri": root_uri, "name": name }],
                    "capabilities": {
                        "textDocument": {
                            "synchronization": { "dynamicRegistration": false, "didSave": false },
                            "publishDiagnostics": {}
                        },
                        "workspace": { "workspaceFolders": true, "configuration": true }
                    }
                });
                // 寫不進去表示伺服器一啟動就結束了（例如 rustup 的 rust-analyzer 沒有裝那個元件）
                match c.send_message(&make_request(id, "initialize", params)) {
                    Ok(()) => Server { client: Some(c), status: LspStatus::Starting, init_id: id, docs: HashMap::new() },
                    Err(_) => Server { client: None, status: LspStatus::Exited, init_id: id, docs: HashMap::new() },
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Server { client: None, status: LspStatus::NotInstalled, init_id: 0, docs: HashMap::new() }
            }
            Err(e) => Server { client: None, status: LspStatus::Failed(e.to_string()), init_id: 0, docs: HashMap::new() },
        };
        self.servers.insert(kind, server);
    }

    /// 每一幀呼叫：需要的伺服器還沒啟動就啟動、讀進伺服器送來的訊息，再把 `docs`（目前開著的文件）同步給
    /// 已就緒的伺服器。`text(i)` 取得 `docs[i]` 的全文（只在要送的時候才呼叫）
    pub fn sync(&mut self, docs: &[OpenDoc], text: impl Fn(usize) -> String) {
        for d in docs {
            if let Some(kind) = ServerKind::for_path(&d.path) {
                if !self.servers.contains_key(&kind) {
                    self.start(kind);
                }
            }
        }
        for (kind, s) in self.servers.iter_mut() {
            s.poll();
            if s.status != LspStatus::Ready {
                continue;
            }
            let mine: Vec<(usize, String, &OpenDoc)> = docs
                .iter()
                .enumerate()
                .filter(|(_, d)| ServerKind::for_path(&d.path) == Some(*kind))
                .map(|(i, d)| (i, uri_from_path(&d.path), d))
                .collect();
            // 分頁關掉了：didClose
            let open: HashSet<&str> = mine.iter().map(|(_, u, _)| u.as_str()).collect();
            let gone: Vec<String> = s.docs.keys().filter(|u| !open.contains(u.as_str())).cloned().collect();
            for uri in gone {
                s.docs.remove(&uri);
                s.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri } }));
            }
            for (i, uri, d) in mine {
                match s.docs.get(&uri).copied() {
                    None => {
                        let params = json!({ "textDocument": {
                            "uri": uri, "languageId": ServerKind::language_id(&d.path), "version": 1, "text": text(i)
                        }});
                        s.docs.insert(uri, (1, d.revision));
                        s.notify("textDocument/didOpen", params);
                    }
                    Some((version, rev)) if rev != d.revision => {
                        let params = json!({
                            "textDocument": { "uri": uri, "version": version + 1 },
                            "contentChanges": [{ "text": text(i) }]
                        });
                        s.docs.insert(uri, (version + 1, d.revision));
                        s.notify("textDocument/didChange", params);
                    }
                    Some(_) => {}
                }
            }
        }
    }
}
