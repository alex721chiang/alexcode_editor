#include "FindInFilesDialog.h"
#include <QRegularExpression>
#include <QMessageBox>
#include "FilterEngine.h"

FindInFilesDialog::FindInFilesDialog(QWidget *parent) : QDialog(parent) {
    setWindowTitle("Find in Files");
    resize(600, 400);
    setupUI();
}

void FindInFilesDialog::setupUI() {
    auto mainLayout = new QVBoxLayout(this);

    // Directory
    auto dirLayout = new QHBoxLayout();
    dirLayout->addWidget(new QLabel("Directory:"));
    dirInput = new QLineEdit();
    dirLayout->addWidget(dirInput);
    browseBtn = new QPushButton("Browse...");
    dirLayout->addWidget(browseBtn);
    mainLayout->addLayout(dirLayout);

    // Filter
    auto filterLayout = new QHBoxLayout();
    filterLayout->addWidget(new QLabel("Filters:"));
    filterInput = new QLineEdit("*.cpp *.h *.txt");
    filterLayout->addWidget(filterInput);
    mainLayout->addLayout(filterLayout);

    // Search Term
    auto searchLayout = new QHBoxLayout();
    searchLayout->addWidget(new QLabel("Search Term:"));
    searchInput = new QLineEdit();
    searchInput->setPlaceholderText(tr("\u53ef\u7528 || \u5206\u9694\u591a\u500b\u95dc\u9375\u5b57 (OR)"));
    searchLayout->addWidget(searchInput);
    mainLayout->addLayout(searchLayout);

    // Replace Term（2.8）
    auto replaceLayout = new QHBoxLayout();
    replaceLayout->addWidget(new QLabel("Replace With:"));
    replaceInput = new QLineEdit();
    replaceInput->setPlaceholderText(tr("逐字取代（不分大小寫；Search Term 視為單一字串）"));
    replaceLayout->addWidget(replaceInput);
    mainLayout->addLayout(replaceLayout);

    // Buttons
    auto btnLayout = new QHBoxLayout();
    searchBtn = new QPushButton("Search");
    replaceBtn = new QPushButton("Replace All");
    btnLayout->addStretch();
    btnLayout->addWidget(searchBtn);
    btnLayout->addWidget(replaceBtn);
    mainLayout->addLayout(btnLayout);

    // Results
    resultsList = new QListWidget();
    mainLayout->addWidget(resultsList);

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

void FindInFilesDialog::performReplaceAll() {
    const QString dirPath = dirInput->text();
    const QString searchTerm = searchInput->text();      // 逐字字串（不拆 ||）
    const QString replacement = replaceInput->text();
    if (dirPath.isEmpty() || searchTerm.isEmpty()) return;

    QStringList nameFilters;
    for (const QString& f : filterInput->text().split(QRegularExpression("[, ]+"), Qt::SkipEmptyParts))
        nameFilters << f.trimmed();

    // 先統計，確認後才寫檔
    const QRegularExpression re(QRegularExpression::escape(searchTerm),
                                QRegularExpression::CaseInsensitiveOption);
    struct Hit { QString path; QString newText; int count; };
    QList<Hit> hits;
    qint64 totalCount = 0;
    QDirIterator it(dirPath, nameFilters, QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        const QString filePath = it.next();
        QFile file(filePath);
        if (!file.open(QIODevice::ReadOnly)) continue;
        QString text = QString::fromUtf8(file.readAll());
        file.close();
        int count = 0;
        auto mit = re.globalMatch(text);
        while (mit.hasNext()) { mit.next(); ++count; }
        if (count == 0) continue;
        text.replace(re, replacement);
        hits.append({filePath, text, count});
        totalCount += count;
    }
    if (hits.isEmpty()) {
        QMessageBox::information(this, "Replace in Files", tr("找不到符合的內容"));
        return;
    }
    const auto ret = QMessageBox::question(this, "Replace in Files",
        tr("將取代 %1 個檔案中的 %2 處：\n\"%3\" → \"%4\"\n\n確定執行？（無法復原）")
            .arg(hits.size()).arg(totalCount).arg(searchTerm).arg(replacement),
        QMessageBox::Yes | QMessageBox::No);
    if (ret != QMessageBox::Yes) return;

    int written = 0;
    for (const Hit& h : hits) {
        QFile file(h.path);
        if (file.open(QIODevice::WriteOnly)) {
            file.write(h.newText.toUtf8());
            ++written;
        }
    }
    QMessageBox::information(this, "Replace in Files",
        tr("完成：%1 個檔案、%2 處取代").arg(written).arg(totalCount));
    performSearch();                                     // 重新整理結果
}

void FindInFilesDialog::performSearch() {
    resultsList->clear();
    QString dirPath = dirInput->text();
    QString filtersStr = filterInput->text();
    QString searchTerm = searchInput->text();

    if (dirPath.isEmpty() || searchTerm.isEmpty()) return;

    // 支援 || 多關鍵字 (OR 邏輯)
    FilterEngine engine;
    engine.setKeywords(FilterEngine::parseQuery(searchTerm));
    engine.setLogic(FilterLogic::OR);

    QStringList nameFilters;
    // Split by comma or space
    for (const QString& f : filtersStr.split(QRegularExpression("[, ]+"), Qt::SkipEmptyParts)) {
        nameFilters << f.trimmed();
    }

    QDirIterator it(dirPath, nameFilters, QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        QString filePath = it.next();
        QFile file(filePath);
        if (file.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QTextStream in(&file);
            int lineNum = 1;
            while (!in.atEnd()) {
                QString line = in.readLine();
                if (engine.matchLine(line)) {
                    QString snippet = line.trimmed();
                    if (snippet.length() > 100) {
                        snippet = snippet.left(97) + "...";
                    }
                    QString displayText = QString("[%1] Line %2: %3").arg(filePath).arg(lineNum).arg(snippet);
                    auto item = new QListWidgetItem(displayText);
                    
                    QVariantMap data;
                    data["filePath"] = filePath;
                    data["lineNum"] = lineNum;
                    item->setData(Qt::UserRole, data);
                    
                    resultsList->addItem(item);
                }
                lineNum++;
            }
        }
    }
}
