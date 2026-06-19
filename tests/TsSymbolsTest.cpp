#include <gtest/gtest.h>
#include "../src/TsSymbols.h"

using TsSymbols::symbolKind;
using TsSymbols::isSymbolNode;

TEST(TsSymbols, FunctionKinds) {
    EXPECT_EQ(symbolKind("function_definition"), QString("function"));   // C/C++/Python
    EXPECT_EQ(symbolKind("function_declaration"), QString("function"));  // JS
    EXPECT_EQ(symbolKind("generator_function_declaration"), QString("function"));
    EXPECT_EQ(symbolKind("method_definition"), QString("method"));
}

TEST(TsSymbols, ClassKinds) {
    EXPECT_EQ(symbolKind("class_specifier"), QString("class"));    // C++
    EXPECT_EQ(symbolKind("class_definition"), QString("class"));   // Python
    EXPECT_EQ(symbolKind("class_declaration"), QString("class"));  // JS
}

TEST(TsSymbols, OtherKinds) {
    EXPECT_EQ(symbolKind("struct_specifier"), QString("struct"));
    EXPECT_EQ(symbolKind("union_specifier"), QString("union"));
    EXPECT_EQ(symbolKind("enum_specifier"), QString("enum"));
    EXPECT_EQ(symbolKind("namespace_definition"), QString("namespace"));
}

TEST(TsSymbols, NonSymbolsReturnEmpty) {
    EXPECT_TRUE(symbolKind("identifier").isEmpty());
    EXPECT_TRUE(symbolKind("if_statement").isEmpty());
    EXPECT_TRUE(symbolKind("translation_unit").isEmpty());
    EXPECT_TRUE(symbolKind("comment").isEmpty());
}

TEST(TsSymbols, IsSymbolNode) {
    EXPECT_TRUE(isSymbolNode("function_definition"));
    EXPECT_TRUE(isSymbolNode("class_specifier"));
    EXPECT_FALSE(isSymbolNode("identifier"));
    EXPECT_FALSE(isSymbolNode("expression_statement"));
}
