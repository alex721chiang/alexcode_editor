add_executable(AlexCode main.cpp MainWindow.cpp FilterEngine.cpp CodeEditor.cpp SyntaxHighlighter.cpp)
target_link_libraries(AlexCode PRIVATE Qt6::Widgets Qt6::Core Qt6::Gui)
