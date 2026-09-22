#include "QuickOpenDialog.h"
#include "IoPool.h"
#include "FilterEngine.h"
#include "Theme.h"
#include <QVBoxLayout>
#include <QDirIterator>
#include <QKeyEvent>
#include <QtConcurrent/QtConcurrent>
#include <QFileInfo>

QuickOpenDialog::QuickOpenDialog(QWidget* parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint)
{
    setModal(true);
    resize(620, 420);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);

    input = new QLineEdit(this);
    input->setPlaceholderText(tr("輸入檔名（支援模糊比對，例如 mwcpp → MainWindow.cpp）"));
    list = new QListWidget(this);
    layout->addWidget(input);
    layout->addWidget(list);

    setStyleSheet(QStringLiteral("QuickOpenDialog { border: 1px solid %1; border-radius: 8px; }")
                      .arg(Theme::ACCENT));

    indexWatcher = new QFutureWatcher<QStringList>(this);
    connect(indexWatcher, &QFutureWatcher<QStringList>::finished, this, [this]() {
        fileIndex = indexWatcher->result();
        refreshResults(input->text());
    });

    connect(input, &QLineEdit::textChanged, this, &QuickOpenDialog::refreshResults);
    connect(input, &QLineEdit::returnPressed, this, &QuickOpenDialog::acceptCurrent);
    connect(list, &QListWidget::itemActivated, this, [this](QListWidgetItem*) { acceptCurrent(); });
    input->installEventFilter(this);
}

void QuickOpenDialog::setRootFolder(const QString& folder) {
    rootFolder = folder;
    fileIndex.clear();
    if (folder.isEmpty()) return;
    indexWatcher->setFuture(QtConcurrent::run(IoPool::instance(), &QuickOpenDialog::buildIndex, folder));
}

QStringList QuickOpenDialog::buildIndex(const QString& folder) {
    static const QStringList skipDirs = {".git", "build", "node_modules", ".vs",
                                         "__pycache__", ".idea", "dist", "out"};
    QStringList files;
    QDirIterator it(folder, QDir::Files, QDirIterator::Subdirectories);
    const int prefixLen = folder.length() + 1;
    while (it.hasNext() && files.size() < 50000) {
        const QString path = it.next();
        const QString rel = path.mid(prefixLen);
        bool skip = false;
        for (const auto& d : skipDirs) {
            if (rel.startsWith(d + "/") || rel.contains("/" + d + "/")) { skip = true; break; }
        }
        if (!skip) files << rel;
    }
    return files;
}

void QuickOpenDialog::refreshResults(const QString& query) {
    list->clear();
    const QString q = query.trimmed();
    struct Hit { int score; QString rel; };
    QVector<Hit> hits;

    for (const QString& rel : fileIndex) {
        const QString name = QFileInfo(rel).fileName();
        int score = -1;
        if (q.isEmpty())                                    score = 0;
        else if (name.contains(q, Qt::CaseInsensitive))     score = 300 - name.length();   // 檔名子字串最優先
        else if (FilterEngine::fuzzyContains(name, q))      score = 200 - name.length();   // 檔名模糊
        else if (rel.contains(q, Qt::CaseInsensitive))      score = 100 - rel.length();    // 路徑子字串
        else if (FilterEngine::fuzzyContains(rel, q))       score = 50  - rel.length();    // 路徑模糊
        if (score >= 0) hits.append({score, rel});
        if (hits.size() > 5000) break;
    }
    std::stable_sort(hits.begin(), hits.end(),
                     [](const Hit& a, const Hit& b) { return a.score > b.score; });

    const int n = qMin(hits.size(), 50);
    for (int i = 0; i < n; ++i) {
        auto* item = new QListWidgetItem(hits[i].rel);
        item->setData(Qt::UserRole, rootFolder + "/" + hits[i].rel);
        list->addItem(item);
    }
    if (list->count() > 0) list->setCurrentRow(0);
}

void QuickOpenDialog::acceptCurrent() {
    QListWidgetItem* item = list->currentItem();
    if (!item) return;
    emit fileChosen(item->data(Qt::UserRole).toString());
    accept();
}

void QuickOpenDialog::open() {
    input->clear();
    refreshResults("");
    show();
    raise();
    activateWindow();
    input->setFocus();
}

// 在輸入框按上下鍵操作清單
bool QuickOpenDialog::eventFilter(QObject* obj, QEvent* event) {
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
