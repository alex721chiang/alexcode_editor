#pragma once
#include <QDialog>
#include <QVector>
#include <QString>

class QLineEdit;
class QListWidget;
class QAction;

// Ctrl+Shift+P 命令面板：彙整所有 QAction，模糊搜尋並執行（顯示對應快捷鍵）。
class CommandPalette : public QDialog {
    Q_OBJECT
public:
    struct Command { QString name; QString shortcut; QAction* action; };

    explicit CommandPalette(QWidget* parent = nullptr);
    void openWith(const QVector<Command>& cmds, const QString& initialQuery = QString());

protected:
    bool eventFilter(QObject* obj, QEvent* event) override;

private:
    void refresh(const QString& query);
    void acceptCurrent();

    QLineEdit* input;
    QListWidget* list;
    QVector<Command> m_cmds;
};
