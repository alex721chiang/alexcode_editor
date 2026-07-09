include(FetchContent)
FetchContent_Declare(googletest URL https://github.com/google/googletest/archive/03597a01ee50ed33e9dfd640b249b4be3799d395.zip)
FetchContent_MakeAvailable(googletest)
add_executable(AlexCodeTests FilterEngineTest.cpp PerformanceTest.cpp LspProtocolTest.cpp GitGutterTest.cpp VtParserTest.cpp BoxSelectTest.cpp LocalCompletionTest.cpp TextToolsTest.cpp TerminalSelectionTest.cpp TsCategoryTest.cpp MdLinkTest.cpp MarkdownLinkIndexTest.cpp GraphLayoutTest.cpp MarkdownRenderTest.cpp AutoPairTest.cpp TsEditTest.cpp TsSymbolsTest.cpp FileTierTest.cpp StickyScrollTest.cpp CommandMatchTest.cpp ProjectSymbolIndexTest.cpp TextRefsTest.cpp DiffCalcTest.cpp GitBlameTest.cpp ../src/FilterEngine.cpp ../src/ProjectSymbolIndex.cpp ../src/LspProtocol.cpp ../src/GitGutter.cpp ../src/VtParser.cpp ../src/TextTools.cpp ../src/MarkdownLinkIndex.cpp ../src/GraphLayout.cpp ../src/DiffCalc.cpp ../src/GitBlame.cpp)
target_link_libraries(AlexCodeTests PRIVATE gtest_main Qt6::Core)
add_test(NAME AlexCodeUnitTests COMMAND AlexCodeTests)   # 讓 ctest 能跑（CI 用）

# Widget 測試：連 alexcode_lib（含 CodeEditor 全依賴），自帶 QApplication(offscreen) main
# Qt6::Test 提供 QTest::keyClick 等鍵盤模擬（多游標測試用）
find_package(Qt6 COMPONENTS Test REQUIRED)
add_executable(AlexCodeWidgetTests WidgetTestMain.cpp EditorOpsTest.cpp PluginManagerTest.cpp AiCompletionTest.cpp)
target_link_libraries(AlexCodeWidgetTests PRIVATE gtest alexcode_lib Qt6::Test)
add_test(NAME AlexCodeWidgetTests COMMAND AlexCodeWidgetTests)
# 無頭 LSP 整合煙霧測試（需系統已安裝 clangd；手動執行 ./tests/lsp_smoke）
add_executable(lsp_smoke lsp_smoke.cpp ../src/LspProtocol.cpp ../src/LspClient.cpp)
set_target_properties(lsp_smoke PROPERTIES AUTOMOC ON)
target_link_libraries(lsp_smoke PRIVATE Qt6::Core)

# 無頭 PTY 煙霧測試（Windows=ConPTY、POSIX=forkpty）
add_executable(pty_smoke pty_smoke.cpp ../src/PtySession.cpp)
set_target_properties(pty_smoke PROPERTIES AUTOMOC ON)
target_link_libraries(pty_smoke PRIVATE Qt6::Core)
if(WIN32)
    target_compile_definitions(pty_smoke PRIVATE _WIN32_WINNT=0x0A00 WINVER=0x0A00 NTDDI_VERSION=0x0A000006)
else()
    # 舊 glibc 的 forkpty 在 libutil（glibc 2.34 起併入 libc，連結空 stub 也無害）；
    # macOS 在 libc 本體，不需額外連結
    if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
        target_link_libraries(pty_smoke PRIVATE util)
    endif()
    # 只在 POSIX 註冊進 ctest：CI 真正跑 forkpty 驗證行為。
    # Windows 不註冊——ConPTY 在無頭/桌面鎖定環境行為不穩定（會誤報紅），僅供手動執行。
    add_test(NAME PtySmoke COMMAND pty_smoke)
endif()
