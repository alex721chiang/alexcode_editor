#include "FindInFilesDialog.h"
#include <QRegularExpression>

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
    searchLayout->addWidget(searchInput);
    mainLayout->addLayout(searchLayout);

    // Buttons
    auto btnLayout = new QHBoxLayout();
    searchBtn = new QPushButton("Search");
    btnLayout->addStretch();
    btnLayout->addWidget(searchBtn);
    mainLayout->addLayout(btnLayout);

    // Results
    resultsList = new QListWidget();
    mainLayout->addWidget(resultsList);

    connect(browseBtn, &QPushButton::clicked, this, &FindInFilesDialog::browseDirectory);
    connect(searchBtn, &QPushButton::clicked, this, &FindInFilesDialog::performSearch);
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

void FindInFilesDialog::performSearch() {
    resultsList->clear();
    QString dirPath = dirInput->text();
    QString filtersStr = filterInput->text();
    QString searchTerm = searchInput->text();

    if (dirPath.isEmpty() || searchTerm.isEmpty()) return;

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
                if (line.contains(searchTerm, Qt::CaseInsensitive)) {
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
