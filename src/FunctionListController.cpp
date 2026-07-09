#include "FunctionListController.h"
#include "CodeEditor.h"
#include <QMainWindow>
#include <QDockWidget>
#include <QTreeWidget>
#include <QHeaderView>
#include <QLineEdit>
#include <QVBoxLayout>
#include <QTreeWidgetItemIterator>

namespace {
constexpr int kLineRole    = Qt::UserRole;       // 符號起始行（0-based）
constexpr int kEndLineRole = Qt::UserRole + 1;   // 符號結束行（游標包含判斷用）
} // namespace

FunctionListController::FunctionListController(QMainWindow* host, QObject* parent)
    : QObject(parent) {
    m_dock = new QDockWidget(tr("FUNCTIONS — 函式清單"), host);

    auto* wrap = new QWidget(host);
    auto* lay = new QVBoxLayout(wrap);
    lay->setContentsMargins(0, 0, 0, 0);
    lay->setSpacing(2);
    m_filter = new QLineEdit(wrap);
    m_filter->setPlaceholderText(tr("過濾符號…"));
    m_filter->setClearButtonEnabled(true);
    m_tree = new QTreeWidget(wrap);
    m_tree->setHeaderHidden(true);
    m_tree->setIndentation(12);
    lay->addWidget(m_filter);
    lay->addWidget(m_tree);
    m_dock->setWidget(wrap);
    host->addDockWidget(Qt::RightDockWidgetArea, m_dock);
    m_dock->hide();

    connect(m_filter, &QLineEdit::textChanged, this,
            [this](const QString& t) { applyFilter(t); });
    connect(m_tree, &QTreeWidget::itemActivated, this, [this](QTreeWidgetItem* item, int) {
        if (!item) return;
        const QVariant v = item->data(0, kLineRole);
        if (v.isValid()) emit lineActivated(v.toInt());
    });
}

void FunctionListController::refresh(CodeEditor* editor) {
    m_tree->clear();
    if (!editor) return;

    const QVector<TsSymbols::Symbol> syms = editor->documentSymbols();
    QVector<QTreeWidgetItem*> parentStack;   // parentStack[i] = 目前深度 i 的節點（當深度 i+1 的父節點）
    for (const TsSymbols::Symbol& s : syms) {
        auto* item = new QTreeWidgetItem();
        item->setText(0, s.kind.isEmpty() ? s.name : QStringLiteral("%1  %2").arg(s.name, s.kind));
        item->setToolTip(0, s.kind);
        item->setData(0, kLineRole, s.line);
        item->setData(0, kEndLineRole, s.endLine);

        if (parentStack.size() > s.depth) parentStack.resize(s.depth);   // 深度變淺：截斷堆疊
        if (s.depth == 0 || parentStack.isEmpty())
            m_tree->addTopLevelItem(item);
        else
            parentStack.last()->addChild(item);
        parentStack.append(item);
    }
    m_tree->expandAll();
    if (!m_filter->text().isEmpty()) applyFilter(m_filter->text());   // 換分頁後沿用過濾條件
}

// 游標追蹤：選取「包含游標行、起始行最大（= 巢狀最深）」的符號。
// 只改選取與捲動、不 setFocus，使用者打字不會被搶焦點。
void FunctionListController::highlightLine(int line) {
    QTreeWidgetItem* best = nullptr;
    int bestStart = -1;
    for (QTreeWidgetItemIterator it(m_tree); *it; ++it) {
        const int start = (*it)->data(0, kLineRole).toInt();
        const int end = (*it)->data(0, kEndLineRole).toInt();
        if (start <= line && line <= end && start > bestStart) {
            best = *it;
            bestStart = start;
        }
    }
    if (best && m_tree->currentItem() != best) {
        m_tree->setCurrentItem(best);
        m_tree->scrollToItem(best);
    }
}

// 過濾：命中（名稱含過濾字串，不分大小寫）者顯示；祖先鏈保持可見以維持階層脈絡
void FunctionListController::applyFilter(const QString& text) {
    const QString needle = text.trimmed();
    if (needle.isEmpty()) {
        for (QTreeWidgetItemIterator it(m_tree); *it; ++it) (*it)->setHidden(false);
        return;
    }
    // 先全部隱藏，再把命中者與其祖先顯示出來
    for (QTreeWidgetItemIterator it(m_tree); *it; ++it) (*it)->setHidden(true);
    for (QTreeWidgetItemIterator it(m_tree); *it; ++it) {
        if (!(*it)->text(0).contains(needle, Qt::CaseInsensitive)) continue;
        for (QTreeWidgetItem* p = *it; p; p = p->parent()) p->setHidden(false);
    }
}
