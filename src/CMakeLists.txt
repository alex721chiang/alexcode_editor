set(LIB_SRCS
    MainWindow.cpp
    FilterEngine.cpp
    CodeEditor.cpp
    SyntaxHighlighter.cpp
    AICompletionProvider.cpp
    SuggestionWidget.cpp
    CallGraphWidget.cpp
    FindInFilesDialog.cpp
    QuickOpenDialog.cpp
    LspProtocol.cpp
    LspClient.cpp
    LspManager.cpp
    GitGutter.cpp
    TextTools.cpp
    SettingsDialog.cpp
    TreeSitterHighlighter.cpp
    MarkdownLinkIndex.cpp
    GraphLayout.cpp
    GraphView.cpp
    SymbolDialog.cpp
    CommandPalette.cpp
    TsSymbolParser.cpp
    ProjectSymbolIndex.cpp
    ProjectSymbolIndexBuild.cpp
    ProjectSymbolDialog.cpp
    ProjectSymbolController.cpp
    MarkdownLinkController.cpp
    RecentFilesController.cpp
    SnippetsController.cpp
    GitGutterController.cpp
    LspStatusController.cpp
    LspSyncController.cpp
    LspController.cpp
    FunctionListController.cpp
    FindController.cpp
    VtParser.cpp
    PtySession.cpp
    TerminalWidget.cpp
    TimelineBar.h
)

# 版本標頭（由 project() 版本產生），供關於對話框/標題使用
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/Version.h.in ${CMAKE_CURRENT_BINARY_DIR}/Version.h @ONLY)

# 靜態程式庫：主程式與 widget 測試（tests/）共用，避免測試各自撿 .cpp 造成依賴閉包不同步
add_library(alexcode_lib STATIC ${LIB_SRCS})
target_include_directories(alexcode_lib PUBLIC ${CMAKE_CURRENT_BINARY_DIR} ${TREESITTER_INCLUDE} ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(alexcode_lib PUBLIC Qt6::Widgets Qt6::Core Qt6::Gui Qt6::Network Qt6::Concurrent Qt6::Core5Compat ${TREESITTER_LIBS})

add_executable(AlexCode WIN32 main.cpp resources.qrc $<$<BOOL:${WIN32}>:alexcode.rc>)
target_link_libraries(AlexCode PRIVATE alexcode_lib)

# ConPTY（CreatePseudoConsole/HPCON）於 Win10 1809(RS5) 加入；MinGW 以 NTDDI_VERSION 把關
if(WIN32)
    target_compile_definitions(alexcode_lib PUBLIC
        _WIN32_WINNT=0x0A00 WINVER=0x0A00 NTDDI_VERSION=0x0A000006
        NOMINMAX)   # 避免 windows.h 的 min/max 巨集破壞 std::min/std::max（MSVC）
endif()

# 編譯警告（不加 -Werror，避免 CI 因既有警告變紅；逐步清理）
if(MSVC)
    target_compile_options(alexcode_lib PRIVATE /W3)
else()
    target_compile_options(alexcode_lib PRIVATE -Wall -Wextra)
endif()
