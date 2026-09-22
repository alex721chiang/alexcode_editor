# src/ 依職責分組為子目錄；原始碼內 #include 維持扁平寫法（各子目錄皆在 include path）。
set(LIB_SRCS
    # core：主視窗、程式進入點、主題、設定、命令面板
    core/MainWindow.cpp
    core/SettingsDialog.cpp
    core/NeonIcons.cpp
    core/HudOverlay.cpp
    core/CommandPalette.cpp
    core/RecentFilesController.cpp
    core/ScreenshotHelper.cpp
    # editor：編輯器本體與周邊（高亮、配對、選取、工具）
    editor/CodeEditor.cpp
    editor/SyntaxHighlighter.cpp
    editor/TreeSitterHighlighter.cpp
    editor/TextTools.cpp
    editor/FunctionListController.cpp
    editor/SnippetsController.cpp
    # lsp：語言伺服器協定與控制器
    lsp/LspProtocol.cpp
    lsp/LspClient.cpp
    lsp/LspManager.cpp
    lsp/LspController.cpp
    lsp/LspStatusController.cpp
    lsp/LspSyncController.cpp
    # git：gutter、diff、blame
    git/GitGutter.cpp
    git/GitGutterController.cpp
    git/DiffCalc.cpp
    git/DiffViewer.cpp
    git/GitBlame.cpp
    # terminal：VT 解析、PTY、終端機 widget
    terminal/VtParser.cpp
    terminal/PtySession.cpp
    terminal/TerminalWidget.cpp
    # search：篩選引擎、檔案內搜尋、快速開啟
    search/FilterEngine.cpp
    search/FindInFilesDialog.cpp
    search/QuickOpenDialog.cpp
    search/FindController.cpp
    # index：符號/連結索引、關係圖、對話框
    index/MarkdownLinkIndex.cpp
    index/GraphLayout.cpp
    index/GraphView.cpp
    index/ProjectSymbolIndex.cpp
    index/ProjectSymbolIndexBuild.cpp
    index/TsSymbolParser.cpp
    index/MarkdownLinkController.cpp
    index/ProjectSymbolController.cpp
    index/ProjectSymbolDialog.cpp
    index/CallGraphWidget.cpp
    index/SymbolDialog.cpp
    # ai：AI 補全
    ai/AICompletionProvider.cpp
    ai/SuggestionWidget.cpp
    # plugins：外掛系統
    plugins/PluginManager.cpp
)

# 版本標頭（由 project() 版本產生），供關於對話框/標題使用
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/core/Version.h.in ${CMAKE_CURRENT_BINARY_DIR}/Version.h @ONLY)

# 靜態程式庫：主程式與 widget 測試（tests/）共用，避免測試各自撿 .cpp 造成依賴閉包不同步
add_library(alexcode_lib STATIC ${LIB_SRCS})
target_include_directories(alexcode_lib PUBLIC
    ${CMAKE_CURRENT_BINARY_DIR}
    ${TREESITTER_INCLUDE}
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/core
    ${CMAKE_CURRENT_SOURCE_DIR}/editor
    ${CMAKE_CURRENT_SOURCE_DIR}/lsp
    ${CMAKE_CURRENT_SOURCE_DIR}/git
    ${CMAKE_CURRENT_SOURCE_DIR}/terminal
    ${CMAKE_CURRENT_SOURCE_DIR}/search
    ${CMAKE_CURRENT_SOURCE_DIR}/index
    ${CMAKE_CURRENT_SOURCE_DIR}/ai
    ${CMAKE_CURRENT_SOURCE_DIR}/plugins
)
target_link_libraries(alexcode_lib PUBLIC Qt6::Widgets Qt6::Core Qt6::Gui Qt6::Network Qt6::Concurrent Qt6::Core5Compat Qt6::Qml ${TREESITTER_LIBS})
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    target_link_libraries(alexcode_lib PUBLIC util)   # forkpty（舊 glibc 在 libutil；2.34 起為空 stub）
endif()

add_executable(AlexCode WIN32 core/main.cpp resources.qrc $<$<BOOL:${WIN32}>:alexcode.rc>)
target_link_libraries(AlexCode PRIVATE alexcode_lib)

# i18n：.ts 在版控，.qm 由建置期自動產生並嵌入 :/i18n（main.cpp 以 :/i18n/app_<lang>.qm 載入）
qt_add_translations(alexcode_lib
    TS_FILES
        i18n/app_en.ts
        i18n/app_zh_TW.ts
    RESOURCE_PREFIX "/i18n"
)

# ConPTY（CreatePseudoConsole/HPCON）於 Win10 1809(RS5) 加入；MinGW 以 NTDDI_VERSION 把關
if(WIN32)
    target_compile_definitions(alexcode_lib PUBLIC
        _WIN32_WINNT=0x0A00 WINVER=0x0A00 NTDDI_VERSION=0x0A000006
        NOMINMAX)   # 避免 windows.h 的 min/max 巨集破壞 std::min/std::max（MSVC）
    target_link_libraries(alexcode_lib PUBLIC dwmapi)   # 標題列著色（DwmSetWindowAttribute）
endif()

# 編譯警告（不加 -Werror，避免 CI 因既有警告變紅；逐步清理）
if(MSVC)
    target_compile_options(alexcode_lib PRIVATE /W3)
else()
    target_compile_options(alexcode_lib PRIVATE -Wall -Wextra)
endif()
