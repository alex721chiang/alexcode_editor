set(SRCS 
    main.cpp 
    MainWindow.cpp 
    FilterEngine.cpp 
    CodeEditor.cpp 
    SyntaxHighlighter.cpp 
    FindInFilesDialog.cpp
)

if(WIN32)
    set(SRCS ${SRCS} alexcode.rc)
endif()

add_executable(AlexCode WIN32 ${SRCS})
target_link_libraries(AlexCode PRIVATE Qt6::Widgets Qt6::Core Qt6::Gui)