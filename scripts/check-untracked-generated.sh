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
    if git -C "$repo" ls-files --error-unmatch -- "$path" >/dev/null 2>&1; then
        echo "check-untracked-generated: $path is tracked by git. It is generated at build time" >&2
        echo "  (npm run fixture:pages); remove it from the index with: git rm --cached $path" >&2
        status=1
    fi
done
exit "$status"
