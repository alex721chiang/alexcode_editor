#include "FindInFilesDialog.h"
#include "FindInFilesEngine.h"
#include "IoPool.h"
#include <QRegularExpression>
#include <QMessageBox>
#include <QFutureWatcher>
#include <QTimer>
#include <QtConcurrent>
#include <mutex>

// 背景工作與對話框共享的狀態：對話框關閉後工作仍可安全結束（只碰這個物件）
struct FindInFilesDialog::Job {
    std::atomic<bool> cancel{false};
    std::atomic<bool> done{false};
    std::mutex mutex;
    QVector<FindInFiles::Match> pending;           // 已找到、尚未加入清單
    FindInFiles::Stats stats;                      // done 之後有效
};

FindInFilesDialog::FindInFilesDialog(QWidget *parent) : QDialog(parent) {
    setWindowTitle("Find in Files");
    resize(600, 400);
    setupUI();
}

FindInFilesDialog::~FindInFilesDialog() {
    if (m_job) m_job->cancel = true;
}

void FindInFilesDialog::setupUI() {
    auto mainLayout = new QVBoxLayout(this);

    // Directory
    auto dirLayout = new QHBoxLayout();
    dirLayout->addWidget(new QLabel("Directory:"));
    dirInput = new QLineEdit();
    dirInput->setObjectName("dirInput");
    dirLayout->addWidget(dirInput);
    browseBtn = new QPushButton("Browse...");
    dirLayout->addWidget(browseBtn);
    mainLayout->addLayout(dirLayout);

    // Filter
    auto filterLayout = new QHBoxLayout();
    filterLayout->addWidget(new QLabel("Filters:"));
    filterInput = new QLineEdit("*.cpp *.h *.txt");
    filterInput->setObjectName("filterInput");
    filterLayout->addWidget(filterInput);
    mainLayout->addLayout(filterLayout);

    // Search Term
    auto searchLayout = new QHBoxLayout();
    searchLayout->addWidget(new QLabel("Search Term:"));
    searchInput = new QLineEdit();
    searchInput->setObjectName("searchInput");
    searchInput->setPlaceholderText(tr("\u53ef\u7528 || \u5206\u9694\u591a\u500b\u95dc\u9375\u5b57 (OR)"));
    searchLayout->addWidget(searchInput);
    mainLayout->addLayout(searchLayout);

    // Replace Term（2.8）
    auto replaceLayout = new QHBoxLayout();
    replaceLayout->addWidget(new QLabel("Replace With:"));
    replaceInput = new QLineEdit();
    replaceInput->setObjectName("replaceInput");
    replaceInput->setPlaceholderText(tr("逐字取代（不分大小寫；Search Term 視為單一字串）"));
    replaceLayout->addWidget(replaceInput);
    mainLayout->addLayout(replaceLayout);

    // Buttons
    auto btnLayout = new QHBoxLayout();
    searchBtn = new QPushButton("Search");
    searchBtn->setObjectName("searchBtn");
    replaceBtn = new QPushButton("Replace All");
    replaceBtn->setObjectName("replaceBtn");
    btnLayout->addStretch();
    btnLayout->addWidget(searchBtn);
    btnLayout->addWidget(replaceBtn);
    mainLayout->addLayout(btnLayout);

    // Results
    resultsList = new QListWidget();
    resultsList->setUniformItemSizes(true);      // 大量結果時排版成本固定
    mainLayout->addWidget(resultsList);
    statusLabel = new QLabel();
    mainLayout->addWidget(statusLabel);

    drainTimer = new QTimer(this);
    drainTimer->setInterval(100);
    connect(drainTimer, &QTimer::timeout, this, &FindInFilesDialog::drainResults);

    connect(browseBtn, &QPushButton::clicked, this, &FindInFilesDialog::browseDirectory);
    connect(searchBtn, &QPushButton::clicked, this, &FindInFilesDialog::performSearch);
    connect(replaceBtn, &QPushButton::clicked, this, &FindInFilesDialog::performReplaceAll);
    connect(resultsList, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem* item) {
        emit resultDoubleClicked(item);
    });
}

void FindInFilesDialog::browseDirectory() {
    QString dir = QFileDialog::getExistingDirectory(this, "Select Directory", dirInput->text());
    if (!dir.isEmpty()) {
        dirInput->setText(dir);
    }
}

QStringList FindInFilesDialog::nameFilters() const {
    QStringList filters;
    for (const QString& f : filterInput->text().split(QRegularExpression("[, ]+"), Qt::SkipEmptyParts))
        filters << f.trimmed();
    return filters;
}

void FindInFilesDialog::setBusy(bool busy, const QString& status) {
    m_busy = busy;
    searchBtn->setText(busy ? tr("Cancel") : QStringLiteral("Search"));   // 進行中按鈕即「取消」
    replaceBtn->setEnabled(!busy);
    statusLabel->setText(status);
}

