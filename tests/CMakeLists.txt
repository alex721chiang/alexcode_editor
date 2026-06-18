include(FetchContent)
FetchContent_Declare(googletest URL https://github.com/google/googletest/archive/03597a01ee50ed33e9dfd640b249b4be3799d395.zip)
FetchContent_MakeAvailable(googletest)
add_executable(AlexCodeTests FilterEngineTest.cpp PerformanceTest.cpp LspProtocolTest.cpp GitGutterTest.cpp VtParserTest.cpp BoxSelectTest.cpp LocalCompletionTest.cpp TextToolsTest.cpp TerminalSelectionTest.cpp TsCategoryTest.cpp MdLinkTest.cpp MarkdownLinkIndexTest.cpp GraphLayoutTest.cpp MarkdownRenderTest.cpp AutoPairTest.cpp TsEditTest.cpp ../src/FilterEngine.cpp ../src/LspProtocol.cpp ../src/GitGutter.cpp ../src/VtParser.cpp ../src/TextTools.cpp ../src/MarkdownLinkIndex.cpp ../src/GraphLayout.cpp)
target_link_libraries(AlexCodeTests PRIVATE gtest_main Qt6::Core)
add_test(NAME AlexCodeUnitTests COMMAND AlexCodeTests)   # 讓 ctest 能跑（CI 用）
# 無頭 LSP 整合煙霧測試（需系統已安裝 clangd；手動執行 ./tests/lsp_smoke）
add_executable(lsp_smoke lsp_smoke.cpp ../src/LspProtocol.cpp ../src/LspClient.cpp)
set_target_properties(lsp_smoke PROPERTIES AUTOMOC ON)
target_link_libraries(lsp_smoke PRIVATE Qt6::Core)

# 無頭 ConPTY 煙霧測試（Windows）
if(WIN32)
    add_executable(pty_smoke pty_smoke.cpp ../src/PtySession.cpp)
    set_target_properties(pty_smoke PROPERTIES AUTOMOC ON)
    target_compile_definitions(pty_smoke PRIVATE _WIN32_WINNT=0x0A00 WINVER=0x0A00 NTDDI_VERSION=0x0A000006)
    target_link_libraries(pty_smoke PRIVATE Qt6::Core)
endif()
