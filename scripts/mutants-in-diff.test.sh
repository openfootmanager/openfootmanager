#!/usr/bin/env bash
#
# Drives `mutants-in-diff.sh` (the /preflight mutants step) against a throwaway repository and a
# fake `cargo`, so every way the step can pass without testing anything is seen failing here.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/mutants-in-diff.sh"

passed=0
failed=0

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# A fake cargo-mutants. `--list` prints FAKE_LIST lines; a run writes a report of FAKE_TOTAL
# mutants with FAKE_MISSED of them missed (unless FAKE_NO_REPORT is set), optionally leaves a
# source file mutated (FAKE_LEAVE_MUTATED), and exits FAKE_STATUS. Every call is logged.
fake_cargo="$scratch/fake-cargo"
cat > "$fake_cargo" <<'FAKE'
#!/usr/bin/env bash
echo "$*" >> "$FAKE_LOG"
case " $* " in
  *" --list "*)
    for ((i = 0; i < ${FAKE_LIST:-0}; i++)); do echo "crates/core/src/lib.rs:$((i + 1)):1: replace x"; done
    exit 0 ;;
esac
out=""
while [ $# -gt 0 ]; do [ "$1" = "--output" ] && out="$2"; shift; done
if [ -z "${FAKE_NO_REPORT:-}" ]; then
  mkdir -p "$out/mutants.out"
  printf '{"outcomes": [], "total_mutants": %s}\n' "${FAKE_TOTAL:-0}" > "$out/mutants.out/outcomes.json"
  : > "$out/mutants.out/caught.txt"; : > "$out/mutants.out/missed.txt"
  : > "$out/mutants.out/timeout.txt"; : > "$out/mutants.out/unviable.txt"
  for ((i = 0; i < ${FAKE_MISSED:-0}; i++)); do
    echo "crates/core/src/lib.rs:1:1: replace answer -> u32 with 0" >> "$out/mutants.out/missed.txt"
  done
  for ((i = ${FAKE_MISSED:-0}; i < ${FAKE_TOTAL:-0}; i++)); do
    echo "crates/core/src/lib.rs:2:1: caught one" >> "$out/mutants.out/caught.txt"
  done
fi
[ -n "${FAKE_LEAVE_MUTATED:-}" ] && echo "// mutated" >> crates/core/src/lib.rs
exit "${FAKE_STATUS:-0}"
FAKE
chmod +x "$fake_cargo"

# A repository with a `src-tauri/` workspace and one commit on top of `base` that changes Rust.
new_repo() {
    local repo="$scratch/repo-$RANDOM$RANDOM"
    mkdir -p "$repo/src-tauri/crates/core/src"
    git -C "$repo" init -q
    git -C "$repo" config user.email test@example.invalid
    git -C "$repo" config user.name test
    echo "pub fn answer() -> u32 { 41 }" > "$repo/src-tauri/crates/core/src/lib.rs"
    git -C "$repo" add -A && git -C "$repo" commit -qm base && git -C "$repo" branch -q base
    echo "pub fn answer() -> u32 { 42 }" > "$repo/src-tauri/crates/core/src/lib.rs"
    git -C "$repo" commit -qam change
    echo "$repo"
}

# expect <expected exit> <description> <output must contain> -- <env assignments...>
expect() {
    local want="$1" description="$2" needle="$3"
    shift 4
    local repo output got
    repo="$(new_repo)"
    : > "$scratch/cargo.log"
    output="$(cd "$repo" && env FAKE_LOG="$scratch/cargo.log" CARGO="$fake_cargo" "$@" \
        "$script" base 2>&1)"
    got=$?
    if [ "$got" -eq "$want" ] && [[ "$output" == *"$needle"* ]]; then
        passed=$((passed + 1))
        printf '  ok    %s\n' "$description"
    else
        failed=$((failed + 1))
        printf '  FAIL  %s\n' "$description"
        printf '        expected exit %s with "%s", got %s. Output was:\n' "$want" "$needle" "$got"
        printf '%s\n' "$output" | sed 's/^/        /'
    fi
    LAST_REPO="$repo"
}

ran_list() { grep -q -- "--list" "$scratch/cargo.log"; }
ran_mutants() { grep -qv -- "--list" "$scratch/cargo.log"; }

echo "mutants-in-diff.test: what the step reports"
expect 0 "survivors are the result, listed with the counts" "1 missed" -- \
    FAKE_LIST=3 FAKE_TOTAL=3 FAKE_MISSED=1 FAKE_STATUS=2
expect 0 "a diff with no Rust production line reports zero and mutates nothing" \
    "0 mutants" -- FAKE_LIST=0
if ran_list && ! ran_mutants; then
    passed=$((passed + 1)); echo "  ok    a zero listing is asked for, and starts no mutation run"
else
    failed=$((failed + 1)); echo "  FAIL  a zero listing was not asked for, or still started a mutation run"
fi

echo "mutants-in-diff.test: ways to pass without testing anything"
root_patch="$scratch/root.patch"
printf 'diff --git a/src-tauri/crates/core/src/lib.rs b/src-tauri/crates/core/src/lib.rs\n--- a/src-tauri/crates/core/src/lib.rs\n+++ b/src-tauri/crates/core/src/lib.rs\n@@ -1 +1 @@\n-x\n+y\n' > "$root_patch"
expect 1 "a patch rooted at the repository, which matches nothing, is refused" \
    "no src-tauri/src-tauri" -- FAKE_LIST=0 MUTANTS_IN_DIFF_PATCH="$root_patch"
expect 1 "a run that wrote no report fails" "wrote no report" -- \
    FAKE_LIST=2 FAKE_NO_REPORT=1 FAKE_STATUS=0
expect 1 "a run that tested no mutant fails" "tested no mutants" -- \
    FAKE_LIST=2 FAKE_TOTAL=0 FAKE_STATUS=0

echo "mutants-in-diff.test: the working tree"
dirty_repo="$(new_repo)"
echo "// uncommitted" >> "$dirty_repo/src-tauri/crates/core/src/lib.rs"
: > "$scratch/cargo.log"
dirty_output="$(cd "$dirty_repo" && env FAKE_LOG="$scratch/cargo.log" CARGO="$fake_cargo" FAKE_LIST=1 "$script" base 2>&1)"
dirty_status=$?
if [ "$dirty_status" -eq 1 ] && [ ! -s "$scratch/cargo.log" ] && [[ "$dirty_output" == *"commit"* ]]; then
    passed=$((passed + 1)); echo "  ok    uncommitted changes in src-tauri are refused before anything runs"
else
    failed=$((failed + 1)); echo "  FAIL  uncommitted changes were not refused (exit $dirty_status): $dirty_output"
fi
expect 124 "an interrupted run that left a mutant behind is reported" "restored" -- \
    FAKE_LIST=1 FAKE_TOTAL=1 FAKE_LEAVE_MUTATED=1 FAKE_STATUS=124
if ran_mutants && git -C "$LAST_REPO" diff --quiet; then
    passed=$((passed + 1)); echo "  ok    and the mutated file is restored"
else
    failed=$((failed + 1)); echo "  FAIL  the mutated file was left in the tree"
fi

echo ""
if [ "$failed" -ne 0 ]; then
    echo "mutants-in-diff.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "mutants-in-diff.test: all $passed cases behaved as expected"
