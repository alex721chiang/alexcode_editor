#pragma once
#include <QString>
#include <QVector>
#include <QByteArray>

// 純邏輯 VT100/ANSI 解析器 + 螢幕模型（不依賴 GUI，方便單元測試）。
// 支援：可列印字元、CR/LF/BS/TAB、CSI 游標移動(CUU/CUD/CUF/CUB/CUP)、
// 清除(ED/EL)、SGR(重置/粗體/前景/背景/亮色)。容忍跨 feed 切割的逸出序列；
// 無法辨識的序列安全略過。捲離頂端的行進入 scrollback。
class VtParser {
public:
    struct Cell {
        QChar ch = QLatin1Char(' ');
        qint8 fg = -1;          // -1 = 預設；0..15 = ANSI 顏色索引
        qint8 bg = -1;
        bool bold = false;
    };

    VtParser(int rows = 24, int cols = 80) { resize(rows, cols); }

    void resize(int rows, int cols);
    void feed(const QByteArray& bytes);
    void reset();

    int rows() const { return m_rows; }
    int cols() const { return m_cols; }
    int cursorRow() const { return m_cy; }
    int cursorCol() const { return m_cx; }

    const Cell& cellAt(int row, int col) const;
    QString lineText(int row) const;                 // 該可視行的文字（含尾端空白裁去）

    int scrollbackCount() const { return m_scrollback.size(); }
    QString scrollbackLine(int i) const;             // 0 = 最舊

private:
    void putChar(QChar ch);
    void feedTextByte(unsigned char c);              // UTF-8 累積解碼（跨 feed 邊界）
    void emitUtf8(const QByteArray& bytes);          // 解碼一個完整序列並輸出
    void flushUtf8();                                // 控制字元/逸出打斷未完成序列時清掉
    void newline();
    void scrollUp();
    void execCsi(QChar final);
    void applySgr(const QVector<int>& params);
    void clearCell(Cell& c) const;
    int  paramOr(int idx, int def) const;

    int m_rows = 24, m_cols = 80;
    int m_cx = 0, m_cy = 0;
    QVector<QVector<Cell>> m_grid;                    // [row][col]
    QVector<QString> m_scrollback;
    Cell m_cur;                                       // 目前 SGR 狀態

    // 解析狀態機（跨 feed 保留）
    enum class State { Ground, Esc, Csi, Osc } m_state = State::Ground;
    QByteArray m_csiBuf;                              // CSI 參數/中間位元組累積
    QByteArray m_utf8Buf;                             // 未完成的 UTF-8 序列位元組
    int m_utf8Need = 0;                              // 還缺幾個延續位元組
    static const int kMaxScrollback = 5000;
};
