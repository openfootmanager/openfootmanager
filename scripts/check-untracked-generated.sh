#!/usr/bin/env bash
#
# Assert that files generated at build time are not tracked by git.
#
# src/dev/pageHarness/fixture.generated.ts is written by `npm run fixture:pages` in CI's frontend
# job and by scripts/ensure-page-fixture.mjs locally. When it was committed, most PRs that touched
# a page or the world regenerated it, and two of them either conflicted or merged into a copy no
# fresh generation produced, which dropped them from the merge queue. .gitignore keeps it out of
# an ordinary `git add`, but `git add -f`, or an .gitignore edit, would bring it back silently, and
# the merge queue would start dropping PRs again. This check makes that a red build instead.
#
# Usage: check-untracked-generated.sh [repo_dir] [path ...]
# The arguments exist so the fixtures can drive this against a scratch repository.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo="${1:-$repo_root}"
shift || true
if [ "$#" -eq 0 ]; then
    set -- src/dev/pageHarness/fixture.generated.ts
fi

status=0
for path in "$@"; do
    # `ls-files --error-unmatch` exits 0 for a tracked path and 1 for an untracked one. Anything
    # else is git failing (not a repository, an unreadable index), and must not read as "untracked",
    # or the check would pass on exactly the checkout it cannot inspect.
    # Git's own message is kept for that case only: on exit 1 it is the pathspec error every
    # untracked pass produces.
    rc=0
    git_error="$(git -C "$repo" ls-files --error-unmatch -- "$path" 2>&1 >/dev/null)" || rc=$?
    case "$rc" in
        0)
            echo "check-untracked-generated: $path is tracked by git. It is generated at build time" >&2
            echo "  (npm run fixture:pages); remove it from the index with: git rm --cached $path" >&2
            status=1
            ;;
        1) ;;
        *)
            echo "check-untracked-generated: git could not inspect $repo (exit $rc):" >&2
            printf '  %s\n' "$git_error" >&2
            status=1
            ;;
    esac
done
exit "$status"
