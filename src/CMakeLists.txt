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
    TimelineBar.h
    resources.qrc
)

if(WIN32)
    set(SRCS ${SRCS} alexcode.rc)
endif()

add_executable(AlexCode WIN32 ${SRCS})
target_link_libraries(AlexCode PRIVATE Qt6::Widgets Qt6::Core Qt6::Gui Qt6::Network Qt6::Concurrent Qt6::Core5Compat)