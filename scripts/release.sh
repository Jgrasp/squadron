#!/usr/bin/env bash
set -euo pipefail
version=${1:?usage: scripts/release.sh <version> [notes]}
notes=${2:-"Squadron $version"}
repo=Jgrasp/squadron
cd "$(dirname "$0")/.."
export NVM_DIR="$HOME/.nvm"
. "$NVM_DIR/nvm.sh"
. "$HOME/.cargo/env"
[ -z "$(git status --porcelain)" ] || { echo "Commit or stash your changes first." >&2; exit 1; }
key="${SQUADRON_SIGNING_KEY:-$HOME/.tauri/squadron.key}"

sed -i "0,/^version = .*/s//version = \"$version\"/" src-tauri/Cargo.toml
(cd src-tauri && cargo update -p squadron --offline -q)
git diff --quiet || git commit -qam "Squadron $version"

TAURI_SIGNING_PRIVATE_KEY="$(cat "$key")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm tauri build --bundles appimage
dir=src-tauri/target/release/bundle/appimage
appimage=$(ls "$dir"/*_"$version"_*.AppImage)
python3 - "$version" "$notes" "$appimage" "https://github.com/$repo/releases/download/v$version/$(basename "$appimage")" > "$dir/latest.json" <<'PY'
import json, sys, datetime
version, notes, appimage, url = sys.argv[1:5]
print(json.dumps({
    'version': version,
    'notes': notes,
    'pub_date': datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
    'platforms': {'linux-x86_64': {'signature': open(appimage + '.sig').read(), 'url': url}},
}, indent=2))
PY

parent=$(git rev-parse -q --verify refs/heads/public || true)
public=$(git commit-tree "HEAD^{tree}" ${parent:+-p "$parent"} -m "Squadron $version")
git update-ref refs/heads/public "$public"
git push -q "git@github.com:$repo.git" public:main
gh release create "v$version" --repo "$repo" --target main --title "Squadron $version" --notes "$notes" "$appimage" "$appimage.sig" "$dir/latest.json"
