#pragma once
#include <QWidget>
#include <QStringList>
#include "VtParser.h"
#include "PtySession.h"

class QTimer;

// 互動式終端機視圖：以等寬字元網格渲染 VtParser 的螢幕模型，
// 鍵盤輸入轉送給 ConPTY（PtySession）。支援捲動回看（滾輪）。
class TerminalWidget : public QWidget {
    Q_OBJECT
public:
    explicit TerminalWidget(QWidget* parent = nullptr);

    void startShell(const QString& workingDir = QString());
    void sendText(const QByteArray& data) { m_pty.writeData(data); }   // 供截圖示範等程式化輸入
    bool isRunning() const { return m_pty.isRunning(); }

protected:
    void paintEvent(QPaintEvent*) override;
    void keyPressEvent(QKeyEvent*) override;
    void resizeEvent(QResizeEvent*) override;
    void wheelEvent(QWheelEvent*) override;
    void focusInEvent(QFocusEvent*) override;
    void focusOutEvent(QFocusEvent*) override;
    void mousePressEvent(QMouseEvent*) override;
    void mouseMoveEvent(QMouseEvent*) override;
    void mouseReleaseEvent(QMouseEvent*) override;

private:
    void recomputeGrid();
    QColor ansiColor(int idx, const QColor& fallback) const;
    QStringList visibleLines() const;            // 目前顯示的各行文字
    void cellAtPos(const QPoint& pos, int* row, int* col) const;
    void copySelection() const;

    VtParser m_vt;
    PtySession m_pty;
    qreal m_cellW = 8, m_cellH = 16;
    int m_scrollOffset = 0;          // 0 = 貼齊底部
    bool m_started = false;

    // 游標閃爍
    QTimer* m_blinkTimer = nullptr;
    bool m_cursorOn = true;

    // 滑鼠選取（行/欄，顯示空間）
    bool m_selecting = false;
    bool m_hasSelection = false;
    int m_selR0 = 0, m_selC0 = 0, m_selR1 = 0, m_selC1 = 0;
};
