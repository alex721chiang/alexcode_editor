#include "SymbolDialog.h"
#include "FilterEngine.h"
#include "Theme.h"
#include <QVBoxLayout>
#include <QLineEdit>
#include <QListWidget>
#include <QKeyEvent>
#include <algorithm>

SymbolDialog::SymbolDialog(QWidget* parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint) {
    setModal(true);
    resize(560, 440);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);

    input = new QLineEdit(this);
    input->setPlaceholderText(tr("跳至符號（類別/函式，支援模糊比對）"));
    list = new QListWidget(this);
    layout->addWidget(input);
    layout->addWidget(list);

    setStyleSheet(QStringLiteral("SymbolDialog { border: 1px solid %1; border-radius: 8px; }")
                      .arg(Theme::ACCENT));

    connect(input, &QLineEdit::textChanged, this, &SymbolDialog::refresh);
    connect(input, &QLineEdit::returnPressed, this, &SymbolDialog::acceptCurrent);
    connect(list, &QListWidget::itemActivated, this, [this](QListWidgetItem*) { acceptCurrent(); });
    input->installEventFilter(this);
}

void SymbolDialog::openWith(const QVector<TsSymbols::Symbol>& syms) {
    m_syms = syms;
    input->clear();
    refresh("");
    show();
    raise();
    activateWindow();
    input->setFocus();
}

void SymbolDialog::refresh(const QString& query) {
    list->clear();
    const QString q = query.trimmed();
    struct Hit { int score; int idx; };
    QVector<Hit> hits;
    for (int i = 0; i < m_syms.size(); ++i) {
        const QString& name = m_syms[i].name;
        int score = -1;
        if (q.isEmpty())                                   score = 1000 - i;          // 維持文件順序
        else if (name.startsWith(q, Qt::CaseInsensitive))  score = 400 - name.length();
        else if (name.contains(q, Qt::CaseInsensitive))    score = 300 - name.length();
        else if (FilterEngine::fuzzyContains(name, q))     score = 200 - name.length();
        if (score >= 0) hits.append({score, i});
    }
    std::stable_sort(hits.begin(), hits.end(),
                     [](const Hit& a, const Hit& b) { return a.score > b.score; });

    for (const Hit& h : hits) {
        const TsSymbols::Symbol& s = m_syms[h.idx];
        const QString indent = q.isEmpty() ? QString(s.depth * 2, QLatin1Char(' ')) : QString();
        auto* item = new QListWidgetItem(QStringLiteral("%1%2  %3")
                                             .arg(indent, s.name, s.kind));
        item->setData(Qt::UserRole, s.line);
        list->addItem(item);
    }
    if (list->count() > 0) list->setCurrentRow(0);
}

void SymbolDialog::acceptCurrent() {
    QListWidgetItem* item = list->currentItem();
    if (!item) { reject(); return; }
    emit symbolChosen(item->data(Qt::UserRole).toInt());
    accept();
}

bool SymbolDialog::eventFilter(QObject* obj, QEvent* event) {
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
