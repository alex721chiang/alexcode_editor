# AlexCode Editor v2.0 Features Plan

## Overview
Implement three major user-requested features:
1. Font Family and Size Configuration.
2. Find & Replace (Replace All).
3. Find in Files (Recursive directory search with extension filtering).

## Task 1: Font Configuration
**Objective**: Allow users to change the editor font.
- **UI Updates**:
  - Add `QAction* fontAction` to the "View" menu.
- **Logic**:
  - On trigger, open `QFontDialog::getFont()`.
  - If a font is selected, apply it to all existing `CodeEditor` instances in `tabWidget`.
  - Store the selected `QFont` as a member variable in `MainWindow` to apply to any newly opened/created files.

## Task 2: Find & Replace
**Objective**: Enhance the existing Find dialog with replace capabilities.
- **UI Updates**:
  - Expand `findDialog` layout in `MainWindow.cpp`.
  - Add `replaceInput` (`QLineEdit`).
  - Add "Replace" and "Replace All" buttons.
- **Logic**:
  - `performReplace()`: Replaces the currently selected text if it matches `findInput`, then finds the next occurrence.
  - `performReplaceAll()`: Iterates through the active `CodeEditor`'s document, replacing all occurrences of `findInput` text with `replaceInput` text.
  - Show a small `QMessageBox` indicating how many replacements were made.

## Task 3: Find in Files
**Objective**: Search for text across multiple files in a specified directory.
- **UI Updates**:
  - Add `QAction* findInFilesAction` to the "Edit" menu (shortcut: `Ctrl+Shift+F`).
  - Create a new dialog `FindInFilesDialog` or add it to a dock. Let's use a non-modal `QDockWidget` or `QDialog`. A `QDialog` is simpler for now:
    - Inputs: Directory path (with a "Browse..." button), Extension filter (`QLineEdit`, e.g., `*.cpp *.h *.txt`), Search term (`QLineEdit`).
    - Output: `QListWidget` for results.
    - Buttons: "Search".
- **Logic**:
  - Use `QDirIterator` to recursively search the specified directory, applying the name filters.
  - For each file, read line by line. If a line contains the search term, add an item to the results list.
  - Store the file path and line number in the `QListWidgetItem`'s `Qt::UserRole` data.
  - Connect double-click on the results list to a slot that calls `openFile(filePath)` (requires modifying `openFile` to accept a path programmatically and jump to the line).

## Task 5: Windows Subsystem Configuration
**Objective**: Ensure the compiled Windows executable does not open a background console (CMD) window when launched.
- **CMake Updates**:
  - Modify `src/CMakeLists.txt` or the main `CMakeLists.txt` to set the `WIN32` property on the executable.
  - E.g., `add_executable(AlexCode WIN32 main.cpp ...)` OR `set_property(TARGET AlexCode PROPERTY WIN32_EXECUTABLE true)`.