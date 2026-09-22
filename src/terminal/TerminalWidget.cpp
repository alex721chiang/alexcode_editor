#include "TerminalWidget.h"
#include "Theme.h"
#include "TerminalSelection.h"
#include <QPainter>
#include <QKeyEvent>
#include <QWheelEvent>
#include <QMouseEvent>
#include <QFontDatabase>
#include <QTimer>
#include <QApplication>
#include <QClipboard>

TerminalWidget::TerminalWidget(QWidget* parent) : QWidget(parent) {
    QFont f("Consolas");
    f.setStyleHint(QFont::Monospace);
    f.setPointSize(11);
    setFont(f);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_OpaquePaintEvent);

    const QFontMetricsF fm(font());
    m_cellW = fm.horizontalAdvance(QLatin1Char('M'));
    m_cellH = fm.height();

    m_blinkTimer = new QTimer(this);
    m_blinkTimer->setInterval(530);
    connect(m_blinkTimer, &QTimer::timeout, this, [this]() {
        m_cursorOn = !m_cursorOn;
        update();
    });

    connect(&m_pty, &PtySession::dataReceived, this, [this](const QByteArray& d) {
        m_vt.feed(d);
        m_cursorOn = true;          // 有輸出時游標亮起
        update();
    });
    connect(&m_pty, &PtySession::exited, this, [this]() { update(); });
}

QStringList TerminalWidget::visibleLines() const {
    QStringList out;
    if (m_scrollOffset == 0) {
        for (int r = 0; r < m_vt.rows(); ++r) out << m_vt.lineText(r);
    } else {
        QStringList all;
        for (int i = 0; i < m_vt.scrollbackCount(); ++i) all << m_vt.scrollbackLine(i);
        for (int r = 0; r < m_vt.rows(); ++r) all << m_vt.lineText(r);
        const int bottom = all.size() - 1 - m_scrollOffset;
        for (int vr = 0; vr < m_vt.rows(); ++vr) {
            const int idx = bottom - (m_vt.rows() - 1 - vr);
            out << ((idx >= 0 && idx < all.size()) ? all[idx] : QString());
        }
    }
    return out;
}

void TerminalWidget::cellAtPos(const QPoint& pos, int* row, int* col) const {
    *row = qBound(0, int(pos.y() / m_cellH), m_vt.rows() - 1);
    *col = qMax(0, int((pos.x() + m_cellW / 2) / m_cellW));
}

void TerminalWidget::copySelection() const {
    if (!m_hasSelection) return;
    const QString text = TermSelection::extractText(visibleLines(), m_selR0, m_selC0, m_selR1, m_selC1);
    if (!text.isEmpty()) QApplication::clipboard()->setText(text);
}

void TerminalWidget::startShell(const QString& workingDir) {
    if (m_started) return;
    recomputeGrid();
#ifdef Q_OS_WIN
    const QString shell = QStringLiteral("powershell.exe");
#else
    // 尊重使用者的預設 shell；沒設 $SHELL 時退回 bash
    const QString shell = qEnvironmentVariable("SHELL", QStringLiteral("/bin/bash"));
#endif
    m_started = m_pty.start(shell, workingDir, m_vt.cols(), m_vt.rows());
    if (!m_started) {
        // 啟動失敗時於畫面顯示訊息，而非靜默；區分「此平台不支援」與「真的啟動失敗」。
        // Windows(ConPTY) 與 Linux/macOS(forkpty) 均已實作，「不支援」分支目前僅是
        // 未來新平台的安全網。
        if (!PtySession::isPlatformSupported()) {
            m_vt.feed(QByteArray("\r\n  [此平台尚未支援內建終端機（目前僅支援 Windows）]\r\n")
                      + "  [Built-in terminal is not supported on this platform yet (Windows only for now)]\r\n");
        } else {
            m_vt.feed(QByteArray("\r\n  [無法啟動終端機：找不到或無法執行 ")
                      + shell.toUtf8() + "]\r\n"
                      + "  [Failed to start terminal shell: " + shell.toUtf8() + "]\r\n");
        }
        update();
    }
    setFocus();
}

void TerminalWidget::recomputeGrid() {
    const QFontMetricsF fm(font());
    m_cellW = qMax<qreal>(fm.horizontalAdvance(QLatin1Char('M')), 1.0);
    m_cellH = qMax<qreal>(fm.height(), 1.0);
    const int cols = qMax(1, int(width() / m_cellW));
    const int rows = qMax(1, int(height() / m_cellH));
    if (cols != m_vt.cols() || rows != m_vt.rows()) {
        m_vt.resize(rows, cols);
        m_pty.resize(cols, rows);
    }
}

// xterm 16 色標準調色盤
QColor TerminalWidget::ansiColor(int idx, const QColor& fallback) const {
    static const QColor pal[16] = {
        QColor("#000000"), QColor("#cd0000"), QColor("#00cd00"), QColor("#cdcd00"),
        QColor("#2222ee"), QColor("#cd00cd"), QColor("#00cdcd"), QColor("#e5e5e5"),
        QColor("#7f7f7f"), QColor("#ff5555"), QColor("#55ff55"), QColor("#ffff55"),
        QColor("#5c5cff"), QColor("#ff55ff"), QColor("#55ffff"), QColor("#ffffff")
    };
    return (idx >= 0 && idx < 16) ? pal[idx] : fallback;
}

void TerminalWidget::resizeEvent(QResizeEvent*) { recomputeGrid(); }
void TerminalWidget::focusInEvent(QFocusEvent*) { m_cursorOn = true; m_blinkTimer->start(); update(); }
void TerminalWidget::focusOutEvent(QFocusEvent*) { m_blinkTimer->stop(); m_cursorOn = false; update(); }

