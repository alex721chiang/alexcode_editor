add_executable(AlexCode main.cpp MainWindow.cpp FilterEngine.cpp)
target_link_libraries(AlexCode PRIVATE Qt6::Widgets Qt6::Core)
