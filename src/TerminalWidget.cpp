#include "TerminalWidget.h"
#include "Theme.h"
#include <QPainter>
#include <QKeyEvent>
#include <QWheelEvent>
#include <QFontDatabase>

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

    connect(&m_pty, &PtySession::dataReceived, this, [this](const QByteArray& d) {
        m_vt.feed(d);
        update();
    });
    connect(&m_pty, &PtySession::exited, this, [this]() { update(); });
}

void TerminalWidget::startShell(const QString& workingDir) {
    if (m_started) return;
    recomputeGrid();
#ifdef Q_OS_WIN
    const QString shell = QStringLiteral("powershell.exe");
#else
    const QString shell = QStringLiteral("/bin/bash");
#endif
    m_started = m_pty.start(shell, workingDir, m_vt.cols(), m_vt.rows());
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
void TerminalWidget::focusInEvent(QFocusEvent*) { update(); }

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
        // 游標（外框）
        p.setPen(QColor(Theme::LINE_NUM_ACTIVE));
        p.drawRect(QRectF(m_vt.cursorCol() * m_cellW, m_vt.cursorRow() * m_cellH,
                          m_cellW, m_cellH));
    } else {
        // 捲動回看：以純文字呈現 scrollback + 目前畫面的最後 rows 行
        QStringList all;
        for (int i = 0; i < m_vt.scrollbackCount(); ++i) all << m_vt.scrollbackLine(i);
        for (int r = 0; r < rows; ++r) all << m_vt.lineText(r);
        const int bottom = all.size() - 1 - m_scrollOffset;
        p.setPen(defFg);
        for (int vr = 0; vr < rows; ++vr) {
            const int idx = bottom - (rows - 1 - vr);
            if (idx >= 0 && idx < all.size())
                p.drawText(QPointF(0, vr * m_cellH + asc), all[idx]);
        }
    }
}

void TerminalWidget::keyPressEvent(QKeyEvent* e) {
    QByteArray seq;
    const int k = e->key();
    const Qt::KeyboardModifiers m = e->modifiers();

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
        m_pty.writeData(seq);
        e->accept();
    } else {
        QWidget::keyPressEvent(e);
    }
}
