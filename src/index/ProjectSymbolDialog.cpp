#include "ProjectSymbolDialog.h"
#include "ProjectSymbolIndex.h"
#include "Theme.h"
#include <QVBoxLayout>
#include <QLineEdit>
#include <QListWidget>
#include <QKeyEvent>
#include <QFileInfo>

static const int kFileRole = Qt::UserRole;
static const int kLineRole = Qt::UserRole + 1;

ProjectSymbolDialog::ProjectSymbolDialog(QWidget* parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint) {
    setModal(true);
    resize(680, 460);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);

    input = new QLineEdit(this);
    input->setPlaceholderText(tr("在整個專案搜尋符號（類別/函式，支援模糊比對）"));
    list = new QListWidget(this);
    layout->addWidget(input);
    layout->addWidget(list);

    setStyleSheet(QStringLiteral("ProjectSymbolDialog { border: 1px solid %1; border-radius: 8px; }")
                      .arg(Theme::ACCENT));

    connect(input, &QLineEdit::textChanged, this, &ProjectSymbolDialog::refresh);
    connect(input, &QLineEdit::returnPressed, this, &ProjectSymbolDialog::acceptCurrent);
    connect(list, &QListWidget::itemActivated, this, [this](QListWidgetItem*) { acceptCurrent(); });
    input->installEventFilter(this);
}

void ProjectSymbolDialog::openWith(const ProjectSymbolIndex* index, const QString& initialQuery) {
    m_index = index;
    input->setText(initialQuery);          // 觸發 textChanged → refresh
    if (initialQuery.isEmpty()) refresh("");
    show();
    raise();
    activateWindow();
    input->setFocus();
}

void ProjectSymbolDialog::refresh(const QString& query) {
    list->clear();
    if (!m_index) return;
    const QVector<ProjectSymbolIndex::Entry> hits = m_index->search(query, 200);
    for (const ProjectSymbolIndex::Entry& e : hits) {
        const QString label = QStringLiteral("%1   %2   —   %3:%4")
            .arg(e.name, e.kind, QFileInfo(e.file).fileName()).arg(e.line + 1);
        auto* item = new QListWidgetItem(label);
        item->setData(kFileRole, e.file);
        item->setData(kLineRole, e.line);
        item->setToolTip(e.file);
        list->addItem(item);
    }
    if (list->count() > 0) list->setCurrentRow(0);
}

void ProjectSymbolDialog::acceptCurrent() {
    QListWidgetItem* item = list->currentItem();
    if (!item) { reject(); return; }
    const QString file = item->data(kFileRole).toString();
    const int line = item->data(kLineRole).toInt();
    accept();
    emit symbolChosen(file, line);
}

bool ProjectSymbolDialog::eventFilter(QObject* obj, QEvent* event) {
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
