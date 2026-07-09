#include "FindController.h"
#include "CodeEditor.h"
#include "Theme.h"
#include <QDialog>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLineEdit>
#include <QCheckBox>
#include <QLabel>
#include <QPushButton>
#include <QTimer>
#include <QListWidget>
#include <QDockWidget>
#include <QTextBlock>

FindController::FindController(QWidget* dialogParent, EditorFn activeEditor, EditorsFn allEditors,
                               QListWidget* resultsList, QLabel* filterCountLabel,
                               QDockWidget* filterResultsDock, QObject* parent)
    : QObject(parent), m_dialogParent(dialogParent),
      m_activeEditor(std::move(activeEditor)), m_allEditors(std::move(allEditors)),
      m_resultsList(resultsList), m_filterCountLabel(filterCountLabel),
      m_filterResultsDock(filterResultsDock) {}

void FindController::ensureDialog() {
    if (m_dialog) return;
    m_dialog = new QDialog(m_dialogParent);
    m_dialog->setWindowTitle("Find & Replace");
    QGridLayout* layout = new QGridLayout(m_dialog);
    layout->addWidget(new QLabel("Find:", m_dialog), 0, 0);
    m_findInput = new QLineEdit(m_dialog);
    layout->addWidget(m_findInput, 0, 1, 1, 3);
    layout->addWidget(new QLabel("Replace:", m_dialog), 1, 0);
    m_replaceInput = new QLineEdit(m_dialog);
    layout->addWidget(m_replaceInput, 1, 1, 1, 3);

    m_caseCheck      = new QCheckBox("Match case", m_dialog);
    m_wholeWordCheck = new QCheckBox("Whole word", m_dialog);
    m_regexCheck     = new QCheckBox("Regex", m_dialog);
    layout->addWidget(m_caseCheck, 2, 1);
    layout->addWidget(m_wholeWordCheck, 2, 2);
    layout->addWidget(m_regexCheck, 2, 3);

    m_countLabel = new QLabel(m_dialog);
    m_countLabel->setStyleSheet(QStringLiteral("color:%1;").arg(Theme::ACCENT));
    layout->addWidget(m_countLabel, 3, 1, 1, 3);

    m_replaceAllTabsCheck = new QCheckBox(tr("套用到全部開啟分頁"), m_dialog);
    layout->addWidget(m_replaceAllTabsCheck, 4, 1, 1, 3);

    QPushButton* findNextBtn   = new QPushButton("Find Next", m_dialog);
    QPushButton* findPrevBtn   = new QPushButton("Find Prev", m_dialog);
    QPushButton* replaceBtn    = new QPushButton("Replace", m_dialog);
    QPushButton* replaceAllBtn = new QPushButton("Replace All", m_dialog);
    QHBoxLayout* btnLayout = new QHBoxLayout();
    btnLayout->addWidget(findNextBtn);
    btnLayout->addWidget(findPrevBtn);
    btnLayout->addWidget(replaceBtn);
    btnLayout->addWidget(replaceAllBtn);
    layout->addLayout(btnLayout, 5, 0, 1, 4);

    QPushButton* countBtn    = new QPushButton(tr("Count"), m_dialog);
    QPushButton* markAllBtn  = new QPushButton(tr("Mark All"), m_dialog);
    QPushButton* findAllBtn  = new QPushButton(tr("Find All"), m_dialog);
    QHBoxLayout* btnLayout2 = new QHBoxLayout();
    btnLayout2->addWidget(countBtn);
    btnLayout2->addWidget(markAllBtn);
    btnLayout2->addWidget(findAllBtn);
    layout->addLayout(btnLayout2, 6, 0, 1, 4);

    connect(findNextBtn,   &QPushButton::clicked, this, &FindController::findNext);
    connect(findPrevBtn,   &QPushButton::clicked, this, &FindController::findPrev);
    connect(replaceBtn,    &QPushButton::clicked, this, &FindController::performReplace);
    connect(replaceAllBtn, &QPushButton::clicked, this, &FindController::performReplaceAll);
    connect(countBtn,      &QPushButton::clicked, this, &FindController::performFindCount);
    connect(markAllBtn,    &QPushButton::clicked, this, &FindController::performMarkAll);
    connect(findAllBtn,    &QPushButton::clicked, this, &FindController::performFindAll);
    connect(m_findInput, &QLineEdit::returnPressed, this, &FindController::findNext);

    // 輸入時即時標示所有符合項目；計數走防抖（大檔全文掃描有成本，
    // 每敲一鍵就掃一次會卡輸入，高亮本身是編輯器增量繪製、可即時）
    m_countTimer = new QTimer(m_dialog);
    m_countTimer->setSingleShot(true);
    m_countTimer->setInterval(300);
    connect(m_countTimer, &QTimer::timeout, this, &FindController::updateFindCount);
    auto refreshHighlight = [this]() {
        if (auto editor = m_activeEditor()) {
            editor->setSearchHighlightPattern(buildRegex());
            m_countTimer->start();
        }
    };
    connect(m_findInput, &QLineEdit::textChanged, this, refreshHighlight);
    connect(m_caseCheck, &QCheckBox::toggled, this, refreshHighlight);
    connect(m_wholeWordCheck, &QCheckBox::toggled, this, refreshHighlight);
    connect(m_regexCheck, &QCheckBox::toggled, this, refreshHighlight);
    connect(m_dialog, &QDialog::finished, this, [this](int) {
        if (auto editor = m_activeEditor())
            editor->setSearchHighlightPattern(QRegularExpression());
    });
}

