# Tree-sitter 執行庫 + 文法（皆為可移植 C，MinGW/MSVC/gcc 皆可編）。
# tree-sitter 無 CMake，採單檔 amalgamation lib/src/lib.c；文法編 parser.c(+scanner.c)。
include(FetchContent)

FetchContent_Declare(treesitter
    GIT_REPOSITORY https://github.com/tree-sitter/tree-sitter.git
    GIT_TAG v0.22.6)
FetchContent_GetProperties(treesitter)
if(NOT treesitter_POPULATED)
    FetchContent_Populate(treesitter)
endif()
add_library(treesitter STATIC ${treesitter_SOURCE_DIR}/lib/src/lib.c)
target_include_directories(treesitter
    PUBLIC ${treesitter_SOURCE_DIR}/lib/include
    PRIVATE ${treesitter_SOURCE_DIR}/lib/src)

# 一個文法 → 一個 static lib（自動偵測有無 external scanner）
function(add_ts_grammar name repo tag)
    FetchContent_Declare(${name} GIT_REPOSITORY ${repo} GIT_TAG ${tag})
    FetchContent_GetProperties(${name})
    if(NOT ${name}_POPULATED)
        FetchContent_Populate(${name})
    endif()
    set(srcs ${${name}_SOURCE_DIR}/src/parser.c)
    if(EXISTS ${${name}_SOURCE_DIR}/src/scanner.c)
        list(APPEND srcs ${${name}_SOURCE_DIR}/src/scanner.c)
    endif()
    add_library(${name} STATIC ${srcs})
    target_include_directories(${name} PRIVATE ${${name}_SOURCE_DIR}/src)
    # 文法是第三方 C，關掉警告避免洗版面
    if(NOT MSVC)
        target_compile_options(${name} PRIVATE -w)
    endif()
endfunction()

add_ts_grammar(tree_sitter_cpp        https://github.com/tree-sitter/tree-sitter-cpp.git        v0.22.0)
add_ts_grammar(tree_sitter_python     https://github.com/tree-sitter/tree-sitter-python.git     v0.21.0)
add_ts_grammar(tree_sitter_javascript https://github.com/tree-sitter/tree-sitter-javascript.git v0.21.2)
add_ts_grammar(tree_sitter_json       https://github.com/tree-sitter/tree-sitter-json.git       v0.21.0)

set(TREESITTER_LIBS treesitter tree_sitter_cpp tree_sitter_python tree_sitter_javascript tree_sitter_json
    CACHE INTERNAL "tree-sitter libs")
set(TREESITTER_INCLUDE ${treesitter_SOURCE_DIR}/lib/include CACHE INTERNAL "tree-sitter include")
