#include "VtParser.h"

void VtParser::clearCell(Cell& c) const {
    c.ch = QLatin1Char(' ');
    c.fg = -1; c.bg = -1; c.bold = false;
}

void VtParser::resize(int rows, int cols) {
    m_rows = qMax(1, rows);
    m_cols = qMax(1, cols);
    m_grid.resize(m_rows);
    for (auto& row : m_grid) {
        row.resize(m_cols);
        for (auto& cell : row) clearCell(cell);
    }
    m_cx = qMin(m_cx, m_cols - 1);
    m_cy = qMin(m_cy, m_rows - 1);
}

void VtParser::reset() {
    m_cx = m_cy = 0;
    m_cur = Cell();
    m_state = State::Ground;
    m_csiBuf.clear();
    m_utf8Buf.clear();
    m_utf8Need = 0;
    m_scrollback.clear();
    for (auto& row : m_grid)
        for (auto& cell : row) clearCell(cell);
}

const VtParser::Cell& VtParser::cellAt(int row, int col) const {
    static const Cell empty;
    if (row < 0 || row >= m_rows || col < 0 || col >= m_cols) return empty;
    return m_grid[row][col];
}

QString VtParser::lineText(int row) const {
    if (row < 0 || row >= m_rows) return QString();
    QString s;
    for (const Cell& c : m_grid[row]) s += c.ch;
    while (s.endsWith(QLatin1Char(' '))) s.chop(1);
    return s;
}

QString VtParser::scrollbackLine(int i) const {
    return (i >= 0 && i < m_scrollback.size()) ? m_scrollback[i] : QString();
}

void VtParser::scrollUp() {
    QString top;
    for (const Cell& c : m_grid[0]) top += c.ch;
    while (top.endsWith(QLatin1Char(' '))) top.chop(1);
    m_scrollback.append(top);
    if (m_scrollback.size() > kMaxScrollback)
        m_scrollback.remove(0, m_scrollback.size() - kMaxScrollback);
    for (int r = 0; r < m_rows - 1; ++r) m_grid[r] = m_grid[r + 1];
    QVector<Cell>& last = m_grid[m_rows - 1];
    for (auto& cell : last) clearCell(cell);
}

void VtParser::newline() {
    if (m_cy >= m_rows - 1) scrollUp();
    else ++m_cy;
}

void VtParser::putChar(QChar ch) {
    if (m_cx >= m_cols) {            // 自動換行
        m_cx = 0;
        newline();
    }
    Cell& cell = m_grid[m_cy][m_cx];
    cell.ch = ch;
    cell.fg = m_cur.fg; cell.bg = m_cur.bg; cell.bold = m_cur.bold;
    ++m_cx;
}

void VtParser::emitUtf8(const QByteArray& bytes) {
    const QString s = QString::fromUtf8(bytes);          // 處理代理對（emoji 等）
    for (const QChar ch : s) putChar(ch);
}

void VtParser::flushUtf8() {
    if (m_utf8Need > 0) {                                 // 序列被打斷 → 輸出替換字元
        putChar(QChar(0xFFFD));
        m_utf8Buf.clear();
        m_utf8Need = 0;
    }
}

// 逐位元組累積 UTF-8；可列印 ASCII 直接輸出，多位元組湊齊整個序列才解碼。
void VtParser::feedTextByte(unsigned char c) {
    if (m_utf8Need > 0) {
        if ((c & 0xC0) == 0x80) {                        // 合法延續位元組
            m_utf8Buf.append(static_cast<char>(c));
            if (--m_utf8Need == 0) { emitUtf8(m_utf8Buf); m_utf8Buf.clear(); }
        } else {                                         // 不合法 → 先吐替換字元，再重新處理本位元組
            putChar(QChar(0xFFFD));
            m_utf8Buf.clear();
            m_utf8Need = 0;
            feedTextByte(c);
        }
        return;
    }
    if (c < 0x80) { putChar(QChar(c)); return; }          // ASCII
    int len = 0;                                          // 前導位元組 → 序列長度
    if      ((c & 0xE0) == 0xC0) len = 2;
    else if ((c & 0xF0) == 0xE0) len = 3;
    else if ((c & 0xF8) == 0xF0) len = 4;
    if (len == 0) { putChar(QChar(0xFFFD)); return; }     // 不合法前導
    m_utf8Buf.clear();
    m_utf8Buf.append(static_cast<char>(c));
    m_utf8Need = len - 1;
}

int VtParser::paramOr(int idx, int def) const {
    const QList<QByteArray> parts = m_csiBuf.split(';');
    if (idx >= parts.size()) return def;
    bool ok = false;
    const int v = parts[idx].toInt(&ok);
    return ok ? v : def;
}

