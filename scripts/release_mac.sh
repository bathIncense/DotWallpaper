#!/bin/bash
# macOS 发布流程：构建 .app -> strip 主二进制 -> ad-hoc 重签名 -> hdiutil 打 DMG -> 输出体积报告
# 说明：
# - cargo profile.release 刻意 strip=false（新版 macOS strip 会损坏 proc-macro dylib，rust-lang/rust#157750），
#   因此这里对最终二进制单独 strip -x，再重新 ad-hoc 签名
# - tauri 自带 bundle_dmg.sh 依赖 Finder 自动化，在部分环境不可用，这里用 hdiutil 直出 UDZO DMG
set -euo pipefail
cd "$(dirname "$0")/.."

APP=src-tauri/target/aarch64-apple-darwin/release/bundle/macos/DotWallpaper.app
BIN=$APP/Contents/MacOS/dotwallpaper
DMG=src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/DotWallpaper_0.1.2_aarch64.dmg

npm run build:mac:app

strip -x "$BIN"
rm -f "$BIN.strip"
codesign --force -s - "$BIN"
codesign --force --deep -s - "$APP"

STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
mkdir -p "$(dirname "$DMG")"
rm -f "$DMG"
hdiutil create -volname DotWallpaper -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"

echo
echo "== size report =="
ls -l "$BIN" | awk '{printf "executable (stripped): %s B\n", $5}'
echo "frontend dist:         $(du -sk ui/dist | cut -f1) KB"
du -sh "$APP" | awk '{printf ".app total:            %s\n", $1}'
ls -l "$DMG" | awk '{printf "dmg (UDZO):            %s B\n", $5}'
