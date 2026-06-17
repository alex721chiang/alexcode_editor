set(SRCS 
    main.cpp 
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
    VtParser.cpp
    PtySession.cpp
    TerminalWidget.cpp
    TimelineBar.h
    resources.qrc
)

if(WIN32)
    set(SRCS ${SRCS} alexcode.rc)
endif()

# 版本標頭（由 project() 版本產生），供關於對話框/標題使用
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/Version.h.in ${CMAKE_CURRENT_BINARY_DIR}/Version.h @ONLY)

add_executable(AlexCode WIN32 ${SRCS})
target_include_directories(AlexCode PRIVATE ${CMAKE_CURRENT_BINARY_DIR} ${TREESITTER_INCLUDE})
target_link_libraries(AlexCode PRIVATE Qt6::Widgets Qt6::Core Qt6::Gui Qt6::Network Qt6::Concurrent Qt6::Core5Compat ${TREESITTER_LIBS})

# ConPTY（CreatePseudoConsole/HPCON）於 Win10 1809(RS5) 加入；MinGW 以 NTDDI_VERSION 把關
if(WIN32)
    target_compile_definitions(AlexCode PRIVATE
        _WIN32_WINNT=0x0A00 WINVER=0x0A00 NTDDI_VERSION=0x0A000006
        NOMINMAX)   # 避免 windows.h 的 min/max 巨集破壞 std::min/std::max（MSVC）
endif()

# 編譯警告（不加 -Werror，避免 CI 因既有警告變紅；逐步清理）
if(MSVC)
    target_compile_options(AlexCode PRIVATE /W3)
else()
    target_compile_options(AlexCode PRIVATE -Wall -Wextra)
endif()