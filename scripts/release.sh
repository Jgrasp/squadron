#!/usr/bin/env bash
set -euo pipefail
version=${1:?usage: scripts/release.sh <version> [notes] [fr=notes ...]}
notes=${2:-}
repo=Jgrasp/squadron
cd "$(dirname "$0")/.."
. "$HOME/.cargo/env"
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "Commit or stash your changes first." >&2; exit 1; }
copy=../squadron-extensions/api
if [ ! -d "$copy" ]; then
  echo "No $copy to compare extensions/api with." >&2
elif ! diff -rq -x target -x Cargo.lock extensions/api "$copy" >/dev/null; then
  echo "extensions/api differs from $copy: copy it there and commit before the release." >&2
  exit 1
fi

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

git fetch -q "git@github.com:$repo.git" main
public=$(git commit-tree "HEAD^{tree}" -p FETCH_HEAD -m "Squadron $version" ${notes:+-m "$notes"})
branch=release/v$version
git push -q "git@github.com:$repo.git" "$public:refs/heads/$branch"
gh pr create -R "$repo" --base main --head "$branch" --title "Squadron $version" --body "${notes:-Squadron $version}" >/dev/null
for try in 1 2 3 4 5; do
  gh pr merge -R "$repo" "$branch" --rebase --delete-branch && break
  [ "$try" = 5 ] && exit 1
  sleep 5
done
git fetch -q "git@github.com:$repo.git" main
git update-ref refs/heads/public FETCH_HEAD
git tag "v$version" FETCH_HEAD
echo "Squadron $version merged into main: GitHub tests it, then builds the release, https://github.com/$repo/actions"
