# AlexCode Editor 🐲

A high-performance, cross-platform code editor built with **C++17** and **Qt6**, inspired by Notepad++. Designed for speed, clarity, and developer productivity.

---

## ✨ Features

### Core Editing
- **Tabbed Interface** — Open and manage multiple files simultaneously
- **Syntax Highlighting** — Full support for C++ and Python
- **Line Numbers** — Always-visible gutter for easy navigation
- **Font Configuration** — Customize font family and size to your preference

### Search & Filter
- **Find & Replace** — Powerful in-file search with replace support
- **Multi-Keyword Filtering** — Filter lines using `|` separator with **AND** / **OR** logic
- **Recursive Cross-File Search** — Search across all files in a directory tree via the Find in Files dialog
- **100k-Line Performance** — Extreme optimization for large files without lag

### Platform & Build
- **Windows Cross-Compilation** — Built via **GitHub Actions CI/CD** on every push
- **Custom Dragon Icon** — RGBA transparency `.ico` for crisp rendering on Windows
- **Automated NAS Deployment** — Release artifacts automatically deployed to home NAS

---

## 🏗️ Architecture

```
alexcode_editor/
├── src/
│   ├── main.cpp                  # Entry point
│   ├── MainWindow.cpp/h          # Main application window, tab management
│   ├── CodeEditor.cpp/h          # Core editor widget with line numbers
│   ├── SyntaxHighlighter.cpp/h   # C++ & Python syntax highlighting
│   ├── FilterEngine.cpp/h        # AND/OR multi-keyword filter logic
│   ├── FindInFilesDialog.cpp/h   # Recursive cross-file search dialog
│   ├── resources.qrc             # Qt resource bundle (icons)
│   └── alexcode.ico              # Windows RGBA app icon
├── tests/                        # TDD unit tests
├── .github/workflows/
│   └── windows.yml               # CI/CD: cross-compile & release for Windows
└── CMakeLists.txt                # CMake build configuration
```

---

## 🛠️ Build Requirements

| Dependency | Version |
|------------|---------|
| C++ Standard | C++17 |
| Qt | 6.x (Core, Gui, Widgets) |
| CMake | ≥ 3.16 |

---

## 🚀 Build Instructions

### Local (macOS / Linux)

```bash
git clone https://github.com/alex721chiang/alexcode_editor.git
cd alexcode_editor
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release
```

### Windows (via GitHub Actions)

Every push to `main` triggers the CI/CD pipeline in `.github/workflows/windows.yml`, which:
1. Cross-compiles the project for Windows (x64)
2. Bundles Qt platform plugins
3. Uploads the Release artifact

---

## 🧪 Testing

Tests are located in the `tests/` directory and follow **Test-Driven Development (TDD)** principles.

```bash
cmake --build build
cd build && ctest
```

---

## 📋 Development Methodology

This project was built using the [Superpowers](https://github.com/obra/superpowers) agentic development framework, following:

- **Design-first** — Architecture and spec reviewed before any code is written
- **TDD** — RED → GREEN → REFACTOR cycle enforced throughout
- **Subagent-driven development** — Complex features broken into independently verifiable tasks

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.
