#include "SettingsDialog.h"
#include <QTabWidget>
#include <QTableWidget>
#include <QHeaderView>
#include <QPushButton>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QDialogButtonBox>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>

SettingsDialog::SettingsDialog(const QString& lspPath, const QString& snippetPath,
                               const QString& keymapPath,
                               const QList<QPair<QString, QString>>& actions, QWidget* parent)
    : QDialog(parent), m_lspPath(lspPath), m_snippetPath(snippetPath), m_keymapPath(keymapPath) {
    setWindowTitle(tr("設定中心"));
    resize(720, 480);
    auto* outer = new QVBoxLayout(this);
    auto* tabs = new QTabWidget(this);
    outer->addWidget(tabs);

    // 每個分頁：表格 + 新增/刪除（快捷鍵分頁不可增刪，只改）
    auto makePage = [this](QTableWidget* table, bool editableRows) {
        auto* page = new QWidget(this);
        auto* lay = new QVBoxLayout(page);
        lay->addWidget(table);
        if (editableRows) {
            auto* row = new QHBoxLayout();
            auto* add = new QPushButton(tr("新增"), page);
            auto* del = new QPushButton(tr("刪除選取列"), page);
            row->addWidget(add); row->addWidget(del); row->addStretch();
            lay->addLayout(row);
            connect(add, &QPushButton::clicked, this, [table]() {
                table->insertRow(table->rowCount());
            });
            connect(del, &QPushButton::clicked, this, [table]() {
                const auto sel = table->selectionModel()->selectedRows();
                QList<int> rows;
                for (const auto& idx : sel) rows << idx.row();
                std::sort(rows.begin(), rows.end(), std::greater<int>());
                for (int r : rows) table->removeRow(r);
            });
        }
        return page;
    };

    m_lspTable  = makeTable({tr("語言 ID"), tr("副檔名（空白分隔）"), tr("執行檔"), tr("參數（空白分隔）")});
    m_snipTable = makeTable({tr("觸發字"), tr("語言"), tr("內容（\\n 換行）")});
    m_keyTable  = makeTable({tr("動作"), tr("快捷鍵")});

    loadLsp();
    loadSnippets();
    loadKeys(actions);

    tabs->addTab(makePage(m_lspTable, true),  tr("LSP 伺服器"));
    tabs->addTab(makePage(m_snipTable, true), tr("Snippet"));
    tabs->addTab(makePage(m_keyTable, false), tr("快捷鍵"));

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    outer->addWidget(buttons);
}

QTableWidget* SettingsDialog::makeTable(const QStringList& headers) {
    auto* t = new QTableWidget(0, headers.size(), this);
    t->setHorizontalHeaderLabels(headers);
    t->horizontalHeader()->setSectionResizeMode(QHeaderView::Stretch);
    t->setSelectionBehavior(QAbstractItemView::SelectRows);
    return t;
}

void SettingsDialog::addRow(QTableWidget* t, const QStringList& values) {
    const int r = t->rowCount();
    t->insertRow(r);
    for (int c = 0; c < values.size() && c < t->columnCount(); ++c)
        t->setItem(r, c, new QTableWidgetItem(values[c]));
}

void SettingsDialog::loadLsp() {
    QFile f(m_lspPath);
    if (!f.open(QIODevice::ReadOnly)) return;
    const QJsonArray servers = QJsonDocument::fromJson(f.readAll()).object().value("servers").toArray();
    for (const QJsonValue& v : servers) {
        const QJsonObject o = v.toObject();
        QStringList exts, args;
        for (const QJsonValue& e : o.value("extensions").toArray()) exts << e.toString();
        for (const QJsonValue& a : o.value("args").toArray()) args << a.toString();
        addRow(m_lspTable, {o.value("languageId").toString(), exts.join(' '),
                            o.value("command").toString(), args.join(' ')});
    }
}

void SettingsDialog::loadSnippets() {
    QFile f(m_snippetPath);
    if (!f.open(QIODevice::ReadOnly)) return;
    for (const QJsonValue& v : QJsonDocument::fromJson(f.readAll()).array()) {
        const QJsonObject o = v.toObject();
        QString body = o.value("body").toString();
        body.replace('\n', QStringLiteral("\\n"));   // 表格內以字面 \n 顯示
        addRow(m_snipTable, {o.value("trigger").toString(), o.value("language").toString(), body});
    }
}

void SettingsDialog::loadKeys(const QList<QPair<QString, QString>>& actions) {
    for (const auto& a : actions) {
        addRow(m_keyTable, {a.first, a.second});
        m_keyTable->item(m_keyTable->rowCount() - 1, 0)->setFlags(Qt::ItemIsEnabled);  // 動作名不可改
    }
}

void SettingsDialog::save() const {
    auto cell = [](QTableWidget* t, int r, int c) {
        QTableWidgetItem* it = t->item(r, c);
        return it ? it->text() : QString();
    };

    // LSP → {servers:[...]}
    QJsonArray servers;
    for (int r = 0; r < m_lspTable->rowCount(); ++r) {
        const QString lang = cell(m_lspTable, r, 0).trimmed();
        const QString cmd  = cell(m_lspTable, r, 2).trimmed();
        if (lang.isEmpty() || cmd.isEmpty()) continue;
        QJsonArray exts, args;
        for (const QString& e : cell(m_lspTable, r, 1).split(' ', Qt::SkipEmptyParts)) exts.append(e);
        for (const QString& a : cell(m_lspTable, r, 3).split(' ', Qt::SkipEmptyParts)) args.append(a);
        servers.append(QJsonObject{{"languageId", lang}, {"extensions", exts},
                                   {"command", cmd}, {"args", args}});
    }
    QFile lf(m_lspPath);
    if (lf.open(QIODevice::WriteOnly))
        lf.write(QJsonDocument(QJsonObject{{"servers", servers}}).toJson(QJsonDocument::Indented));

    // Snippet → [...]
    QJsonArray snips;
    for (int r = 0; r < m_snipTable->rowCount(); ++r) {
        const QString trig = cell(m_snipTable, r, 0).trimmed();
        if (trig.isEmpty()) continue;
        QString body = cell(m_snipTable, r, 2);
        body.replace(QStringLiteral("\\n"), QStringLiteral("\n"));
        snips.append(QJsonObject{{"trigger", trig}, {"language", cell(m_snipTable, r, 1).trimmed()},
                                 {"body", body}});
    }
    QFile sf(m_snippetPath);
    if (sf.open(QIODevice::WriteOnly))
        sf.write(QJsonDocument(snips).toJson(QJsonDocument::Indented));

    // Keys → {action: shortcut}
    QJsonObject keys;
    for (int r = 0; r < m_keyTable->rowCount(); ++r) {
        const QString name = cell(m_keyTable, r, 0);
        if (!name.isEmpty()) keys.insert(name, cell(m_keyTable, r, 1).trimmed());
    }
    QFile kf(m_keymapPath);
    if (kf.open(QIODevice::WriteOnly))
        kf.write(QJsonDocument(keys).toJson(QJsonDocument::Indented));
}