void FindController::showDialog() {
    ensureDialog();
    // 預填目前選取文字
    if (auto editor = m_activeEditor()) {
        const QString sel = editor->textCursor().selectedText();
        if (!sel.isEmpty() && !sel.contains(QChar(0x2029)))
            m_findInput->setText(sel);
    }
    m_dialog->show();
    m_dialog->raise();
    m_dialog->activateWindow();
    m_findInput->setFocus();
    m_findInput->selectAll();
}

QRegularExpression FindController::buildRegex() const {
    if (!m_findInput) return QRegularExpression();
    QString text = m_findInput->text();
    if (text.isEmpty()) return QRegularExpression();

    QString pattern = (m_regexCheck && m_regexCheck->isChecked())
                          ? text : QRegularExpression::escape(text);
    if (m_wholeWordCheck && m_wholeWordCheck->isChecked())
        pattern = QStringLiteral("\\b") + pattern + QStringLiteral("\\b");

    QRegularExpression::PatternOptions opts = QRegularExpression::NoPatternOption;
    if (!m_caseCheck || !m_caseCheck->isChecked())
        opts |= QRegularExpression::CaseInsensitiveOption;
    return QRegularExpression(pattern, opts);
}

QTextDocument::FindFlags FindController::buildFlags(bool backward) const {
    QTextDocument::FindFlags flags;
    if (backward) flags |= QTextDocument::FindBackward;
    // 修正既有 bug：QTextDocument::find() 對 QRegularExpression 版本的多載，
    // 大小寫是否相符要看這個旗標，不是看 regex 本身的 CaseInsensitiveOption
    // （這點跟 QRegularExpression::match() 的行為不一樣，很容易誤踩）。
    // 少了這一行，「Match case」核取方塊勾了也不會有效果。
    if (m_caseCheck && m_caseCheck->isChecked()) flags |= QTextDocument::FindCaseSensitively;
    return flags;
}

void FindController::findNext() {
    auto editor = m_activeEditor();
    if (!editor || !m_dialog) { showDialog(); return; }
    QRegularExpression re = buildRegex();
    if (!re.isValid() || re.pattern().isEmpty()) return;

    if (!editor->find(re, buildFlags(false))) {
        QTextCursor cursor = editor->textCursor();
        cursor.movePosition(QTextCursor::Start);
        editor->setTextCursor(cursor);
        if (!editor->find(re, buildFlags(false)))
            emit statusMessage("Cannot find \"" + m_findInput->text() + "\"", 3000);
    }
}

void FindController::findPrev() {
    auto editor = m_activeEditor();
    if (!editor || !m_dialog) { showDialog(); return; }
    QRegularExpression re = buildRegex();
    if (!re.isValid() || re.pattern().isEmpty()) return;

    if (!editor->find(re, buildFlags(true))) {
        QTextCursor cursor = editor->textCursor();
        cursor.movePosition(QTextCursor::End);
        editor->setTextCursor(cursor);
        if (!editor->find(re, buildFlags(true)))
            emit statusMessage("Cannot find \"" + m_findInput->text() + "\"", 3000);
    }
}

void FindController::performReplace() {
    if (auto editor = m_activeEditor()) {
        if (!m_dialog) return;
        QRegularExpression re = buildRegex();
        if (!re.isValid() || re.pattern().isEmpty()) return;
        QString replaceText = m_replaceInput->text();
        QTextCursor cursor = editor->textCursor();
        if (cursor.hasSelection() && re.match(cursor.selectedText()).capturedLength() == cursor.selectedText().length())
            cursor.insertText(replaceText);
        findNext();
    }
}

