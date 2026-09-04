#!/usr/bin/env bash
# NetManager.app paketini üretir (menü çubuğu uygulaması, Dock'ta görünmez).
set -euo pipefail
cd "$(dirname "$0")/.."

APP="target/NetManager.app"
ICONSET="target/AppIcon.iconset"

cargo build --release --bin net_manager --bin gen_icon

# --- İkon ---
rm -rf "$ICONSET"; mkdir -p "$ICONSET"
./target/release/gen_icon target/icon.png >/dev/null
for sz in 16 32 128 256 512; do
  sips -z $sz $sz target/icon.png --out "$ICONSET/icon_${sz}x${sz}.png" >/dev/null
  sips -z $((sz*2)) $((sz*2)) target/icon.png --out "$ICONSET/icon_${sz}x${sz}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o target/AppIcon.icns

# --- Paket ---
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/net_manager "$APP/Contents/MacOS/NetManager"
cp target/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>            <string>Net Manager</string>
  <key>CFBundleDisplayName</key>     <string>Net Manager</string>
  <key>CFBundleIdentifier</key>      <string>com.atilla.netmanager</string>
  <key>CFBundleVersion</key>         <string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleExecutable</key>      <string>NetManager</string>
  <key>CFBundleIconFile</key>        <string>AppIcon</string>
  <key>CFBundlePackageType</key>     <string>APPL</string>
  <key>LSMinimumSystemVersion</key>  <string>11.0</string>
  <key>LSUIElement</key>             <true/>
</dict>
</plist>
PLIST

codesign --force --deep -s - "$APP" 2>/dev/null || true
touch "$APP"
echo "Hazır: $APP"

# --install: /Applications'a kur ve yeniden başlat
if [[ "${1:-}" == "--install" ]]; then
  pkill -f "NetManager.app/Contents/MacOS/NetManager" 2>/dev/null || true
  sleep 1
  rm -rf /Applications/NetManager.app
  cp -R "$APP" /Applications/
  open /Applications/NetManager.app
  echo "Kuruldu ve başlatıldı: /Applications/NetManager.app"
fi
