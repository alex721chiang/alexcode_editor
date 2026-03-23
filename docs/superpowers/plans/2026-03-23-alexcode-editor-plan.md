# AlexCode Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立一個支援多關鍵字過濾 (AND/OR)、且具備 C/C++/Python 語法高亮與行號顯示的高效能桌面文字編輯器。

**Architecture:** 專案採 MVC 架構概念。核心過濾邏輯抽離至獨立的 `FilterEngine` 類別以便進行單元測試。UI 層使用 Qt 6 的 `QMainWindow` 整合 `QPlainTextEdit` (主編輯區) 與 `QDockWidget` (結果面板)。所有組件透過 Qt Signals and Slots 進行非同步/事件驅動的溝通。

**Tech Stack:** C++17, Qt 6 (Core, Gui, Widgets), CMake, GoogleTest

---

### Task 1: Project Scaffolding & CMake Setup

**Files:**
- Create: `CMakeLists.txt`
- Create: `tests/CMakeLists.txt`
- Create: `src/main.cpp`

- [ ] **Step 1: Write root CMakeLists.txt**

```cmake
cmake_minimum_required(VERSION 3.16)
project(AlexCode VERSION 1.0.0 LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 COMPONENTS Core Gui Widgets REQUIRED)

add_subdirectory(src)
enable_testing()
add_subdirectory(tests)
```

- [ ] **Step 2: Write src/CMakeLists.txt and main.cpp skeleton**

`src/CMakeLists.txt`:
```cmake
add_executable(AlexCode main.cpp)
target_link_libraries(AlexCode PRIVATE Qt6::Widgets)
```

`src/main.cpp`:
```cpp
#include <QApplication>
#include <QWidget>
int main(int argc, char *argv[]) {
    QApplication a(argc, argv);
    QWidget w;
    w.show();
    return a.exec();
}
```

- [ ] **Step 3: Build to verify setup**

Run: `mkdir build && cd build && cmake .. && make`
Expected: Builds successfully and generates AlexCode executable.

- [ ] **Step 4: Commit**

```bash
git add CMakeLists.txt src/ tests/
git commit -m "chore: setup CMake project structure with Qt6"
```

### Task 2: Core Filter Logic (TDD)

**Files:**
- Create: `src/FilterEngine.h`, `src/FilterEngine.cpp`
- Create: `tests/FilterEngineTest.cpp`
- Modify: `src/CMakeLists.txt`, `tests/CMakeLists.txt`

- [ ] **Step 1: Setup GoogleTest in tests/CMakeLists.txt**

```cmake
include(FetchContent)
FetchContent_Declare(googletest URL https://github.com/google/googletest/archive/03597a01ee50ed33e9dfd640b249b4be3799d395.zip)
FetchContent_MakeAvailable(googletest)
add_executable(AlexCodeTests FilterEngineTest.cpp ../src/FilterEngine.cpp)
target_link_libraries(AlexCodeTests PRIVATE gtest_main)
```

- [ ] **Step 2: Write failing test for FilterEngine**

`tests/FilterEngineTest.cpp`:
```cpp
#include <gtest/gtest.h>
#include "../src/FilterEngine.h"

TEST(FilterEngineTest, FilterOrLogic) {
    FilterEngine engine;
    engine.setKeywords({"error", "fail"});
    engine.setLogic(FilterLogic::OR);
    
    EXPECT_TRUE(engine.matchLine("This is an error log"));
    EXPECT_TRUE(engine.matchLine("Process failed successfully"));
    EXPECT_FALSE(engine.matchLine("Everything is fine"));
}

TEST(FilterEngineTest, FilterAndLogic) {
    FilterEngine engine;
    engine.setKeywords({"error", "timeout"});
    engine.setLogic(FilterLogic::AND);
    
    EXPECT_TRUE(engine.matchLine("error: connection timeout occurred"));
    EXPECT_FALSE(engine.matchLine("This is an error log"));
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd build && cmake .. && make AlexCodeTests && ./tests/AlexCodeTests`
Expected: Compile error because `FilterEngine` doesn't exist.

- [ ] **Step 4: Write minimal implementation**

`src/FilterEngine.h`:
```cpp
#pragma once
#include <QStringList>
#include <QString>

enum class FilterLogic { AND, OR };

class FilterEngine {
public:
    void setKeywords(const QStringList& keywords);
    void setLogic(FilterLogic logic);
    bool matchLine(const QString& line) const;
private:
    QStringList m_keywords;
    FilterLogic m_logic = FilterLogic::OR;
};
```

