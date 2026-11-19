#!/usr/bin/env bash
set -euo pipefail
repo=Jgrasp/squadron
cd "$(dirname "$0")/.."
root=$PWD
. "$HOME/.cargo/env"

tag_release() {
  local version=$1 merged crate
  merged=$(gh pr list -R "$repo" --head "release/v$version" --state merged --json mergeCommit --jq '.[0].mergeCommit.oid // empty')
  [ -n "$merged" ] || { echo "The pull request of Squadron $version is not merged." >&2; return 1; }
  git fetch -q "git@github.com:$repo.git" main
  crate=$(git show "$merged:src-tauri/Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)
  [ "$crate" = "$version" ] || { echo "The merged commit $merged is version $crate, not $version." >&2; return 1; }
  git tag "v$version" "$merged"
  git push -q "git@github.com:$repo.git" "refs/tags/v$version"
  echo "Tagged v$version on $merged: the Release workflow publishes it."
}

watch_release() {
  local version=$1 run line name state conclusion started now since seen
  local queue_max=${QUEUE_MAX:-300} run_max=${RUN_MAX:-900}
  declare -A last first
  while :; do
    run=$(gh run list -R "$repo" --workflow Release --limit 20 --json databaseId,headBranch,event,status \
      --jq "[.[] | select(.headBranch == \"v$version\" and .event == \"push\")][0] | \"\(.databaseId) \(.status)\"")
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
  tag_release "${2:?usage: scripts/release.sh tag <version>}"
  exit
fi

if [ "${1:-}" = watch ]; then
  watch_release "${2:?usage: scripts/release.sh watch <version>}"
  exit
fi

version=${1:?usage: scripts/release.sh <version> [notes] [fr=notes ...], scripts/release.sh tag <version> once the pull request is merged, or scripts/release.sh watch <version>}
notes=${2:-}
branch=release/v$version
work=$root/.claude/worktrees/release-v$version
if git ls-remote --exit-code --tags "git@github.com:$repo.git" "refs/tags/v$version" >/dev/null; then
  echo "v$version is already released." >&2
  exit 1
fi
git fetch -q "git@github.com:$repo.git" main
git worktree add -q --detach "$work" FETCH_HEAD
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
git commit -qm "Squadron $version" ${notes:+-m "$notes"} -a
git push -q "git@github.com:$repo.git" "HEAD:refs/heads/$branch"
url=$(gh pr create -R "$repo" --base main --head "$branch" --title "Squadron $version" --body "${notes:-Squadron $version}")
echo "Squadron $version waits in $url. Once it is merged, scripts/release.sh tag $version publishes it."
