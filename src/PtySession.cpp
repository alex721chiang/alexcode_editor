#include "PtySession.h"
#include <vector>

PtySession::PtySession(QObject* parent) : QObject(parent) {}
PtySession::~PtySession() { stop(); }

#ifdef Q_OS_WIN

bool PtySession::start(const QString& program, const QString& workingDir, int cols, int rows) {
    if (m_running) return false;
    HANDLE inRead = nullptr, outWrite = nullptr;
    if (!CreatePipe(&inRead, &m_inWrite, nullptr, 0)) return false;
    if (!CreatePipe(&m_outRead, &outWrite, nullptr, 0)) { CloseHandle(inRead); return false; }

    COORD size{ static_cast<SHORT>(qMax(cols, 1)), static_cast<SHORT>(qMax(rows, 1)) };
    HRESULT hr = CreatePseudoConsole(size, inRead, outWrite, 0, &m_hPC);
    CloseHandle(inRead);          // ConPTY 已持有子行程端
    CloseHandle(outWrite);
    if (FAILED(hr)) { stop(); return false; }

    STARTUPINFOEXW si{};
    si.StartupInfo.cb = sizeof(STARTUPINFOEXW);
    SIZE_T bytes = 0;
    InitializeProcThreadAttributeList(nullptr, 1, 0, &bytes);
    m_attrList = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(
        HeapAlloc(GetProcessHeap(), 0, bytes));
    if (!m_attrList || !InitializeProcThreadAttributeList(m_attrList, 1, 0, &bytes)) { stop(); return false; }
    if (!UpdateProcThreadAttribute(m_attrList, 0, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
                                   m_hPC, sizeof(m_hPC), nullptr, nullptr)) { stop(); return false; }
    si.lpAttributeList = m_attrList;

    std::wstring wcmd = program.toStdWString();
    std::vector<wchar_t> cmdline(wcmd.begin(), wcmd.end());
    cmdline.push_back(L'\0');
    const std::wstring wdir = workingDir.toStdWString();

    PROCESS_INFORMATION pi{};
    BOOL ok = CreateProcessW(nullptr, cmdline.data(), nullptr, nullptr, FALSE,
                             EXTENDED_STARTUPINFO_PRESENT, nullptr,
                             wdir.empty() ? nullptr : wdir.c_str(),
                             &si.StartupInfo, &pi);
    if (!ok) { stop(); return false; }
    m_hProcess = pi.hProcess;
    m_hThread  = pi.hThread;

    m_running = true;
    m_reader = std::thread(&PtySession::readerLoop, this);
    return true;
}

void PtySession::readerLoop() {
    m_readerTid = GetCurrentThreadId();  // 給 stop() 的 CancelSynchronousIo 用（MinGW 的
                                         // std::thread::native_handle() 不是 Win32 HANDLE）
    const DWORD BUFSZ = 4096;
    std::vector<char> buf(BUFSZ);
    while (m_running && !m_stopping) {
        DWORD n = 0;
        BOOL ok = ReadFile(m_outRead, buf.data(), BUFSZ, &n, nullptr);
        if (!ok || n == 0 || m_stopping) break;
        emit dataReceived(QByteArray(buf.data(), static_cast<int>(n)));
    }
    m_running = false;
    if (!m_stopping) emit exited();      // 由 stop() 主動關閉時不再發訊號
    m_readerDone = true;                 // 執行緒真正跑完，stop() 才可以安全 CloseHandle
}

void PtySession::writeData(const QByteArray& data) {
    if (m_inWrite && !data.isEmpty()) {
        DWORD written = 0;
        WriteFile(m_inWrite, data.constData(), static_cast<DWORD>(data.size()), &written, nullptr);
    }
}

void PtySession::resize(int cols, int rows) {
    if (m_hPC) {
        COORD s{ static_cast<SHORT>(qMax(cols, 1)), static_cast<SHORT>(qMax(rows, 1)) };
        ResizePseudoConsole(m_hPC, s);
    }
}

bool PtySession::isPlatformSupported() { return true; }

void PtySession::stop() {
    if (m_stopping.exchange(true)) return;   // 僅執行一次
    m_running = false;
    if (m_reader.joinable()) {
        // 中斷 reader 執行緒中可能正阻塞的 ReadFile：用 CancelSynchronousIo 對該執行緒
        // 送出取消，而不是像舊版那樣直接在這個執行緒 CloseHandle(m_outRead) 逼 ReadFile 返回——
        // 後者是未定義行為：handle 數值在關閉後可能被系統重新分配給別的物件，若 reader
        // 執行緒剛好在那個時間點做 ReadFile，就可能讀到錯誤的 handle，導致偶發 crash 或卡死。
        //
        // CancelSynchronousIo 與上面的 while 判斷之間仍有極窄的競態窗口（reader 剛通過
        // while 檢查、還沒真正呼叫到 ReadFile，這時取消會因為「沒有待處理的 I/O」而失敗）。
        // 用短暫重試涵蓋：只要 reader 還沒真正跑完（m_readerDone），就持續嘗試取消。
        //
        // 註：不能用 m_reader.native_handle() —— MinGW（winpthreads）回傳的是 pthread
        // 控制代碼而非 Win32 HANDLE，無法轉型。改由 reader 執行緒自報 thread id，
        // 這裡用 OpenThread 取得真正的 HANDLE（CancelSynchronousIo 需要 THREAD_TERMINATE 權限）。
        while (!m_readerDone.load()) {
            if (const DWORD tid = m_readerTid.load()) {
                if (HANDLE th = OpenThread(THREAD_TERMINATE, FALSE, tid)) {
                    CancelSynchronousIo(th);
                    CloseHandle(th);
                }
            }
            Sleep(1);
        }
        m_reader.join();
    }
    if (m_hPC)     { ClosePseudoConsole(m_hPC); m_hPC = nullptr; }   // 關閉 PC → 子行程收到結束
    if (m_outRead) { CloseHandle(m_outRead); m_outRead = nullptr; }  // reader 已確定跑完，此時關閉安全
    if (m_inWrite) { CloseHandle(m_inWrite); m_inWrite = nullptr; }
    if (m_hProcess){ TerminateProcess(m_hProcess, 0); CloseHandle(m_hProcess); m_hProcess = nullptr; }
    if (m_hThread) { CloseHandle(m_hThread); m_hThread = nullptr; }
    if (m_attrList){ DeleteProcThreadAttributeList(m_attrList);
                     HeapFree(GetProcessHeap(), 0, m_attrList); m_attrList = nullptr; }
}

#else   // 非 Windows：佔位（本專案目前僅 Windows）

bool PtySession::start(const QString&, const QString&, int, int) { return false; }
void PtySession::readerLoop() {}
void PtySession::writeData(const QByteArray&) {}
void PtySession::resize(int, int) {}
void PtySession::stop() { if (m_reader.joinable()) m_reader.join(); }
bool PtySession::isPlatformSupported() { return false; }

#endif
