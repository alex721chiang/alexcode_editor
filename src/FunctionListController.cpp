#include "FunctionListController.h"
#include "CodeEditor.h"
#include <QMainWindow>
#include <QDockWidget>
#include <QTreeWidget>
#include <QHeaderView>

FunctionListController::FunctionListController(QMainWindow* host, QObject* parent)
    : QObject(parent) {
    m_dock = new QDockWidget(tr("FUNCTIONS — 函式清單"), host);
    m_tree = new QTreeWidget(host);
    m_tree->setHeaderHidden(true);
    m_tree->setIndentation(12);
    m_dock->setWidget(m_tree);
    host->addDockWidget(Qt::RightDockWidgetArea, m_dock);
    m_dock->hide();

    connect(m_tree, &QTreeWidget::itemActivated, this, [this](QTreeWidgetItem* item, int) {
        if (!item) return;
        const QVariant v = item->data(0, Qt::UserRole);
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
        item->setData(0, Qt::UserRole, s.line);

        if (parentStack.size() > s.depth) parentStack.resize(s.depth);   // 深度變淺：截斷堆疊
        if (s.depth == 0 || parentStack.isEmpty())
            m_tree->addTopLevelItem(item);
        else
            parentStack.last()->addChild(item);
        parentStack.append(item);
    }
    m_tree->expandAll();
}
