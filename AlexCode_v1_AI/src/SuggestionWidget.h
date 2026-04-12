#pragma once

#include <QListWidget>
#include <QKeyEvent>

/**
 * @brief SuggestionWidget 是一個浮動在游標旁邊的 AI 建議列表
 */
class SuggestionWidget : public QListWidget {
    Q_OBJECT

public:
    explicit SuggestionWidget(QWidget *parent = nullptr);

    // 顯示建議並定位到游標位置
    void showSuggestions(const QStringList &suggestions, const QPoint &pos);

signals:
    void suggestionSelected(const QString &text);

protected:
    void keyPressEvent(QKeyEvent *event) override;
    void focusOutEvent(QFocusEvent *event) override;
};