void FindController::performReplaceAll() {
    if (!m_dialog) return;
    QRegularExpression re = buildRegex();
    if (!re.isValid() || re.pattern().isEmpty()) return;
    const QString replaceText = m_replaceInput->text();

    // 對單一編輯器做全部取代（單一 Undo 步驟），回傳取代筆數
    const QTextDocument::FindFlags findFlags = buildFlags();
    auto replaceInEditor = [&re, &replaceText, findFlags](CodeEditor* editor) -> int {
        QTextCursor cursor(editor->document());
        cursor.beginEditBlock();
        int count = 0;
        QTextCursor found = editor->document()->find(re, 0, findFlags);
        while (!found.isNull()) {
            const int after = found.selectionEnd();
            found.insertText(replaceText);
            count++;
            int next = found.position();
            if (replaceText.isEmpty() && next == after) next++;   // 避免空字串無限迴圈
            found = editor->document()->find(re, next, findFlags);
        }
        cursor.endEditBlock();
        return count;
    };

    if (m_replaceAllTabsCheck && m_replaceAllTabsCheck->isChecked()) {
        int total = 0, filesTouched = 0;
        const QList<CodeEditor*> editors = m_allEditors();
        for (CodeEditor* e : editors) {
            const int n = replaceInEditor(e);
            if (n > 0) { total += n; ++filesTouched; }
        }
        emit statusMessage(tr("%1 個分頁、共 %2 處取代完成").arg(filesTouched).arg(total), 5000);
    } else if (auto editor = m_activeEditor()) {
        const int count = replaceInEditor(editor);
        emit statusMessage(QString::number(count) + " replacements made.", 4000);
    }
}

// 即時計數（防抖後執行）：超過上限即停止掃描，避免超大檔案拖慢輸入；
// 需要精確數字時用 Count 按鈕（performFindCount，無上限）
void FindController::updateFindCount() {
    if (!m_countLabel) return;
    auto editor = m_activeEditor();
    if (!editor) return;
    const QRegularExpression re = buildRegex();
    if (!re.isValid() || re.pattern().isEmpty()) { m_countLabel->clear(); return; }
    constexpr int kCountCap = 5000;
    int n = 0;
    QTextCursor c(editor->document());
    while (!(c = editor->document()->find(re, c, buildFlags())).isNull())
        if (++n >= kCountCap) break;
    m_countLabel->setText(n >= kCountCap ? tr("%1+ 個符合").arg(kCountCap)
                                         : tr("%1 個符合").arg(n));
}

// Find 強化：計數（不標示，只回報符合筆數 —— Notepad++ 的 Count 按鈕）
void FindController::performFindCount() {
    auto editor = m_activeEditor();
    if (!editor) return;
    const QRegularExpression re = buildRegex();
    if (!re.isValid() || re.pattern().isEmpty()) {
        emit statusMessage(tr("請先輸入搜尋內容"), 3000);
        return;
    }
    int n = 0;
    QTextCursor c(editor->document());
    while (!(c = editor->document()->find(re, c, buildFlags())).isNull()) ++n;
    if (m_countLabel) m_countLabel->setText(tr("%1 個符合").arg(n));
    emit statusMessage(tr("找到 %1 個符合").arg(n), 4000);
}

// Find 強化：標示全部符合（等同即時高亮，但可在不打字的情況下手動觸發一次）
void FindController::performMarkAll() {
    auto editor = m_activeEditor();
    if (!editor) return;
    const QRegularExpression re = buildRegex();
    editor->setSearchHighlightPattern(re);
    if (!re.isValid() || re.pattern().isEmpty()) {
        if (m_countLabel) m_countLabel->clear();
        return;
    }
    int n = 0;
    QTextCursor c(editor->document());
    while (!(c = editor->document()->find(re, c, buildFlags())).isNull()) ++n;
    if (m_countLabel) m_countLabel->setText(tr("%1 個符合").arg(n));
    emit statusMessage(tr("已標示 %1 處符合").arg(n), 4000);
}

// Find 強化：Find All → 結果清單，重用既有的 FILTER RESULTS 面板（雙擊跳轉邏輯共用 onResultDoubleClicked）
void FindController::performFindAll() {
    auto editor = m_activeEditor();
    if (!editor || !m_resultsList) return;
    const QRegularExpression re = buildRegex();
    if (!re.isValid() || re.pattern().isEmpty()) {
        emit statusMessage(tr("請先輸入搜尋內容"), 3000);
        return;
    }

    m_resultsList->clear();
    int count = 0;
    QTextBlock block = editor->document()->begin();
    int lineIndex = 0;
    while (block.isValid()) {
        if (re.match(block.text()).hasMatch()) {
            auto* item = new QListWidgetItem(
                QStringLiteral("Line %1: %2").arg(lineIndex + 1).arg(block.text()));
            item->setData(Qt::UserRole, lineIndex);
            m_resultsList->addItem(item);
            ++count;
        }
        block = block.next();
        ++lineIndex;
    }
    if (m_filterCountLabel) m_filterCountLabel->setText(tr("%1 hits").arg(count));
    if (m_filterResultsDock) { m_filterResultsDock->show(); m_filterResultsDock->raise(); }
    emit statusMessage(tr("Find All：找到 %1 處符合").arg(count), 4000);
}
