#!/usr/bin/env bash
set -euo pipefail
version=${1:?usage: scripts/release.sh <version> [notes]}
notes=${2:-}
repo=Jgrasp/squadron
cd "$(dirname "$0")/.."
. "$HOME/.cargo/env"
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "Commit or stash your changes first." >&2; exit 1; }

sed -i "0,/^version = .*/s//version = \"$version\"/" src-tauri/Cargo.toml
(cd src-tauri && cargo update -p squadron --offline -q)
git diff --quiet || git commit -qam "Squadron $version"

parent=$(git rev-parse -q --verify refs/heads/public || true)
public=$(git commit-tree "HEAD^{tree}" ${parent:+-p "$parent"} -m "Squadron $version" ${notes:+-m "$notes"})
git update-ref refs/heads/public "$public"
git tag "v$version" "$public"
git push -q "git@github.com:$repo.git" public:main "v$version"
echo "v$version pushed: GitHub builds the AppImage, https://github.com/$repo/actions"
