add_executable(AlexCode WIN32 main.cpp MainWindow.cpp FilterEngine.cpp CodeEditor.cpp SyntaxHighlighter.cpp FindInFilesDialog.cpp)
target_link_libraries(AlexCode PRIVATE Qt6::Widgets Qt6::Core Qt6::Gui)