// 2.8 Replace in Files：背景統計 → 使用者確認 → 背景寫入 → 重新搜尋
void FindInFilesDialog::performReplaceAll() {
    if (m_busy) return;
    const QString searchTerm = searchInput->text();      // 逐字字串（不拆 ||）
    const QString replacement = replaceInput->text();
    const FindInFiles::Options opt{dirInput->text(), nameFilters(), searchTerm};
    if (opt.root.isEmpty() || searchTerm.isEmpty()) return;

    auto job = std::make_shared<Job>();
    m_job = job;
    setBusy(true, tr("統計中…"));
    auto* plan = new QFutureWatcher<QVector<FindInFiles::ReplaceHit>>(this);
    connect(plan, &QFutureWatcherBase::finished, this, [this, plan, job, searchTerm, replacement]() {
        const QVector<FindInFiles::ReplaceHit> hits = plan->result();
        plan->deleteLater();
        if (job != m_job) return;                        // 已被新的工作取代
        if (job->cancel) { setBusy(false, tr("已取消")); return; }
        setBusy(false, QString());
        if (hits.isEmpty()) {
            QMessageBox::information(this, "Replace in Files", tr("找不到符合的內容"));
            return;
        }
        qint64 totalCount = 0;
        for (const auto& h : hits) totalCount += h.count;
        const auto ret = QMessageBox::question(this, "Replace in Files",
            tr("將取代 %1 個檔案中的 %2 處：\n\"%3\" → \"%4\"\n\n確定執行？（無法復原）")
                .arg(hits.size()).arg(totalCount).arg(searchTerm).arg(replacement),
            QMessageBox::Yes | QMessageBox::No);
        if (ret != QMessageBox::Yes) return;

        setBusy(true, tr("取代中…"));
        auto* apply = new QFutureWatcher<FindInFiles::ReplaceResult>(this);
        connect(apply, &QFutureWatcherBase::finished, this, [this, apply, job]() {
            const FindInFiles::ReplaceResult res = apply->result();
            apply->deleteLater();
            if (job != m_job) return;
            setBusy(false, QString());
            QMessageBox::information(this, "Replace in Files",
                res.canceled ? tr("已取消：已完成 %1 個檔案、%2 處取代").arg(res.filesWritten).arg(res.replacements)
                             : tr("完成：%1 個檔案、%2 處取代").arg(res.filesWritten).arg(res.replacements));
            performSearch();                             // 重新整理結果
        });
        apply->setFuture(QtConcurrent::run(IoPool::instance(), [hits, searchTerm, replacement, job]() {
            return FindInFiles::applyReplace(hits, searchTerm, replacement, job->cancel);
        }));
    });
    plan->setFuture(QtConcurrent::run(IoPool::instance(), [opt, job]() {
        return FindInFiles::planReplace(opt, job->cancel);
    }));
}

// 搜尋於背景執行緒掃描/讀檔；結果每 100ms 分批加入清單。進行中再按一次即取消。
void FindInFilesDialog::performSearch() {
    if (m_busy) {
        if (m_job) m_job->cancel = true;
        return;
    }
    resultsList->clear();
    const FindInFiles::Options opt{dirInput->text(), nameFilters(), searchInput->text()};
    if (opt.root.isEmpty() || opt.query.isEmpty()) return;

    auto job = std::make_shared<Job>();
    m_job = job;
    setBusy(true, tr("搜尋中…"));
    (void)QtConcurrent::run(IoPool::instance(), [opt, job]() {
        const FindInFiles::Stats st = FindInFiles::search(opt, job->cancel,
            [job](const QVector<FindInFiles::Match>& batch) {
                std::lock_guard<std::mutex> lock(job->mutex);
                job->pending += batch;
            });
        std::lock_guard<std::mutex> lock(job->mutex);
        job->stats = st;
        job->done = true;
    });
    drainTimer->start();
}

void FindInFilesDialog::drainResults() {
    const std::shared_ptr<Job> job = m_job;
    if (!job) { drainTimer->stop(); return; }
    QVector<FindInFiles::Match> batch;
    bool done = false;
    FindInFiles::Stats st;
    {
        std::lock_guard<std::mutex> lock(job->mutex);
        batch.swap(job->pending);
        done = job->done;
        st = job->stats;
    }
    if (!batch.isEmpty()) {
        resultsList->setUpdatesEnabled(false);
        for (const FindInFiles::Match& m : batch) {
            auto* item = new QListWidgetItem(QString("[%1] Line %2: %3").arg(m.filePath).arg(m.line).arg(m.snippet));
            QVariantMap data;
            data["filePath"] = m.filePath;
            data["lineNum"] = m.line;
            item->setData(Qt::UserRole, data);
            resultsList->addItem(item);
        }
        resultsList->setUpdatesEnabled(true);
    }
    if (!done) {
        statusLabel->setText(tr("搜尋中… 已找到 %1 筆").arg(resultsList->count()));
        return;
    }
    drainTimer->stop();
    QString status = tr("完成：掃描 %1 個檔案，找到 %2 筆").arg(st.filesScanned).arg(st.matches);
    if (st.canceled) status = tr("已取消：掃描 %1 個檔案，找到 %2 筆").arg(st.filesScanned).arg(st.matches);
    else if (st.truncated) status += tr("（已達上限，僅顯示前 %1 筆）").arg(st.matches);
    setBusy(false, status);
}
