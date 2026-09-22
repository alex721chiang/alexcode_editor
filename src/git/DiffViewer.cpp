#include "DiffViewer.h"
#include "Theme.h"
#include <QPlainTextEdit>
#include <QTextEdit>
#include <QSplitter>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QScrollBar>
#include <QTextBlock>
#include <QTimer>

namespace {

// 整行底色（依 diff 類型；填充列用面板色與內容區分）
QColor rowColor(DiffCalc::RowType type, bool leftSide, bool filler) {
    if (filler) return QColor(Theme::LINE_NUM_BG);
    QColor c;
    switch (type) {
    case DiffCalc::Removed:  c = QColor(Theme::LSP_ERROR);   break;   // 左：刪除（accent2 系）
    case DiffCalc::Added:    c = QColor(Theme::GIT_ADDED);   break;   // 右：新增（accent 系）
    case DiffCalc::Modified: c = QColor(Theme::GIT_MODIFIED);break;   // 兩側：修改（warn 系）
    default: return QColor();
    }
    Q_UNUSED(leftSide);
    c.setAlphaF(0.16);
    return c;
}

} // namespace

QPlainTextEdit* DiffViewer::makePane(QWidget* parent) const {
    auto* pane = new QPlainTextEdit(parent);
    pane->setReadOnly(true);
    pane->setLineWrapMode(QPlainTextEdit::NoWrap);
    pane->setFrameStyle(0);
    return pane;
}

DiffViewer::DiffViewer(const QString& titleA, const QStringList& linesA,
                       const QString& titleB, const QStringList& linesB,
                       const QList<DiffCalc::Row>& rows, QWidget* parent)
    : QWidget(parent) {
    m_changes = DiffCalc::changeCount(rows);

    auto* lay = new QVBoxLayout(this);
    lay->setContentsMargins(0, 0, 0, 0);
    lay->setSpacing(0);

    // 標題列（左右對應兩欄）
    auto* header = new QWidget(this);
    auto* hlay = new QHBoxLayout(header);
    hlay->setContentsMargins(8, 4, 8, 4);
    auto* labA = new QLabel(titleA, header);
    auto* labB = new QLabel(titleB, header);
    labA->setStyleSheet(QStringLiteral("color:%1; font-weight:600;").arg(Theme::ACCENT2));
    labB->setStyleSheet(QStringLiteral("color:%1; font-weight:600;").arg(Theme::ACCENT));
    hlay->addWidget(labA, 1);
    hlay->addWidget(labB, 1);
    lay->addWidget(header);

    auto* split = new QSplitter(Qt::Horizontal, this);
    QPlainTextEdit* left = makePane(split);
    QPlainTextEdit* right = makePane(split);
    split->addWidget(left);
    split->addWidget(right);
    split->setSizes({ 1, 1 });
    lay->addWidget(split, 1);

    // 依對齊結果組出左右文字（填充列 = 空行），同時記錄每列底色
    QStringList lt, rt;
    lt.reserve(rows.size());
    rt.reserve(rows.size());
    for (const DiffCalc::Row& r : rows) {
        lt << (r.left >= 0 ? linesA.value(r.left) : QString());
        rt << (r.right >= 0 ? linesB.value(r.right) : QString());
    }
    left->setPlainText(lt.join(QChar('\n')));
    right->setPlainText(rt.join(QChar('\n')));

    auto applyRowBackgrounds = [&rows](QPlainTextEdit* pane, bool leftSide) {
        QList<QTextEdit::ExtraSelection> sels;
        QTextBlock b = pane->document()->firstBlock();
        for (int i = 0; i < rows.size() && b.isValid(); ++i, b = b.next()) {
            const DiffCalc::Row& r = rows[i];
            const bool filler = leftSide ? (r.left < 0) : (r.right < 0);
            if (r.type == DiffCalc::Same && !filler) continue;
            const QColor c = rowColor(r.type, leftSide, filler);
            if (!c.isValid()) continue;
            QTextEdit::ExtraSelection sel;
            sel.cursor = QTextCursor(b);
            sel.format.setBackground(c);
            sel.format.setProperty(QTextFormat::FullWidthSelection, true);
            sels.append(sel);
        }
        pane->setExtraSelections(sels);
    };
    applyRowBackgrounds(left, true);
    applyRowBackgrounds(right, false);

    // 同步捲動（雙向；setValue 遇相同值為 no-op，不會遞迴）
    connect(left->verticalScrollBar(), &QScrollBar::valueChanged,
            right->verticalScrollBar(), &QScrollBar::setValue);
    connect(right->verticalScrollBar(), &QScrollBar::valueChanged,
            left->verticalScrollBar(), &QScrollBar::setValue);
    connect(left->horizontalScrollBar(), &QScrollBar::valueChanged,
            right->horizontalScrollBar(), &QScrollBar::setValue);
    connect(right->horizontalScrollBar(), &QScrollBar::valueChanged,
            left->horizontalScrollBar(), &QScrollBar::setValue);

    // 開啟時自動捲到第一處差異（上方留 3 行脈絡）。延後到事件圈執行：
    // 建構當下尚未排版、捲動範圍未就緒，直接 setValue 會被截斷。
    int firstChange = -1;
    for (int i = 0; i < rows.size(); ++i)
        if (rows[i].type != DiffCalc::Same) { firstChange = i; break; }
    if (firstChange > 0) {
        QTimer::singleShot(0, this, [left, firstChange]() {
            left->verticalScrollBar()->setValue(qMax(0, firstChange - 3));   // 同步連接會帶動右側
        });
    }
}