`src/FilterEngine.cpp`:
```cpp
#include "FilterEngine.h"

void FilterEngine::setKeywords(const QStringList& keywords) { m_keywords = keywords; }
void FilterEngine::setLogic(FilterLogic logic) { m_logic = logic; }
bool FilterEngine::matchLine(const QString& line) const {
    if (m_keywords.isEmpty()) return true;
    if (m_logic == FilterLogic::OR) {
        for (const auto& kw : m_keywords) {
            if (line.contains(kw, Qt::CaseInsensitive)) return true;
        }
        return false;
    } else {
        for (const auto& kw : m_keywords) {
            if (!line.contains(kw, Qt::CaseInsensitive)) return false;
        }
        return true;
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd build && make AlexCodeTests && ./tests/AlexCodeTests`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/ tests/
git commit -m "feat: implement FilterEngine with AND/OR logic using TDD"
```

### Task 3: MainWindow & Filter UI Layout

**Files:**
- Create: `src/MainWindow.h`, `src/MainWindow.cpp`
- Modify: `src/CMakeLists.txt`, `src/main.cpp`

- [ ] **Step 1: Write MainWindow definitions**

`src/MainWindow.h`:
```cpp
#pragma once
#include <QMainWindow>
#include <QPlainTextEdit>
#include <QListWidget>
#include <QLineEdit>
#include <QComboBox>

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);
private:
    QPlainTextEdit* editor;
    QListWidget* resultsList;
    QLineEdit* filterInput;
    QComboBox* logicCombo;
    void setupUI();
};
```

- [ ] **Step 2: Implement UI layout**

`src/MainWindow.cpp`:
```cpp
#include "MainWindow.h"
#include <QToolBar>
#include <QDockWidget>
#include <QPushButton>

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
    setupUI();
    resize(800, 600);
}

void MainWindow::setupUI() {
    editor = new QPlainTextEdit(this);
    setCentralWidget(editor);

    QToolBar* toolbar = addToolBar("Filter");
    filterInput = new QLineEdit(this);
    filterInput->setPlaceholderText("Enter keywords separated by |");
    logicCombo = new QComboBox(this);
    logicCombo->addItems({"OR", "AND"});
    QPushButton* filterBtn = new QPushButton("Filter", this);

    toolbar->addWidget(filterInput);
    toolbar->addWidget(logicCombo);
    toolbar->addWidget(filterBtn);

    QDockWidget* dock = new QDockWidget("Filter Results", this);
    resultsList = new QListWidget(this);
    dock->setWidget(resultsList);
    addDockWidget(Qt::BottomDockWidgetArea, dock);
}
```

- [ ] **Step 3: Update main.cpp and CMakeLists**

Update `main.cpp` to use `MainWindow`. Add `MainWindow.cpp` to `src/CMakeLists.txt`.

- [ ] **Step 4: Build and run**

Run: `cd build && make AlexCode && ./src/AlexCode`
Expected: Application launches with Editor, Toolbar, and Bottom dock.

- [ ] **Step 5: Commit**

```bash
git add src/
git commit -m "feat: implement main window layout with filter toolbar and results dock"
```

### Task 4: Connect FilterEngine to UI

**Files:**
- Modify: `src/MainWindow.h`, `src/MainWindow.cpp`

- [ ] **Step 1: Add FilterEngine to MainWindow**

Add `#include "FilterEngine.h"` and `FilterEngine engine;` to `MainWindow.h`.
Add `private slots: void runFilter();`

- [ ] **Step 2: Implement runFilter slot**

`src/MainWindow.cpp` (in runFilter):
```cpp
void MainWindow::runFilter() {
    resultsList->clear();
    QStringList keywords = filterInput->text().split("|", Qt::SkipEmptyParts);
    for (int i=0; i<keywords.size(); ++i) keywords[i] = keywords[i].trimmed();
    
    engine.setKeywords(keywords);
    engine.setLogic(logicCombo->currentText() == "AND" ? FilterLogic::AND : FilterLogic::OR);
    
    QString text = editor->toPlainText();
    QStringList lines = text.split("\n");
    for (int i = 0; i < lines.size(); ++i) {
        if (engine.matchLine(lines[i])) {
            QListWidgetItem* item = new QListWidgetItem(QString("Line %1: %2").arg(i + 1).arg(lines[i]));
            item->setData(Qt::UserRole, i); // Store line number (0-indexed) for safe click-to-jump
            resultsList->addItem(item);
        }
    }
}
```
Connect `filterBtn->clicked` to `runFilter` in `setupUI`.

- [ ] **Step 3: Connect click-to-jump**

