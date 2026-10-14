#!/usr/bin/env bash
# 建立 macOS DMG（CI 與本機共用）。
# 用法：scripts/package-macos.sh <build-dir> <version> [out-dir]
# 需求：已建置的 <build-dir>/src/AlexCode.app；PATH 中有與建置相同 Qt 的 macdeployqt。
set -euo pipefail
BUILD=$(cd "$1" && pwd); VER=$2; OUT=$(mkdir -p "${3:-dist}" && cd "${3:-dist}" && pwd)
STAGE="$BUILD/dmg-stage"
rm -rf "$STAGE" && mkdir -p "$STAGE"
cp -R "$BUILD/src/AlexCode.app" "$STAGE/"
# 把 Qt frameworks / 外掛放進 bundle 並改寫連結路徑
macdeployqt "$STAGE/AlexCode.app" -always-overwrite
# macdeployqt 改寫了執行檔與 framework，原簽章失效；Apple Silicon 上未簽章的程式碼無法執行 → ad-hoc 重簽
codesign --force --deep --sign - "$STAGE/AlexCode.app"
codesign --verify --deep --strict "$STAGE/AlexCode.app"
ln -s /Applications "$STAGE/Applications"            # 拖曳安裝
DMG="$OUT/AlexCode-macOS-$VER.dmg"
hdiutil create -volname "AlexCode $VER" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
echo "DMG: $DMG"
