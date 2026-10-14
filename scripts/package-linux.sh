#!/usr/bin/env bash
# 建立 Linux AppImage（CI 與本機共用）。
# 用法：scripts/package-linux.sh <build-dir> <version> [out-dir]
# 需求：已建置的 <build-dir>/src/AlexCode；QMAKE 指向與建置相同的 Qt（未設定則取 PATH 中的 qmake）。
set -euo pipefail
BUILD=$(cd "$1" && pwd); VER=$2; OUT=$(mkdir -p "${3:-dist}" && cd "${3:-dist}" && pwd)
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TOOLS=${LINUXDEPLOY_DIR:-$BUILD/linuxdeploy}
mkdir -p "$TOOLS"
for t in linuxdeploy/linuxdeploy linuxdeploy/linuxdeploy-plugin-qt; do
    f="$TOOLS/$(basename $t)-x86_64.AppImage"
    [ -x "$f" ] || { curl -fsSL -o "$f" "https://github.com/$t/releases/download/continuous/$(basename $t)-x86_64.AppImage"; chmod +x "$f"; }
done

APPDIR="$BUILD/AppDir"
rm -rf "$APPDIR"
install -Dm755 "$BUILD/src/AlexCode" "$APPDIR/usr/bin/AlexCode"
install -Dm644 "$ROOT/packaging/linux/alexcode.desktop" "$APPDIR/usr/share/applications/alexcode.desktop"
install -Dm644 "$ROOT/src/icon.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/alexcode.png"

export APPIMAGE_EXTRACT_AND_RUN=1                       # CI 環境無 FUSE
export QMAKE=${QMAKE:-$(command -v qmake6 || command -v qmake)}
export EXTRA_PLATFORM_PLUGINS="libqoffscreen.so"        # 讓無螢幕環境（CI 煙霧測試、自動截圖）也能執行
export LDAI_OUTPUT="$OUT/AlexCode-Linux-x86_64-$VER.AppImage" OUTPUT="$OUT/AlexCode-Linux-x86_64-$VER.AppImage"
"$TOOLS/linuxdeploy-x86_64.AppImage" --appdir "$APPDIR" --plugin qt --output appimage
echo "AppImage: $LDAI_OUTPUT"
