// Widget 測試進入點：CodeEditor 等 QWidget 需要 QApplication 才能建構。
// 強制 offscreen 平台，CI（無桌面）與本機（桌面鎖定）都能穩定執行。
#include <gtest/gtest.h>
#include <QApplication>

int main(int argc, char** argv) {
    qputenv("QT_QPA_PLATFORM", "offscreen");
    QApplication app(argc, argv);
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}
