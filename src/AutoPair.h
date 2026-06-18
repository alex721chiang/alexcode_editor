#pragma once
#include <QChar>

// 括號/引號自動配對的純決策邏輯（可單元測試，不碰編輯器狀態）。
namespace AutoPair {

// 開括號 → 閉括號；引號自身配對。非配對字元回傳空 QChar。
inline QChar closingFor(QChar open) {
    switch (open.unicode()) {
        case '(': return QChar(')');
        case '[': return QChar(']');
        case '{': return QChar('}');
        case '"': return QChar('"');
        case '\'': return QChar('\'');
        case '`': return QChar('`');
        default: return QChar();
    }
}

inline bool isQuote(QChar c) {
    return c == QLatin1Char('"') || c == QLatin1Char('\'') || c == QLatin1Char('`');
}
inline bool isOpener(QChar c) {
    return c == QLatin1Char('(') || c == QLatin1Char('[') || c == QLatin1Char('{');
}
inline bool isCloser(QChar c) {
    return c == QLatin1Char(')') || c == QLatin1Char(']') || c == QLatin1Char('}');
}
// 是否為本模組會介入的字元
inline bool isRelevant(QChar c) { return isOpener(c) || isCloser(c) || isQuote(c); }

enum class Action {
    Insert,      // 照常插入（不介入）
    AutoClose,   // 插入「開+閉」，游標停中間
    SkipOver,    // 右側已是相同閉合字元 → 游標右移、不插入
    Surround     // 有選取 → 以開/閉字元包圍選取
};

struct Decision {
    Action action;
    QChar open;
    QChar close;
};

// typed：剛輸入的字元；before/after：游標前/後一個字元（無則空 QChar）；hasSelection：是否有選取。
inline Decision decide(QChar typed, QChar before, QChar after, bool hasSelection) {
    const QChar close = closingFor(typed);

    // 有選取且輸入開括號/引號 → 包圍
    if (hasSelection && (isOpener(typed) || isQuote(typed)))
        return { Action::Surround, typed, isQuote(typed) ? typed : close };

    if (hasSelection)
        return { Action::Insert, QChar(), QChar() };

    // 輸入閉括號，且右側已是相同閉括號 → 跳過
    if (isCloser(typed) && after == typed)
        return { Action::SkipOver, QChar(), typed };

    // 引號：右側已是相同引號 → 跳過
    if (isQuote(typed) && after == typed)
        return { Action::SkipOver, QChar(), typed };

    // 引號自動配對：避免在字詞中（如英文縮寫 don't）誤配
    if (isQuote(typed)) {
        if (before.isLetterOrNumber() || after.isLetterOrNumber())
            return { Action::Insert, QChar(), QChar() };
        return { Action::AutoClose, typed, typed };
    }

    // 括號自動配對：右側若緊接字詞字元則不自動配（避免吃進後面的字）
    if (isOpener(typed)) {
        if (after.isLetterOrNumber())
            return { Action::Insert, QChar(), QChar() };
        return { Action::AutoClose, typed, close };
    }

    return { Action::Insert, QChar(), QChar() };
}

// 退格時：游標位於一對「空配對」中間（前是開、後是對應閉）→ 連同右側閉合一起刪。
inline bool shouldDeletePair(QChar before, QChar after) {
    return closingFor(before) != QChar() && closingFor(before) == after;
}

} // namespace AutoPair
