#!/usr/bin/env bash
#
# Mutates the Rust production lines a branch changes, and prints the result for the PR body.
# The /preflight mutants step (see .claude/skills/preflight/SKILL.md; kept after its 13-27 Oct
# 2026 trial, with the glue list, the size cap and the survivor re-check below).
#
# Usage: scripts/mutants-in-diff.sh [base]      base defaults to upstream/develop
#        scripts/mutants-in-diff.sh --check-exclusions
#                          check that every glue entry still names a function, and run nothing
#
#   CARGO                  the cargo command (default cargo). On a shared machine, wrap the whole run in
#                          one lock instead of passing a build-slot wrapper here: a run lasts hours.
#   MUTANTS_IN_DIFF_PATCH  a patch to use instead of `git diff base...HEAD` (relative to src-tauri/)
#   MUTANTS_IN_DIFF_ONLY   a file of mutant names (a previous run's missed list): mutate only those,
#                          wherever their lines have moved. For re-checking survivors after a fix.
#   MUTANTS_IN_DIFF_MAX    the most mutants run on this machine (default 60); over it, the step says
#                          so and runs nothing, for the diff to be sharded in the cloud
#   MUTANTS_IN_DIFF_EXCLUSIONS  the glue list (default scripts/mutants-in-diff.exclusions)
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

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
read -ra cargo <<< "${CARGO:-cargo}"

root="$(git rev-parse --show-toplevel)" || exit 1
cd "$root" || exit 1

# The characters the regex crate (cargo-mutants' `--re`) treats as metacharacters, escaped, the
# same set as `regex::escape`. `<` and `>` stay bare: `\<` and `\>` are word boundaries there.
regex_escape() {
    sed -e 's/[][\.+*?(){}|^$#&~-]/\\&/g'
}

# Every mutant of one function: its whole body replaced, or an operator inside it.
function_pattern() {
    local path fn
    path="$(regex_escape <<< "$1")" fn="$2"
    printf '^%s:[0-9]+:[0-9]+: (replace %s( ->| with )|.* in %s$)' "$path" "$fn" "$fn"
}

# Fills `excluded` with one `--exclude-re` per glue function, and fails, naming the entry, when an
# entry no longer names a function in its file.
excluded=()
collect_exclusions() {
    local list="${MUTANTS_IN_DIFF_EXCLUSIONS:-$here/mutants-in-diff.exclusions}"
    local path fn _why file name
    [ -f "$list" ] || { echo "mutants-in-diff: no glue list at $list" >&2; return 1; }
    while read -r path fn _why; do
        case "$path" in ''|'#'*) continue ;; esac
        if ! [[ "$fn" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || [ ! -f "src-tauri/$path" ] ||
            ! grep -Eq "fn $fn[<(]" "src-tauri/$path"; then
            echo "mutants-in-diff: the glue entry '$path $fn' no longer names a function;" >&2
            echo "update $list rather than let the exclusion lapse." >&2
            return 1
        fi
        excluded+=(--exclude-re "$(function_pattern "$path" "$fn")")
    done < "$list"
    # A Tauri command that has an `_internal` beside it only forwards to it: the command takes a
    # `tauri::State` that no test can build, and the `_internal` is what is tested and mutated.
    for file in src-tauri/src/commands/*.rs; do
        [ -f "$file" ] || continue
        while read -r name; do
            grep -Eq "fn ${name}_internal[<(]" "$file" &&
                excluded+=(--exclude-re "$(function_pattern "${file#src-tauri/}" "$name")")
        done < <(awk '/#\[tauri::command\]/ { command = 1; next }
                     command && match($0, /fn [A-Za-z_][A-Za-z0-9_]*/) {
                         print substr($0, RSTART + 3, RLENGTH - 3); command = 0 }' "$file")
    done
    # The last command without an `_internal` leaves a failing status; that is not a failure.
    return 0
}

if [ "${1:-}" = "--check-exclusions" ]; then
    collect_exclusions || exit 1
    echo "mutants-in-diff: the glue list holds; $(( ${#excluded[@]} / 2 )) functions are excluded"
    exit 0
fi

base="${1:-upstream/develop}"

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

collect_exclusions || exit 1

# A re-check after fixes: only the survivors named, each anchored to its file and mutation but not
# its line, since a fix above it moves it.
only=()
if [ -n "${MUTANTS_IN_DIFF_ONLY:-}" ]; then
    while IFS= read -r name; do
        [[ "$name" =~ ^([^:]+):[0-9]+:[0-9]+:\ (.+)$ ]] || continue
        only+=(--re "^$(regex_escape <<< "${BASH_REMATCH[1]}"):[0-9]+:[0-9]+: $(regex_escape <<< "${BASH_REMATCH[2]}")\$")
    done < "$MUTANTS_IN_DIFF_ONLY"
    if [ "${#only[@]}" -eq 0 ]; then
        echo "mutants-in-diff: $MUTANTS_IN_DIFF_ONLY names no mutant to re-check." >&2
        exit 1
    fi
fi

listed="$(cd src-tauri && "${cargo[@]}" mutants --workspace --in-diff "$patch" \
    "${excluded[@]}" "${only[@]}" --list)" || exit 1
count="$(printf '%s' "$listed" | grep -c .)"
if [ "$count" -eq 0 ]; then
    echo "Mutants on the diff: 0 mutants (the branch changes no Rust production line)"
    exit 0
fi
# Measured in the trial: a 130-mutant diff never finished on the shared machine (two memory kills
# in three attempts); three cloud shards did it in about 35 minutes each. A re-check is exempt.
max="${MUTANTS_IN_DIFF_MAX:-60}"
if [ "${#only[@]}" -eq 0 ] && [ "$count" -gt "$max" ]; then
    echo "Mutants on the diff: not run locally: $count mutants is over the local cap of $max;" \
        "run it sharded in the cloud (cargo mutants --shard k/n), or note it here as not run"
    exit 0
fi
echo "mutants-in-diff: $count mutants on the diff; about a minute each on a busy machine." >&2

started=$SECONDS
(cd src-tauri && CARGO_INCREMENTAL=1 timeout --signal=INT "${MUTANTS_TIMEOUT:-180m}" \
    "${cargo[@]}" mutants --workspace --in-diff "$patch" "${excluded[@]}" "${only[@]}" \
    --in-place --profile mutants --timeout 300 --output "$out")
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
