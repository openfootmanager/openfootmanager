#!/usr/bin/env bash
#
# Mutates the Rust production lines a branch changes, and prints the result for the PR body.
# The /preflight mutants step (a trial, 13 to 27 Oct 2026; see .claude/skills/preflight/SKILL.md).
#
# Usage: scripts/mutants-in-diff.sh [base]      base defaults to upstream/develop
#
#   CARGO                  the cargo command (default cargo). On a shared machine, wrap the whole run in
#                          one lock instead of passing a build-slot wrapper here: a run lasts hours.
#   MUTANTS_IN_DIFF_PATCH  a patch to use instead of `git diff base...HEAD` (relative to src-tauri/)
#   MUTANTS_TIMEOUT        cap on the whole run, as for timeout(1); default 180m
#
# Exits 0 with the result (survivors are the result, not a failure), and non-zero when the run
# could not produce one. Every way of "passing" without testing anything exits non-zero:
#
#   * A patch rooted at the repository (`src-tauri/crates/...`) matches nothing in the workspace,
#     which lives in src-tauri/. cargo-mutants then prints "No mutants to filter" and exits 0.
#   * A run that writes no report, or a report of zero mutants (scripts/mutation-outcome.sh).
#
# A diff that changes no Rust production line (tests, docs, the frontend) lists zero mutants and
# says so; that is a real result, and nothing is mutated.

set -uo pipefail

base="${1:-upstream/develop}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
read -ra cargo <<< "${CARGO:-cargo}"

root="$(git rev-parse --show-toplevel)" || exit 1
cd "$root" || exit 1

# --in-place edits src-tauri/ one mutant at a time; a tree with uncommitted work would mix the
# two, and the restore below would throw that work away.
if [ -n "$(git status --porcelain -- src-tauri)" ]; then
    echo "mutants-in-diff: commit or stash your changes under src-tauri/ first;" >&2
    echo "the run edits those files in place while it works." >&2
    exit 1
fi

out="$(realpath -m "$(git rev-parse --git-path mutants-in-diff)")"
rm -rf "$out" && mkdir -p "$out" || exit 1
patch="$out/diff.patch"
if [ -n "${MUTANTS_IN_DIFF_PATCH:-}" ]; then
    cp "$MUTANTS_IN_DIFF_PATCH" "$patch" || exit 1
else
    git diff --relative=src-tauri "$base...HEAD" -- src-tauri > "$patch" || exit 1
fi

while read -r path; do
    if [ ! -e "src-tauri/$path" ]; then
        echo "mutants-in-diff: the patch names '$path', but there is no src-tauri/$path." >&2
        echo "A patch must be relative to src-tauri/ (git diff --relative=src-tauri); one rooted" >&2
        echo "at the repository matches nothing, and cargo-mutants would test nothing and exit 0." >&2
        exit 1
    fi
done < <(sed -n 's#^+++ b/##p' "$patch")

listed="$(cd src-tauri && "${cargo[@]}" mutants --workspace --in-diff "$patch" --list)" || exit 1
count="$(printf '%s' "$listed" | grep -c .)"
if [ "$count" -eq 0 ]; then
    echo "Mutants on the diff: 0 mutants (the branch changes no Rust production line)"
    exit 0
fi
echo "mutants-in-diff: $count mutants on the diff; about a minute each on a busy machine." >&2

started=$SECONDS
(cd src-tauri && CARGO_INCREMENTAL=1 timeout --signal=INT "${MUTANTS_TIMEOUT:-180m}" \
    "${cargo[@]}" mutants --workspace --in-diff "$patch" --in-place --profile mutants \
    --timeout 300 --output "$out")
status=$?
minutes=$(( (SECONDS - started + 59) / 60 ))

if [ -n "$(git status --porcelain -- src-tauri)" ]; then
    git checkout -- src-tauri
    echo "mutants-in-diff: the run left mutated files behind; restored them from HEAD." >&2
fi

"$here/mutation-outcome.sh" "$status" "$out/mutants.out/outcomes.json" || exit $?

lines_in() { grep -c . "$out/mutants.out/$1.txt" 2>/dev/null || true; }
echo "Mutants on the diff: $(lines_in caught) caught, $(lines_in missed) missed," \
    "$(lines_in timeout) timed out, $(lines_in unviable) unviable ($minutes min)"
for kind in missed timeout; do
    sed "s/^/- [$kind] /" "$out/mutants.out/$kind.txt" 2>/dev/null
done
