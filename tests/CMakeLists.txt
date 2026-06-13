include(FetchContent)
FetchContent_Declare(googletest URL https://github.com/google/googletest/archive/03597a01ee50ed33e9dfd640b249b4be3799d395.zip)
FetchContent_MakeAvailable(googletest)
add_executable(AlexCodeTests FilterEngineTest.cpp PerformanceTest.cpp LspProtocolTest.cpp GitGutterTest.cpp ../src/FilterEngine.cpp ../src/LspProtocol.cpp ../src/GitGutter.cpp)
target_link_libraries(AlexCodeTests PRIVATE gtest_main Qt6::Core)
# 無頭 LSP 整合煙霧測試（需系統已安裝 clangd；手動執行 ./tests/lsp_smoke）
add_executable(lsp_smoke lsp_smoke.cpp ../src/LspProtocol.cpp ../src/LspClient.cpp)
set_target_properties(lsp_smoke PROPERTIES AUTOMOC ON)
target_link_libraries(lsp_smoke PRIVATE Qt6::Core)
