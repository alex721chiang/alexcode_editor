#pragma once
#include <QDialog>

class QLineEdit;
class QListWidget;
class ProjectSymbolIndex;

// Ctrl+T：在整個專案的符號索引中模糊搜尋並跳轉（顯示所屬檔與行）。
class ProjectSymbolDialog : public QDialog {
    Q_OBJECT
public:
    explicit ProjectSymbolDialog(QWidget* parent = nullptr);
    void openWith(const ProjectSymbolIndex* index, const QString& initialQuery = QString());

signals:
    void symbolChosen(const QString& file, int line);   // line 0-based

protected:
    bool eventFilter(QObject* obj, QEvent* event) override;

private:
    void refresh(const QString& query);
    void acceptCurrent();

    QLineEdit* input;
    QListWidget* list;
    const ProjectSymbolIndex* m_index = nullptr;
};
