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

watch_release() {
  local version=$1 run line name state conclusion started now since seen
  local queue_max=${QUEUE_MAX:-300} run_max=${RUN_MAX:-900}
  declare -A last first
  while :; do
    run=$(gh run list -R "$repo" --workflow Release --limit 20 --json databaseId,displayTitle,event,status \
      --jq "[.[] | select(.displayTitle == \"Squadron $version\")][0] | \"\(.databaseId) \(.status)\"")
    [ -n "$run" ] && [ "$run" != "null null" ] || { echo "No Release run for Squadron $version." >&2; return 1; }
    now=$(date +%s)
    while IFS='|' read -r name state conclusion started; do
      [ -n "$name" ] || continue
      [ "$conclusion" = - ] && conclusion=
      if [ "${last[$name]:-}" != "$state $conclusion" ]; then
        if [ -z "${last[$name]:-}" ] && [ -n "$started" ]; then first[$name]=$(date -d "$started" +%s); else first[$name]=$now; fi
        last[$name]="$state $conclusion"
        echo "$(date +%T) $name: $state${conclusion:+ ($conclusion)}"
      fi
      case $conclusion in failure | cancelled | timed_out) echo "Squadron $version: $name ended $conclusion. https://github.com/$repo/actions/runs/${run%% *}" >&2; return 1 ;; esac
      since=$((now - first[$name]))
      if [ "$state" = queued ] && [ "$since" -gt "$queue_max" ]; then
        echo "Squadron $version: $name has waited for a runner for $((since / 60)) min." >&2
        return 2
      fi
      if [ "$state" = in_progress ] && [ "$since" -gt "$run_max" ]; then
        echo "Squadron $version: $name has run for $((since / 60)) min." >&2
        return 2
      fi
    done < <(gh run view -R "$repo" "${run%% *}" --json jobs --jq '.jobs[] | [.name, .status, (if (.conclusion // "") == "" then "-" else .conclusion end), .startedAt] | join("|")')
    case $run in
      *" completed")
        seen=$(gh run view -R "$repo" "${run%% *}" --json conclusion --jq .conclusion)
        echo "Squadron $version: Release run $seen. https://github.com/$repo/actions/runs/${run%% *}"
        [ "$seen" = success ] && return 0 || return 1
        ;;
      *)
        if [ -z "${last[*]}" ]; then
          seen=${first[__run]:-$now}
          first[__run]=$seen
          [ $((now - seen)) -gt "$queue_max" ] && { echo "Squadron $version: the Release run has waited $(((now - seen) / 60)) min to start." >&2; return 2; }
        fi
        ;;
    esac
    sleep 30
  done
}

if [ "${1:-}" = tag ]; then
  tag_merged
  exit
fi

if [ "${1:-}" = watch ]; then
  watch_release "${2:?usage: scripts/release.sh watch <version>}"
  exit
fi

version=${1:?usage: scripts/release.sh <version> [notes] [fr=notes ...], scripts/release.sh tag once the pull request is merged, or scripts/release.sh watch <version>}
notes=${2:-}
branch=release/v$version
work=$root/.claude/worktrees/release-v$version
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
