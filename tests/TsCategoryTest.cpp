#include <gtest/gtest.h>
#include "../src/TsCategory.h"

using TsCategory::Category;
using TsCategory::forNode;
using TsCategory::isWhole;

// 匿名節點（named=false）：純字母且長度>=2 → 關鍵字；標點/單字元 → None
TEST(TsCategory, AnonymousKeywords) {
    EXPECT_EQ(forNode("if", false), Category::Keyword);
    EXPECT_EQ(forNode("return", false), Category::Keyword);
    EXPECT_EQ(forNode("namespace", false), Category::Keyword);
}

TEST(TsCategory, AnonymousPunctuation) {
    EXPECT_EQ(forNode("(", false), Category::None);
    EXPECT_EQ(forNode(";", false), Category::None);
    EXPECT_EQ(forNode("==", false), Category::None);
    EXPECT_EQ(forNode("{", false), Category::None);
}

TEST(TsCategory, Comments) {
    EXPECT_EQ(forNode("comment", true), Category::Comment);
}

TEST(TsCategory, Strings) {
    EXPECT_EQ(forNode("string_literal", true), Category::String);
    EXPECT_EQ(forNode("char_literal", true), Category::String);
    EXPECT_EQ(forNode("system_lib_string", true), Category::String);
    EXPECT_EQ(forNode("escape_sequence", true), Category::String);
}

TEST(TsCategory, Numbers) {
    EXPECT_EQ(forNode("number_literal", true), Category::Number);
    EXPECT_EQ(forNode("integer", true), Category::Number);
    EXPECT_EQ(forNode("float", true), Category::Number);
}

TEST(TsCategory, Types) {
    EXPECT_EQ(forNode("primitive_type", true), Category::Type);
    EXPECT_EQ(forNode("type_identifier", true), Category::Type);
    EXPECT_EQ(forNode("sized_type_specifier", true), Category::Type);
}

TEST(TsCategory, Preproc) {
    EXPECT_EQ(forNode("preproc_include", true), Category::Preproc);
    EXPECT_EQ(forNode("preproc_def", true), Category::Preproc);
    EXPECT_EQ(forNode("preproc_arg", true), Category::Preproc);
}

TEST(TsCategory, BooleanKeywords) {
    EXPECT_EQ(forNode("true", true), Category::Keyword);
    EXPECT_EQ(forNode("false", true), Category::Keyword);
    EXPECT_EQ(forNode("null", true), Category::Keyword);
    EXPECT_EQ(forNode("none", true), Category::Keyword);
}

TEST(TsCategory, NamedIdentifierIsNone) {
    EXPECT_EQ(forNode("identifier", true), Category::None);
    EXPECT_EQ(forNode("function_definition", true), Category::None);
    EXPECT_EQ(forNode("translation_unit", true), Category::None);
}

// 整段上色（不下探子節點）的類別
TEST(TsCategory, WholeCategories) {
    EXPECT_TRUE(isWhole(Category::Comment));
    EXPECT_TRUE(isWhole(Category::String));
    EXPECT_TRUE(isWhole(Category::Number));
    EXPECT_TRUE(isWhole(Category::Preproc));
    EXPECT_FALSE(isWhole(Category::Keyword));
    EXPECT_FALSE(isWhole(Category::Type));
    EXPECT_FALSE(isWhole(Category::None));
}
