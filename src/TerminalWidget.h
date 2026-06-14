#pragma once
#include <QWidget>
#include "VtParser.h"
#include "PtySession.h"

// 互動式終端機視圖：以等寬字元網格渲染 VtParser 的螢幕模型，
// 鍵盤輸入轉送給 ConPTY（PtySession）。支援捲動回看（滾輪）。
class TerminalWidget : public QWidget {
    Q_OBJECT
public:
    explicit TerminalWidget(QWidget* parent = nullptr);

    void startShell(const QString& workingDir = QString());
    bool isRunning() const { return m_pty.isRunning(); }

protected:
    void paintEvent(QPaintEvent*) override;
    void keyPressEvent(QKeyEvent*) override;
    void resizeEvent(QResizeEvent*) override;
    void wheelEvent(QWheelEvent*) override;
    void focusInEvent(QFocusEvent*) override;

private:
    void recomputeGrid();
    QColor ansiColor(int idx, const QColor& fallback) const;

    VtParser m_vt;
    PtySession m_pty;
    qreal m_cellW = 8, m_cellH = 16;
    int m_scrollOffset = 0;          // 0 = 貼齊底部
    bool m_started = false;
};