Add slot `void onResultDoubleClicked(QListWidgetItem* item);`.
Retrieve the line number using `item->data(Qt::UserRole).toInt()`, use `QTextCursor` to jump the `editor` to that line.

- [ ] **Step 4: Build and test manually**

Run: `cd build && make AlexCode && ./src/AlexCode`
Type text in editor, filter, double click result.

- [ ] **Step 5: Commit**

```bash
git add src/MainWindow.*
git commit -m "feat: integrate FilterEngine and enable click-to-jump in results"
```
*(Line Numbers and Syntax Highlighting are deferred to next phase to keep scope controlled)*

### Task 5: Tabbed UI & File I/O Operations

**Files:**
- Modify: `src/MainWindow.h`, `src/MainWindow.cpp`

- [ ] **Step 1: Replace QPlainTextEdit with QTabWidget**

Modify `MainWindow.h` to use `QTabWidget* tabWidget;` instead of a single `QPlainTextEdit* editor;`.
Update `setupUI` to set the central widget to `tabWidget`.

- [ ] **Step 2: Add File Menu Actions (Open, Save)**

Add `void openFile();`, `void saveFile();` slots in `MainWindow.h`.
In `MainWindow.cpp`, create a menu bar with "File -> Open", "File -> Save".
Connect them to the slots.

- [ ] **Step 3: Implement File Operations**

In `openFile()`: use `QFileDialog::getOpenFileName`. If valid, read file, create a new `QPlainTextEdit`, set its text, and `addTab()` to `tabWidget`.
In `saveFile()`: get current widget from `tabWidget`, cast to `QPlainTextEdit`, save its content to the associated file.

- [ ] **Step 4: Update Filter Logic for Active Tab**

Update `runFilter()` to get the current text from `tabWidget->currentWidget()`.

- [ ] **Step 5: Commit**

```bash
git add src/MainWindow.*
git commit -m "feat: add tabbed UI and basic file open/save operations"
```

### Task 6: View Controls & Syntax Highlighting

**Files:**
- Create: `src/SyntaxHighlighter.h`, `src/SyntaxHighlighter.cpp`
- Create: `src/CodeEditor.h`, `src/CodeEditor.cpp`
- Modify: `src/MainWindow.h`, `src/MainWindow.cpp`, `src/CMakeLists.txt`

- [ ] **Step 1: Implement basic QSyntaxHighlighter**

Create `SyntaxHighlighter` subclassing `QSyntaxHighlighter`.
Define basic rules (C++ keywords, strings, comments) using `QRegularExpression`.
Apply it to the document passed to it.

- [ ] **Step 2: Implement CodeEditor with Line Numbers**

Create `CodeEditor` subclassing `QPlainTextEdit`.
Implement the standard Qt line number area (handle `updateRequest`, `updateLineNumberAreaWidth`, override `resizeEvent`).

- [ ] **Step 3: Add View Controls, Zoom, and Word Wrap to CodeEditor**

Add "View" menu to `MainWindow` and add toggles for "Word Wrap" (sets `setLineWrapMode`).
In `CodeEditor`, explicitly override `wheelEvent(QWheelEvent *event)`: if `event->modifiers() & Qt::ControlModifier`, dynamically adjust `font()` size to support Ctrl+Mouse Wheel zooming.

- [ ] **Step 4: Integration & Python Syntax**

Update `openFile()` to create `CodeEditor` instances instead of standard `QPlainTextEdit`.
Attach the `SyntaxHighlighter` to the `CodeEditor`'s document.
Expand `SyntaxHighlighter` rules to also detect basic Python keywords (`def`, `class`, `import`, `print`) and Python-style comments (`#`).

- [ ] **Step 5: Add Edit Menu (Undo/Redo, Cut/Copy/Paste, Find)**

In `MainWindow.cpp`, create an "Edit" menu.
Connect standard Qt actions: `undo`, `redo`, `cut`, `copy`, `paste` to the active `CodeEditor`.
Implement a basic Ctrl+F (Find) dialog or tool window that searches the active `CodeEditor` using `find()` and highlights the match.

- [ ] **Step 6: Write Performance Test**

Create `tests/PerformanceTest.cpp`.
Generate a mock 100,000 line text file. Instantiate `FilterEngine`, set a keyword, and measure the `matchLine` execution time across all lines.
Assert that the total execution time is under 500ms using `std::chrono`.

- [ ] **Step 7: Commit**

```bash
git add src/ tests/
git commit -m "feat: implement line numbers, syntax highlighting (C++/Python), view controls, edit actions, basic find, and perf test"
```