#!/usr/bin/env bash
set -euo pipefail
repo=Jgrasp/squadron
manifest=$(curl -fsSL "https://github.com/$repo/releases/latest/download/latest.json")
pick() { printf '%s\n' "$manifest" | sed -n "s/.*\"url\": *\"\([^\"]*\.$1\)\".*/\1/p" | head -1; }

if [ "$(uname)" = Darwin ]; then
  url=$(pick 'app\.tar\.gz')
  [ -n "$url" ] || { echo "This release has no macOS build yet." >&2; exit 1; }
  tmp=$(mktemp -d)
  echo "Downloading $(basename "$url")"
  curl -fL --progress-bar "$url" -o "$tmp/squadron.tar.gz"
  tar -xzf "$tmp/squadron.tar.gz" -C "$tmp"
  app=$(find "$tmp" -maxdepth 1 -name '*.app' | head -1)
  [ -n "$app" ] || { echo "The download holds no app." >&2; exit 1; }
  rm -rf "/Applications/$(basename "$app")"
  mv "$app" /Applications/
  rm -rf "$tmp"
  echo "Squadron is installed in /Applications. Open it from Launchpad or Spotlight."
  exit 0
fi

if command -v apt >/dev/null && [ -n "$(pick deb)" ]; then
  url=$(pick deb)
  file=$(mktemp --suffix=.deb)
  echo "Downloading $(basename "$url")"
  curl -fL --progress-bar "$url" -o "$file"
  chmod 644 "$file"
  sudo apt install -y "$file"
  rm -f "$file" "$HOME/.local/bin/squadron.AppImage" "$HOME/.local/share/applications/squadron.desktop"
  echo "Squadron is installed. Open it from your applications, or run squadron"
  exit 0
fi

bin="${SQUADRON_DIR:-$HOME/.local/bin}"
apps="$HOME/.local/share/applications"
data="$HOME/.local/share/squadron"
url=$(pick AppImage)
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
