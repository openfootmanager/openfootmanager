#!/usr/bin/env bash
#
# Assert every commit named in `.git-blame-ignore-revs` still exists.
#
# Why this needs a check at all: the file's whole purpose is to keep `git blame` useful across
# the repo-wide format sweep, and it does that by naming that commit's SHA. A SHA is the one
# thing in this repository that cannot be reviewed by reading it — a rebase, an amend, or a
# squash-merge of the PR that introduced it leaves a hash that is simply wrong, and nothing
# complains. Blame silently goes back to attributing 92 files to the sweep, which is exactly
# what the file was added to prevent, and nobody notices for months.
#
# Needs full history: the commits named here are not the tip, so a shallow checkout cannot
# resolve them. The job that runs this sets `fetch-depth: 0`. Without that this fails on every
# unrelated pull request, which would be a far more expensive mistake than the one it prevents.
#
# Usage: check-blame-ignore-revs.sh [revs_file]
# The argument exists so the fixtures can drive this against deliberately broken files.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
revs_file="${1:-$repo_root/.git-blame-ignore-revs}"

relative() { printf '%s' "${1#"$repo_root"/}"; }

if [ ! -f "$revs_file" ]; then
    echo "check-blame-ignore-revs: $(relative "$revs_file") is missing." >&2
    exit 1
fi

status=0
found=0
lineno=0

while IFS= read -r line || [ -n "$line" ]; do
    lineno=$((lineno + 1))

    # Trim both ends before deciding anything. Order matters, and getting it wrong was this
    # script's first bug: testing for blank *before* trimming means a line of spaces is not
    # "blank", falls through to the SHA check, and fails the build over a file git is perfectly
    # happy with. The rule this script has to obey is that it may be no stricter than git —
    # every shape below was checked against `blame.ignoreRevsFile` itself, and git accepts a
    # whitespace-only line, an indented comment, and a padded SHA without complaint.
    rev="${line#"${line%%[![:space:]]*}"}"
    rev="${rev%"${rev##*[![:space:]]}"}"

    # Comments and blank lines: the file is meant to be read by people as well as by git, and
    # every entry in it should say what it was.
    case "$rev" in
        '' | '#'*) continue ;;
    esac

    found=$((found + 1))

    # Uppercase is accepted for the same no-stricter-than-git reason: `git blame --ignore-rev`
    # and `blame.ignoreRevsFile` both resolve an uppercase object ID, so rejecting one here
    # would fail the build on an entry that works, with a message saying it is the wrong length.
    if ! printf '%s' "$rev" | grep -qE '^[0-9a-fA-F]{40}$'; then
        echo "$(relative "$revs_file"):$lineno: '$rev' is not a full 40-character SHA. git ignores short ones silently." >&2
        status=1
        continue
    fi

    if ! git -C "$repo_root" rev-parse --verify --quiet "$rev^{commit}" >/dev/null; then
        echo "$(relative "$revs_file"):$lineno: $rev is not a commit in this repository." >&2
        status=1
    fi
done < "$revs_file"

if [ "$found" -eq 0 ]; then
    echo "check-blame-ignore-revs: $(relative "$revs_file") names no commits, so it does nothing." >&2
    exit 1
fi

if [ "$status" -ne 0 ]; then
    echo "" >&2
    echo "check-blame-ignore-revs: git blame is silently ignoring nothing for these. If a commit was" >&2
    echo "rebased or squashed, put its new SHA here; if it is gone for good, delete the line." >&2
    exit 1
fi

if [ "$found" -eq 1 ]; then
    echo "check-blame-ignore-revs: the one ignored revision resolves"
else
    echo "check-blame-ignore-revs: all $found ignored revisions resolve"
fi
