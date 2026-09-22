#pragma once
#include <QWidget>

// Neon HUD 專屬的非互動覆蓋層：在編輯區四角畫 cyan 角標（航太儀表 HUD 感）。
// 滑鼠事件穿透、背景透明，只在 currentThemeName == "Neon HUD" 時繪製；
// 其他主題下 paintEvent 直接返回，等於隱形。父層負責跟隨大小（見 MainWindow eventFilter）。
class HudOverlay : public QWidget {
    Q_OBJECT
public:
    explicit HudOverlay(QWidget* parent = nullptr);

protected:
    void paintEvent(QPaintEvent*) override;
};