void TerminalWidget::mousePressEvent(QMouseEvent* e) {
    if (e->button() == Qt::LeftButton) {
        m_selecting = true;
        cellAtPos(e->pos(), &m_selR0, &m_selC0);
        m_selR1 = m_selR0; m_selC1 = m_selC0;
        m_hasSelection = false;
        update();
    }
    setFocus();
}

void TerminalWidget::mouseMoveEvent(QMouseEvent* e) {
    if (m_selecting) {
        cellAtPos(e->pos(), &m_selR1, &m_selC1);
        m_hasSelection = (m_selR0 != m_selR1 || m_selC0 != m_selC1);
        update();
    }
}

void TerminalWidget::mouseReleaseEvent(QMouseEvent* e) {
    if (e->button() == Qt::LeftButton) m_selecting = false;
}

void TerminalWidget::wheelEvent(QWheelEvent* e) {
    const int lines = e->angleDelta().y() / 40;     // 每刻度約 3 行
    m_scrollOffset = qBound(0, m_scrollOffset + lines, m_vt.scrollbackCount());
    update();
    e->accept();
}

void TerminalWidget::paintEvent(QPaintEvent*) {
    QPainter p(this);
    const QColor defBg(Theme::EDITOR_BG), defFg(Theme::EDITOR_FG);
    p.fillRect(rect(), defBg);
    p.setFont(font());
    const QFontMetricsF fm(font());
    const qreal asc = fm.ascent();
    const int rows = m_vt.rows(), cols = m_vt.cols();

    if (m_scrollOffset == 0) {
        for (int r = 0; r < rows; ++r) {
            for (int c = 0; c < cols; ++c) {
                const VtParser::Cell& cell = m_vt.cellAt(r, c);
                const qreal x = c * m_cellW, y = r * m_cellH;
                if (cell.bg >= 0)
                    p.fillRect(QRectF(x, y, m_cellW, m_cellH), ansiColor(cell.bg, defBg));
                if (cell.ch != QLatin1Char(' ')) {
                    QFont cf = font(); cf.setBold(cell.bold); p.setFont(cf);
                    p.setPen(ansiColor(cell.fg, defFg));
                    p.drawText(QPointF(x, y + asc), QString(cell.ch));
                }
            }
        }
        // 游標（外框；閃爍、僅在焦點時亮）
        if (m_cursorOn && hasFocus()) {
            p.setPen(QColor(Theme::LINE_NUM_ACTIVE));
            p.drawRect(QRectF(m_vt.cursorCol() * m_cellW, m_vt.cursorRow() * m_cellH,
                              m_cellW, m_cellH));
        }
    } else {
        // 捲動回看：以純文字呈現
        const QStringList vis = visibleLines();
        p.setPen(defFg);
        for (int vr = 0; vr < vis.size(); ++vr)
            p.drawText(QPointF(0, vr * m_cellH + asc), vis[vr]);
    }

    // 選取範圍標示（半透明）
    if (m_hasSelection) {
        int r0 = m_selR0, c0 = m_selC0, r1 = m_selR1, c1 = m_selC1;
        TermSelection::normalize(r0, c0, r1, c1);
        QColor sel(Theme::SEARCH_MATCH_BG); sel.setAlpha(120);
        for (int r = r0; r <= r1; ++r) {
            const int from = (r == r0) ? c0 : 0;
            const int to   = (r == r1) ? c1 : cols;
            if (to > from)
                p.fillRect(QRectF(from * m_cellW, r * m_cellH, (to - from) * m_cellW, m_cellH), sel);
        }
    }
}

void TerminalWidget::keyPressEvent(QKeyEvent* e) {
    QByteArray seq;
    const int k = e->key();
    const Qt::KeyboardModifiers m = e->modifiers();

    // 複製 / 貼上（Ctrl+Shift+C / Ctrl+Shift+V）優先於 Ctrl+字母（避免被當成 0x03 中斷）
    if ((m & Qt::ControlModifier) && (m & Qt::ShiftModifier)) {
        if (k == Qt::Key_C) { copySelection(); e->accept(); return; }
        if (k == Qt::Key_V) {
            const QString clip = QApplication::clipboard()->text();
            if (!clip.isEmpty()) { m_scrollOffset = 0; m_pty.writeData(clip.toUtf8()); }
            e->accept();
            return;
        }
    }

    if ((m & Qt::ControlModifier) && k >= Qt::Key_A && k <= Qt::Key_Z) {
        seq.append(static_cast<char>(k - Qt::Key_A + 1));     // Ctrl+A..Z → 0x01..0x1a
    } else {
        switch (k) {
        case Qt::Key_Return: case Qt::Key_Enter: seq = "\r"; break;
        case Qt::Key_Backspace: seq = "\x7f"; break;
        case Qt::Key_Tab:    seq = "\t"; break;
        case Qt::Key_Escape: seq = "\x1b"; break;
        case Qt::Key_Up:     seq = "\x1b[A"; break;
        case Qt::Key_Down:   seq = "\x1b[B"; break;
        case Qt::Key_Right:  seq = "\x1b[C"; break;
        case Qt::Key_Left:   seq = "\x1b[D"; break;
        case Qt::Key_Home:   seq = "\x1b[H"; break;
        case Qt::Key_End:    seq = "\x1b[F"; break;
        case Qt::Key_Delete: seq = "\x1b[3~"; break;
        default: if (!e->text().isEmpty()) seq = e->text().toUtf8(); break;
        }
    }
    if (!seq.isEmpty()) {
        m_scrollOffset = 0;          // 鍵入時跳回底部
        if (m_hasSelection) { m_hasSelection = false; }   // 鍵入清除選取
        m_pty.writeData(seq);
        e->accept();
    } else {
        QWidget::keyPressEvent(e);
    }
}
