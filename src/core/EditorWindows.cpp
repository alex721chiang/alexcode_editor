#include "EditorWindows.h"
#include "CodeEditor.h"
#include <QCloseEvent>
#include <QCursor>
#include <QFileInfo>
#include <QHBoxLayout>
#include <QMenu>
#include <QMouseEvent>
#include <QShortcut>
#include <QTabBar>
#include <QTabWidget>
#include <QTextDocument>
#include <QPlainTextDocumentLayout>
#include <QToolButton>
#include <QVBoxLayout>
#include <functional>

namespace {
constexpr int kDetachDistance = 60;                // 放開位置需離開分頁列上下這麼遠才算「拖出」

// 獨立視窗：使用者按 X 時交給 controller 先把編輯器換回自有文件再銷毀
class DetachedWindow : public QWidget {
public:
    std::function<void()> onClose;
    DetachedWindow() : QWidget(nullptr, Qt::Window) {}
protected:
    void closeEvent(QCloseEvent* e) override {
        if (onClose) { auto fn = std::move(onClose); onClose = nullptr; fn(); }
        e->accept();
    }
};
}

EditorWindowsController::EditorWindowsController(QTabWidget* tabs, QWidget* host)
    : QObject(host), m_tabs(tabs), m_host(host) {
    QTabBar* bar = m_tabs->tabBar();
    bar->installEventFilter(this);
    bar->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(bar, &QWidget::customContextMenuRequested, this, &EditorWindowsController::showTabMenu);
}

EditorWindowsController::~EditorWindowsController() {
    closeAll();
}

QList<QWidget*> EditorWindowsController::detachedWindows() const {
    QList<QWidget*> out;
    for (const Entry& e : m_entries) out << e.window;
    return out;
}

CodeEditor* EditorWindowsController::viewOf(QWidget* window) const {
    const int i = indexOfWindow(window);
    return i >= 0 ? m_entries[i].view : nullptr;
}

int EditorWindowsController::indexOfWindow(const QWidget* w) const {
    for (int i = 0; i < m_entries.size(); ++i)
        if (m_entries[i].window == w) return i;
    return -1;
}

QWidget* EditorWindowsController::detach(CodeEditor* source) {
    if (!source) return nullptr;
    for (const Entry& e : m_entries) {
        if (e.source == source) {                  // 同一分頁不重複開
            e.window->show();
            e.window->raise();
            e.window->activateWindow();
            return e.window;
        }
    }
    auto* win = new DetachedWindow;
    const QString path = source->property("filePath").toString();
    QString title = source->property("baseTitle").toString();
    if (title.isEmpty()) title = path.isEmpty() ? tr("Untitled") : QFileInfo(path).fileName();
    win->setWindowTitle(title + QStringLiteral(" — AlexCode"));
    win->setWindowIcon(m_host->windowIcon());
    win->resize(800, 600);

    auto* lay = new QVBoxLayout(win);
    lay->setContentsMargins(0, 0, 0, 0);
    lay->setSpacing(0);
    auto* bar = new QHBoxLayout;
    bar->setContentsMargins(6, 4, 6, 4);
    auto* back = new QToolButton(win);
    back->setObjectName(QStringLiteral("backToMainButton"));
    back->setText(tr("回到主視窗"));
    back->setToolTip(tr("關閉此視窗並切到主視窗中的分頁（內容已即時同步）"));
    bar->addWidget(back);
    bar->addStretch();
    lay->addLayout(bar);

    auto* view = new CodeEditor(win);
    view->setFont(source->font());
    view->setLineWrapMode(source->lineWrapMode());
    // 備用文件由視窗持有（不可是編輯器的子文件：setDocument 換文件時會刪除編輯器擁有的舊文件）
    auto* own = new QTextDocument(win);
    own->setDocumentLayout(new QPlainTextDocumentLayout(own));
    view->setSharedDocument(source->document());   // 共用文件：編輯即同步，高亮由文件上的高亮器提供
    lay->addWidget(view);

    m_entries.append({source, win, view, own});
    win->onClose = [this, win]() {
        const int i = indexOfWindow(win);
        if (i >= 0) closeEntry(i);
    };
    connect(back, &QToolButton::clicked, this, [this, win]() {
        const int i = indexOfWindow(win);
        if (i < 0) return;
        QPointer<CodeEditor> src = m_entries[i].source;
        closeEntry(i);
        if (src) emit activateTabRequested(src);
    });
    auto* save = new QShortcut(QKeySequence::Save, win);
    connect(save, &QShortcut::activated, this, [this, win]() {
        const int i = indexOfWindow(win);
        if (i >= 0 && m_entries[i].source) emit saveRequested(m_entries[i].source);
    });

    win->show();
    win->raise();
    win->activateWindow();
    return win;
}

void EditorWindowsController::closeEntry(int i) {
    Entry e = m_entries.takeAt(i);
    static_cast<DetachedWindow*>(e.window)->onClose = nullptr;
    e.view->setSharedDocument(e.ownDoc);           // 先脫離共用文件（分頁可能即將銷毀它）
    e.window->hide();
    e.window->deleteLater();
}

void EditorWindowsController::editorClosing(CodeEditor* source) {
    for (int i = m_entries.size() - 1; i >= 0; --i)
        if (m_entries[i].source == source) closeEntry(i);
}

void EditorWindowsController::closeAll() {
    while (!m_entries.isEmpty()) closeEntry(m_entries.size() - 1);
}

bool EditorWindowsController::eventFilter(QObject* watched, QEvent* event) {
    auto* bar = qobject_cast<QTabBar*>(watched);
    if (!bar) return QObject::eventFilter(watched, event);
    if (event->type() == QEvent::MouseButtonPress) {
        auto* me = static_cast<QMouseEvent*>(event);
        if (me->button() == Qt::LeftButton) {
            const int idx = bar->tabAt(me->position().toPoint());
            m_pressWidget = idx >= 0 ? m_tabs->widget(idx) : nullptr;
        }
    } else if (event->type() == QEvent::MouseButtonRelease) {
        auto* me = static_cast<QMouseEvent*>(event);
        if (me->button() == Qt::LeftButton && m_pressWidget) {
            const QPoint p = me->position().toPoint();
            const bool outside = p.y() > bar->height() + kDetachDistance || p.y() < -kDetachDistance;
            if (outside) {
                if (auto* src = qobject_cast<CodeEditor*>(m_pressWidget.data())) {
                    QWidget* win = detach(src);
                    if (win) win->move(me->globalPosition().toPoint() - QPoint(win->width() / 2, 20));
                }
            }
        }
        if (me->button() == Qt::LeftButton) m_pressWidget = nullptr;
    }
    return QObject::eventFilter(watched, event);   // 不攔截：分頁列照常處理（排序、切換）
}

void EditorWindowsController::showTabMenu(const QPoint& pos) {
    QTabBar* bar = m_tabs->tabBar();
    const int idx = bar->tabAt(pos);
    auto* editor = idx >= 0 ? qobject_cast<CodeEditor*>(m_tabs->widget(idx)) : nullptr;
    if (!editor) return;
    QMenu menu;
    menu.addAction(tr("在新視窗開啟（可拖到另一個螢幕）"), this, [this, editor]() { detach(editor); });
    menu.addAction(tr("在分割檢視中並排顯示"), this, [this, editor]() { emit pinSplitRequested(editor); });
    menu.exec(bar->mapToGlobal(pos));
}
