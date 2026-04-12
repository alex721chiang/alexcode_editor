#include "SuggestionWidget.h"
#include <QApplication>

SuggestionWidget::SuggestionWidget(QWidget *parent)
    : QListWidget(parent) 
{
    setWindowFlags(Qt::ToolTip | Qt::FramelessWindowHint);
    setFocusPolicy(Qt::StrongFocus);
    setStyleSheet("QListWidget { border: 1px solid #444; background-color: #2b2b2b; color: #ccc; selection-background-color: #4b6eaf; }");
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
