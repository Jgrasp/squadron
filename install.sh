#!/usr/bin/env bash
set -euo pipefail
repo=Jgrasp/squadron
bin="${SQUADRON_DIR:-$HOME/.local/bin}"
apps="$HOME/.local/share/applications"
data="$HOME/.local/share/squadron"

url=$(curl -fsSL "https://github.com/$repo/releases/latest/download/latest.json" | sed -n 's/.*"url": *"\([^"]*\.AppImage\)".*/\1/p' | head -1)
[ -n "$url" ] || { echo "No Squadron release found." >&2; exit 1; }

mkdir -p "$bin" "$apps" "$data"
echo "Downloading $(basename "$url")"
curl -fL --progress-bar "$url" -o "$bin/squadron.AppImage"
chmod +x "$bin/squadron.AppImage"
curl -fsSL "https://raw.githubusercontent.com/$repo/main/src-tauri/icons/128x128.png" -o "$data/icon.png"
cat > "$apps/squadron.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Squadron
Comment=Drive several Claude Code agents side by side
Exec=$bin/squadron.AppImage
Icon=$data/icon.png
Terminal=false
Categories=Development;
StartupWMClass=squadron
DESKTOP

echo "Squadron is installed. Open it from your applications, or run $bin/squadron.AppImage"
