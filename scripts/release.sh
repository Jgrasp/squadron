#!/usr/bin/env bash
set -euo pipefail
version=${1:?usage: scripts/release.sh <version> [notes] [fr=notes ...]}
notes=${2:-}
repo=Jgrasp/squadron
cd "$(dirname "$0")/.."
. "$HOME/.cargo/env"
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "Commit or stash your changes first." >&2; exit 1; }

sed -i "0,/^version = .*/s//version = \"$version\"/" src-tauri/Cargo.toml
(cd src-tauri && cargo update -p squadron --offline -q)
prepend() {
  { head -n 1 "$1"; echo; echo "## $version · $(date +%F)"; echo; echo "$2" | sed 's/^## /### /'; tail -n +2 "$1"; } > "$1.new"
  mv "$1.new" "$1"
}
if [ -n "$notes" ]; then
  for file in CHANGELOG.*.md; do
    lang=${file#CHANGELOG.}; lang=${lang%.md}
    translated=$(for arg in "${@:3}"; do [ "${arg%%=*}" = "$lang" ] && echo "${arg#*=}"; done)
    [ -n "$translated" ] || { git checkout -q src-tauri; echo "Missing $lang=<notes> for $file." >&2; exit 1; }
    prepend "$file" "$translated"
  done
  prepend CHANGELOG.md "$notes"
fi
git diff --quiet || git commit -qam "Squadron $version"

parent=$(git rev-parse -q --verify refs/heads/public || true)
public=$(git commit-tree "HEAD^{tree}" ${parent:+-p "$parent"} -m "Squadron $version" ${notes:+-m "$notes"})
git update-ref refs/heads/public "$public"
git tag "v$version" "$public"
git push -q "git@github.com:$repo.git" public:main "v$version"
echo "v$version pushed: GitHub builds the release, https://github.com/$repo/actions"
