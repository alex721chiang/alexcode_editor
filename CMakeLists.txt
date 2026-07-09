cmake_minimum_required(VERSION 3.16)
project(AlexCode VERSION 4.8.1 LANGUAGES C CXX)   # C：tree-sitter 執行期與文法
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

# 原始碼為 UTF-8（含大量中文字串/註解）。MSVC 預設以系統代碼頁解讀，
# 會出現 C4819 警告甚至破壞字串字面值；強制 /utf-8。MinGW/GCC 預設即 UTF-8。
if(MSVC)
    add_compile_options(/utf-8)
endif()

find_package(Qt6 COMPONENTS Core Gui Widgets Network Concurrent Core5Compat Qml REQUIRED)   # Qml：QJSEngine（腳本外掛）

include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/TreeSitter.cmake)   # 語法樹高亮（tree-sitter + 文法）

add_subdirectory(src)
if(NOT CMAKE_CROSSCOMPILING)
    enable_testing()
    add_subdirectory(tests)
endif()
