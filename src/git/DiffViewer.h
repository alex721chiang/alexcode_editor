#pragma once
#include <QWidget>
#include <QStringList>
#include "DiffCalc.h"

class QPlainTextEdit;

// 並排（side-by-side）diff 檢視器：左舊右新、同步捲動、增刪改整行底色。
// 以 DiffCalc::align 的結果渲染；填充列顯示為空行（面板底色），讓左右逐列對齊。
// 放進 tabWidget 當一般分頁使用（唯讀，關閉不會提示儲存）。
class DiffViewer : public QWidget {
    Q_OBJECT
public:
    DiffViewer(const QString& titleA, const QStringList& linesA,
               const QString& titleB, const QStringList& linesB,
               const QList<DiffCalc::Row>& rows, QWidget* parent = nullptr);

    int changeCount() const { return m_changes; }

private:
    QPlainTextEdit* makePane(QWidget* parent) const;
    int m_changes = 0;
};
