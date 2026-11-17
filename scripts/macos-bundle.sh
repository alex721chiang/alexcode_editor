#!/usr/bin/env bash
# 在 macOS 上把 AlexCode 包成 universal（arm64 + x86_64）的 AlexCode.app，並壓成 zip。
#
# 用法：bash scripts/macos-bundle.sh [輸出資料夾]   （預設 dist）
# 需要：rustup 已安裝 aarch64-apple-darwin 與 x86_64-apple-darwin 兩個 target；
#       macOS 內建的 lipo、sips、iconutil、plutil、ditto。
# 產生：<輸出資料夾>/AlexCode.app 與 <輸出資料夾>/AlexCode-Rust-macos-universal.zip
set -euo pipefail

OUT="${1:-dist}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [ -z "$VERSION" ]; then
  echo "cannot read the version from Cargo.toml" >&2
  exit 1
fi

# 兩種架構各建一次，再用 lipo 合成一個執行檔
for TARGET in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build --release --target "$TARGET"
done

APP="$OUT/AlexCode.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/alexcode" \
  target/aarch64-apple-darwin/release/alexcode \
  target/x86_64-apple-darwin/release/alexcode

ARCHS="$(lipo -archs "$APP/Contents/MacOS/alexcode")"
echo "architectures: $ARCHS"
case "$ARCHS" in
  *x86_64*arm64* | *arm64*x86_64*) ;;
  *)
    echo "the bundled executable is not universal: $ARCHS" >&2
    exit 1
    ;;
esac

# 圖示：從 assets/icon.png（256 × 256）縮放出 iconset，再轉成 icns。
# 512 / 1024 像素的尺寸是放大的，會比較模糊（原圖只有 256 像素）
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ICONSET="$WORK/AppIcon.iconset"
mkdir -p "$ICONSET"
for SIZE in 16 32 128 256 512; do
  sips -z "$SIZE" "$SIZE" assets/icon.png --out "$ICONSET/icon_${SIZE}x${SIZE}.png" >/dev/null
  DOUBLE=$((SIZE * 2))
  sips -z "$DOUBLE" "$DOUBLE" assets/icon.png --out "$ICONSET/icon_${SIZE}x${SIZE}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
test -s "$APP/Contents/Resources/AppIcon.icns"

# Info.plist：程式名稱、版本、圖示，以及可以用 AlexCode 開啟的檔案類型（「打開檔案的應用程式」清單裡會出現 AlexCode）
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>zh_TW</string>
  <key>CFBundleDisplayName</key>
  <string>AlexCode</string>
  <key>CFBundleExecutable</key>
  <string>alexcode</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>io.github.alex721chiang.alexcode</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>AlexCode</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key>
      <string>Text and source code</string>
      <key>CFBundleTypeRole</key>
      <string>Editor</string>
      <key>LSHandlerRank</key>
      <string>Alternate</string>
      <key>LSItemContentTypes</key>
      <array>
        <string>public.plain-text</string>
        <string>public.source-code</string>
        <string>public.script</string>
        <string>public.json</string>
        <string>public.xml</string>
        <string>public.yaml</string>
        <string>net.daringfireball.markdown</string>
        <string>com.apple.log</string>
      </array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key>
      <string>Other code files</string>
      <key>CFBundleTypeRole</key>
      <string>Editor</string>
      <key>LSHandlerRank</key>
      <string>Alternate</string>
      <key>CFBundleTypeExtensions</key>
      <array>
        <string>rs</string>
        <string>toml</string>
        <string>md</string>
        <string>log</string>
        <string>txt</string>
        <string>ini</string>
        <string>cfg</string>
        <string>conf</string>
      </array>
    </dict>
  </array>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist"

rm -f "$OUT/AlexCode-Rust-macos-universal.zip"
(cd "$OUT" && ditto -c -k --keepParent AlexCode.app AlexCode-Rust-macos-universal.zip)
ls -l "$OUT/AlexCode-Rust-macos-universal.zip"
