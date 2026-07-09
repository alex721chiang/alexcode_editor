#include "SuggestionWidget.h"
#include "Theme.h"
#include <QApplication>

SuggestionWidget::SuggestionWidget(QWidget *parent)
    : QListWidget(parent)
{
    setWindowFlags(Qt::ToolTip | Qt::FramelessWindowHint);
    setFocusPolicy(Qt::StrongFocus);
    // 跟主題（原本寫死灰色系，與霓虹主題格格不入、Paper Light 下也突兀）
    setStyleSheet(QStringLiteral(
        "QListWidget { border: 1px solid %1; border-radius: 6px; background-color: %2;"
        " color: %3; selection-background-color: %4; selection-color: %1; }")
        .arg(Theme::ACCENT, Theme::LINE_NUM_BG, Theme::EDITOR_FG, Theme::BRACKET_MATCH_BG));
}

void SuggestionWidget::showSuggestions(const QStringList &suggestions, const QPoint &pos) {
    clear();
    addItems(suggestions);
    if (count() > 0) {
        setCurrentRow(0);
        resize(300, qMin(200, count() * 25 + 10));
        move(pos);
        show();
        setFocus();
    }
}

void SuggestionWidget::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Enter || event->key() == Qt::Key_Return || event->key() == Qt::Key_Tab) {
        if (currentItem()) {
            emit suggestionSelected(currentItem()->text());
            hide();
        }
        event->accept();
    } else if (event->key() == Qt::Key_Escape) {
        hide();
        event->accept();
    } else {
        QListWidget::keyPressEvent(event);
    }
}

void SuggestionWidget::focusOutEvent(QFocusEvent *event) {
    hide();
    QListWidget::focusOutEvent(event);
}
