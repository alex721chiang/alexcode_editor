#pragma once
#include <QDialog>
#include <QVector>
#include "TsSymbols.h"

class QLineEdit;
class QListWidget;

// Ctrl+Shift+O：列出目前檔的類別/函式，模糊比對快速跳轉（重用 FilterEngine::fuzzyContains）。
class SymbolDialog : public QDialog {
    Q_OBJECT
public:
    explicit SymbolDialog(QWidget* parent = nullptr);
    void openWith(const QVector<TsSymbols::Symbol>& syms);

signals:
    void symbolChosen(int line);          // 0-based 行號

protected:
    bool eventFilter(QObject* obj, QEvent* event) override;

private:
    void refresh(const QString& query);
    void acceptCurrent();

    QLineEdit* input;
    QListWidget* list;
    QVector<TsSymbols::Symbol> m_syms;
};
