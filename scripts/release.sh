#!/usr/bin/env bash
set -euo pipefail
repo=Jgrasp/squadron
cd "$(dirname "$0")/.."
root=$PWD
. "$HOME/.cargo/env"

tag_merged() {
  git fetch -q "git@github.com:$repo.git" main
  base=$(git rev-parse FETCH_HEAD)
  local subject tag
  subject=$(git log -1 --format=%s "$base")
  case $subject in "Squadron "[0-9]*) ;; *) return 0 ;; esac
  tag=v${subject#Squadron }
  git update-ref refs/heads/public "$base"
  if ! git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    git tag "$tag" "$base"
    echo "Tagged $tag on the merged $subject."
  fi
}

if [ "${1:-}" = tag ]; then
  tag_merged
  exit
fi

version=${1:?usage: scripts/release.sh <version> [notes] [fr=notes ...], or scripts/release.sh tag once the pull request is merged}
notes=${2:-}
branch=release/v$version
work=$root/.worktrees/release-v$version
tag_merged
git worktree add -q --detach "$work" HEAD
trap 'git -C "$root" worktree remove --force "$work"' EXIT
cd "$work"

copy=$root/../squadron-extensions/api
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
    [ -n "$translated" ] || { echo "Missing $lang=<notes> for $file." >&2; exit 1; }
    prepend "$file" "$translated"
  done
  prepend CHANGELOG.md "$notes"
fi
git commit -qam "Squadron $version"
release=$(git rev-parse HEAD)

public=$(git commit-tree "HEAD^{tree}" -p "$base" -m "Squadron $version" ${notes:+-m "$notes"})
git push -q "git@github.com:$repo.git" "$public:refs/heads/$branch"
url=$(gh pr create -R "$repo" --base main --head "$branch" --title "Squadron $version" --body "${notes:-Squadron $version}")

cd "$root"
if ! git merge -q --ff-only "$release" 2>/dev/null && ! git cherry-pick "$release" >/dev/null; then
  echo "Squadron $version is $release: bring it onto main by hand." >&2
fi
echo "Squadron $version waits in $url: merging it (rebase) starts the release. Then run scripts/release.sh tag."