void VtParser::feed(const QByteArray& bytes) {
    for (char b : bytes) {
        const unsigned char c = static_cast<unsigned char>(b);
        switch (m_state) {
        case State::Ground:
            if (c == 0x1b) { flushUtf8(); m_state = State::Esc; }
            else if (c == '\r') { flushUtf8(); m_cx = 0; }
            else if (c == '\n') { flushUtf8(); newline(); }
            else if (c == '\b') { flushUtf8(); if (m_cx > 0) --m_cx; }
            else if (c == '\t') { flushUtf8(); m_cx = qMin(((m_cx / 8) + 1) * 8, m_cols - 1); }
            else if (c == 0x07) { flushUtf8(); }         // BEL：忽略
            else if (c >= 0x20) { feedTextByte(c); }     // UTF-8 累積解碼（含中文/emoji）
            break;
        case State::Esc:
            if (c == '[') { m_state = State::Csi; m_csiBuf.clear(); }
            else if (c == ']') { m_state = State::Osc; }  // OSC：吞到 BEL 或 ST
            else { m_state = State::Ground; }            // 其他 ESC x：略過
            break;
        case State::Csi:
            if (c >= 0x40 && c <= 0x7e) {                 // 終結位元組
                execCsi(QChar(b));
                m_state = State::Ground;
                m_csiBuf.clear();
            } else {
                m_csiBuf.append(b);                       // 參數/中間位元組
            }
            break;
        case State::Osc:
            if (c == 0x07 || c == 0x1b) m_state = State::Ground;   // BEL 或 ST 結束
            break;
        }
    }
}

void VtParser::execCsi(QChar finalCh) {
    // 私有模式 ?... （游標顯示等）：略過
    if (!m_csiBuf.isEmpty() && m_csiBuf[0] == '?') return;

    const char f = finalCh.toLatin1();
    switch (f) {
    case 'A': m_cy = qMax(0, m_cy - paramOr(0, 1)); break;                 // 上
    case 'B': m_cy = qMin(m_rows - 1, m_cy + paramOr(0, 1)); break;        // 下
    case 'C': m_cx = qMin(m_cols - 1, m_cx + paramOr(0, 1)); break;        // 右
    case 'D': m_cx = qMax(0, m_cx - paramOr(0, 1)); break;                 // 左
    case 'G': m_cx = qBound(0, paramOr(0, 1) - 1, m_cols - 1); break;      // 行內絕對欄
    case 'd': m_cy = qBound(0, paramOr(0, 1) - 1, m_rows - 1); break;      // 絕對列
    case 'H': case 'f':                                                    // 游標定位（1-based）
        m_cy = qBound(0, paramOr(0, 1) - 1, m_rows - 1);
        m_cx = qBound(0, paramOr(1, 1) - 1, m_cols - 1);
        break;
    case 'J': {                                                           // 清除螢幕
        const int mode = paramOr(0, 0);
        auto clearRange = [&](int r0, int c0, int r1, int c1) {
            for (int r = r0; r <= r1; ++r)
                for (int cc = (r == r0 ? c0 : 0); cc <= (r == r1 ? c1 : m_cols - 1); ++cc)
                    clearCell(m_grid[r][cc]);
        };
        if (mode == 0) clearRange(m_cy, m_cx, m_rows - 1, m_cols - 1);
        else if (mode == 1) clearRange(0, 0, m_cy, m_cx);
        else { clearRange(0, 0, m_rows - 1, m_cols - 1); }
        break;
    }
    case 'K': {                                                           // 清除行
        const int mode = paramOr(0, 0);
        int from = 0, to = m_cols - 1;
        if (mode == 0) from = m_cx;
        else if (mode == 1) to = m_cx;
        for (int cc = from; cc <= to; ++cc) clearCell(m_grid[m_cy][cc]);
        break;
    }
    case 'm': {                                                           // SGR
        QVector<int> params;
        const QList<QByteArray> parts = m_csiBuf.split(';');
        for (const QByteArray& p : parts) params.append(p.isEmpty() ? 0 : p.toInt());
        if (params.isEmpty()) params.append(0);
        applySgr(params);
        break;
    }
    default: break;                                                       // 其他：略過
    }
}

void VtParser::applySgr(const QVector<int>& params) {
    for (int p : params) {
        if (p == 0) { m_cur = Cell(); }                  // 重置
        else if (p == 1) m_cur.bold = true;
        else if (p == 22) m_cur.bold = false;
        else if (p >= 30 && p <= 37) m_cur.fg = static_cast<qint8>(p - 30);
        else if (p == 39) m_cur.fg = -1;
        else if (p >= 40 && p <= 47) m_cur.bg = static_cast<qint8>(p - 40);
        else if (p == 49) m_cur.bg = -1;
        else if (p >= 90 && p <= 97) m_cur.fg = static_cast<qint8>(8 + (p - 90));
        else if (p >= 100 && p <= 107) m_cur.bg = static_cast<qint8>(8 + (p - 100));
    }
}
