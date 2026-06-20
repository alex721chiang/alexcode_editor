#include "CommandPalette.h"
#include "CommandMatch.h"
#include "Theme.h"
#include <QVBoxLayout>
#include <QLineEdit>
#include <QListWidget>
#include <QKeyEvent>
#include <QAction>
#include <algorithm>

CommandPalette::CommandPalette(QWidget* parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint) {
    setModal(true);
    resize(620, 460);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);

    input = new QLineEdit(this);
    input->setPlaceholderText(tr("執行指令（模糊搜尋，例如 gts → Go to Symbol）"));
    list = new QListWidget(this);
    layout->addWidget(input);
    layout->addWidget(list);

    setStyleSheet(QStringLiteral("CommandPalette { border: 1px solid %1; border-radius: 8px; }")
                      .arg(Theme::ACCENT));

    connect(input, &QLineEdit::textChanged, this, &CommandPalette::refresh);
    connect(input, &QLineEdit::returnPressed, this, &CommandPalette::acceptCurrent);
    connect(list, &QListWidget::itemActivated, this, [this](QListWidgetItem*) { acceptCurrent(); });
    input->installEventFilter(this);
}

void CommandPalette::openWith(const QVector<Command>& cmds, const QString& initialQuery) {
    m_cmds = cmds;
    input->setText(initialQuery);      // 觸發 textChanged → refresh
    if (initialQuery.isEmpty()) refresh("");
    show();
    raise();
    activateWindow();
    input->setFocus();
}

void CommandPalette::refresh(const QString& query) {
    list->clear();
    struct Hit { int score; int idx; };
    QVector<Hit> hits;
    for (int i = 0; i < m_cmds.size(); ++i) {
        const int s = CommandMatch::score(m_cmds[i].name, query);
        if (s >= 0) hits.append({s, i});
    }
    std::stable_sort(hits.begin(), hits.end(),
                     [](const Hit& a, const Hit& b) { return a.score > b.score; });

    for (const Hit& h : hits) {
        const Command& c = m_cmds[h.idx];
        QString label = c.name;
        if (!c.shortcut.isEmpty()) label += QStringLiteral("    [%1]").arg(c.shortcut);
        auto* item = new QListWidgetItem(label);
        item->setData(Qt::UserRole, h.idx);
        list->addItem(item);
    }
    if (list->count() > 0) list->setCurrentRow(0);
}

void CommandPalette::acceptCurrent() {
    QListWidgetItem* item = list->currentItem();
    if (!item) { reject(); return; }
    const int idx = item->data(Qt::UserRole).toInt();
    accept();
    if (idx >= 0 && idx < m_cmds.size() && m_cmds[idx].action)
        m_cmds[idx].action->trigger();
}

bool CommandPalette::eventFilter(QObject* obj, QEvent* event) {
    if (obj == input && event->type() == QEvent::KeyPress) {
        auto* ke = static_cast<QKeyEvent*>(event);
        if (ke->key() == Qt::Key_Down || ke->key() == Qt::Key_Up) {
            int row = list->currentRow() + (ke->key() == Qt::Key_Down ? 1 : -1);
            row = qBound(0, row, list->count() - 1);
            list->setCurrentRow(row);
            return true;
        }
        if (ke->key() == Qt::Key_Escape) { reject(); return true; }
    }
    return QDialog::eventFilter(obj, event);
}
