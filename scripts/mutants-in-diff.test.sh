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

# A fake cargo-mutants. `--list` prints FAKE_LIST lines, each mutation FAKE_LIST_NAME; a run writes a report of FAKE_TOTAL
# mutants with FAKE_MISSED of them missed (unless FAKE_NO_REPORT is set), optionally leaves a
# source file mutated (FAKE_LEAVE_MUTATED), and exits FAKE_STATUS. Every call is logged.
fake_cargo="$scratch/fake-cargo"
cat > "$fake_cargo" <<'FAKE'
#!/usr/bin/env bash
echo "$*" >> "$FAKE_LOG"
case " $* " in
  *" --list "*)
    for ((i = 0; i < ${FAKE_LIST:-0}; i++)); do echo "crates/core/src/lib.rs:$((i + 1)):1: ${FAKE_LIST_NAME:-replace x}"; done
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
    [ -n "${PREPARE:-}" ] && "$PREPARE" "$repo"
    output="$(cd "$repo" && env FAKE_LOG="$scratch/cargo.log" CARGO="$fake_cargo" \
        MUTANTS_IN_DIFF_EXCLUSIONS="$no_exclusions" "$@" "$script" base 2>&1)"
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

# Cases that are not about the glue list get an empty one: the real list names this repository's
# files, which a throwaway repository does not have.
no_exclusions="$scratch/no-exclusions"
: > "$no_exclusions"

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

echo "mutants-in-diff.test: glue that no unit test can observe"
# A glue entry: a function in a file, which every cargo-mutants call must exclude.
glue="$scratch/glue"
printf '%s\n' '# comment lines and blank lines are ignored' '' \
    'crates/core/src/lib.rs answer  the fake crate entry point' > "$glue"
expect 0 "a glue function is excluded from the listing and from the run" "caught" -- \
    FAKE_LIST=2 FAKE_TOTAL=2 FAKE_STATUS=0 MUTANTS_IN_DIFF_EXCLUSIONS="$glue"
if [ "$(grep -c -F -- '--exclude-re ^crates/core/src/lib\.rs:[0-9]+:[0-9]+: (replace (.*::)?answer( ->| with )|.* in (.*::)?answer$)' "$scratch/cargo.log")" -eq 2 ]; then
    passed=$((passed + 1)); echo "  ok    and both the listing and the run carry its --exclude-re"
else
    failed=$((failed + 1)); echo "  FAIL  the glue pattern is missing from a call: $(cat "$scratch/cargo.log")"
fi

stale="$scratch/stale-glue"
echo 'crates/core/src/lib.rs renamed_away  a function that was renamed' > "$stale"
expect 1 "a glue entry that no longer names a function is refused before anything runs" \
    "no longer" -- FAKE_LIST=2 MUTANTS_IN_DIFF_EXCLUSIONS="$stale"
if [ ! -s "$scratch/cargo.log" ]; then
    passed=$((passed + 1)); echo "  ok    and cargo-mutants is never called"
else
    failed=$((failed + 1)); echo "  FAIL  cargo-mutants ran despite a stale entry: $(cat "$scratch/cargo.log")"
fi

# Tauri commands that only forward to their tested `_internal` (directly, after a log line, or off
# the main thread), one with an `_internal` beside it that does work of its own first, and one with
# no `_internal` at all.
add_commands() {
    mkdir -p "$1/src-tauri/src/commands"
    cat > "$1/src-tauri/src/commands/club.rs" <<'RUST'
#[tauri::command]
pub async fn rename_club(state: State<'_, Arc<StateManager>>, name: String) -> Result<(), String> {
    rename_club_internal(&state, &name)
}

pub fn rename_club_internal(state: &StateManager, name: &str) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn relocate_club(
    state: State<'_, Arc<StateManager>>,
    city: String,
) -> Result<(), String> {
    info!("[cmd] relocate_club: {}", city);
    log::debug!("[cmd] relocate_club; from the settings screen");
    relocate_club_internal(&state, &city)
}

pub fn relocate_club_internal(state: &StateManager, city: &str) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn rebuild_club(state: State<'_, Arc<StateManager>>) -> Result<(), String> {
    let state = state.inner().clone();
    run_off_thread(move || rebuild_club_internal(&state)).await
}

pub fn rebuild_club_internal(state: &StateManager) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn promote_club(
    state: State<'_, Arc<StateManager>>,
    name: String,
) -> Result<(), String> {
    let name = name.trim().to_string();
    promote_club_internal(&state, &name)
}

pub fn promote_club_internal(state: &StateManager, name: &str) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn club_badge(state: State<'_, Arc<StateManager>>, name: Option<String>) -> Result<(), String> {
    club_badge_internal(&state, name.as_deref())
}

pub fn club_badge_internal(state: &StateManager, name: Option<&str>) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn club_rank(state: State<'_, Arc<StateManager>>, limit: Option<u32>) -> Result<(), String> {
    club_rank_internal(&state, limit.unwrap_or(50))
}

pub fn club_rank_internal(state: &StateManager, limit: u32) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn club_size(state: State<'_, Arc<StateManager>>) -> Result<usize, String> {
    club_size_internal(&state).map(|players| players.len() + 1)
}

pub fn club_size_internal(state: &StateManager) -> Result<Vec<u32>, String> {
    Ok(vec![])
}

#[tauri::command]
pub fn club_count(state: State<'_, Arc<StateManager>>) -> usize {
    3
}
RUST
    git -C "$1" add -A && git -C "$1" commit -qm commands
}
PREPARE=add_commands expect 0 "a command that only forwards to its _internal is excluded" "caught" -- \
    FAKE_LIST=2 FAKE_TOTAL=2 FAKE_STATUS=0
if grep -q -F -- 'src/commands/club\.rs:[0-9]+:[0-9]+: (replace (.*::)?rename_club( ->| with )|.* in (.*::)?rename_club$)' "$scratch/cargo.log" &&
    grep -q -F -- '(replace (.*::)?relocate_club( ->| with )|.* in (.*::)?relocate_club$)' "$scratch/cargo.log" &&
    grep -q -F -- '(replace (.*::)?rebuild_club( ->| with )|.* in (.*::)?rebuild_club$)' "$scratch/cargo.log" &&
    grep -q -F -- '(replace (.*::)?club_badge( ->| with )|.* in (.*::)?club_badge$)' "$scratch/cargo.log" &&
    ! grep -q -- 'club_rank' "$scratch/cargo.log" &&
    ! grep -q -- 'club_count' "$scratch/cargo.log" &&
    ! grep -q -- 'promote_club' "$scratch/cargo.log" &&
    ! grep -q -- 'club_size' "$scratch/cargo.log" &&
    ! grep -q -- 'in rename_club_internal' "$scratch/cargo.log"; then
    passed=$((passed + 1)); echo "  ok    and a command with logic of its own, even beside an _internal, and the _internal itself, are not"
else
    failed=$((failed + 1)); echo "  FAIL  wrong command exclusions: $(cat "$scratch/cargo.log")"
fi

# What one glue entry's pattern catches, tried on names in cargo-mutants' own shapes: a free
# function, a method, a trait method and a function in an inline module, each with its whole body
# replaced and with an operator inside it; and nothing of a function that only shares the name.
source <(sed -n -e '/^regex_escape() {/,/^}/p' -e '/^function_pattern() {/,/^}/p' "$script")
glue_pattern="$(function_pattern src/club.rs answer)"
caught_names=(
    'src/club.rs:3:5: replace answer -> u32 with 0'
    'src/club.rs:3:9: replace + with - in answer'
    'src/club.rs:12:9: replace Club::answer -> u32 with 1'
    'src/club.rs:12:13: replace == with != in Club::answer'
    'src/club.rs:20:9: replace <impl Display for Club>::answer -> String with String::new()'
    'src/club.rs:31:9: replace * with / in inner::answer'
)
spared_names=(
    'src/club.rs:40:5: replace answer_internal -> u32 with 0'
    'src/club.rs:41:9: replace + with - in Club::other_answer'
    'src/other.rs:3:5: replace answer -> u32 with 0'
)
missed_glue="" spared_wrongly=""
for name in "${caught_names[@]}"; do
    grep -Eq -- "$glue_pattern" <<< "$name" || missed_glue+=" [$name]"
done
for name in "${spared_names[@]}"; do
    grep -Eq -- "$glue_pattern" <<< "$name" && spared_wrongly+=" [$name]"
done
if [ -z "$missed_glue$spared_wrongly" ]; then
    passed=$((passed + 1)); echo "  ok    a glue entry catches its function as a method or in a module, and nothing else"
else
    failed=$((failed + 1))
    echo "  FAIL  the glue pattern missed:$missed_glue; and wrongly caught:$spared_wrongly"
fi

real_check="$(cd "$here/.." && "$script" --check-exclusions 2>&1)"
if [ $? -eq 0 ]; then
    passed=$((passed + 1)); echo "  ok    every entry of the real glue list still names a function in this repository"
else
    failed=$((failed + 1)); echo "  FAIL  the real glue list is stale: $real_check"
fi

echo "mutants-in-diff.test: a diff too big to run here"
expect 0 "over 60 mutants is not run locally, and says so" "not run locally" -- \
    FAKE_LIST=61 FAKE_TOTAL=61
if ! ran_mutants; then
    passed=$((passed + 1)); echo "  ok    and no mutation run starts"
else
    failed=$((failed + 1)); echo "  FAIL  a mutation run started over the cap"
fi
expect 0 "60 mutants still run" "60 caught" -- FAKE_LIST=60 FAKE_TOTAL=60 FAKE_STATUS=0
expect 1 "a cap that is not a whole number is refused" "MUTANTS_IN_DIFF_MAX" -- \
    FAKE_LIST=61 FAKE_TOTAL=61 MUTANTS_IN_DIFF_MAX=60m
if ! ran_list && ! ran_mutants; then
    passed=$((passed + 1)); echo "  ok    and cargo-mutants is never called"
else
    failed=$((failed + 1)); echo "  FAIL  cargo-mutants ran with a cap of 60m: $(cat "$scratch/cargo.log")"
fi
expect 0 "a cloud shard runs its part of the diff" "40 caught" -- \
    FAKE_LIST=40 FAKE_TOTAL=40 FAKE_STATUS=0 MUTANTS_IN_DIFF_SHARD=1/3
if [ "$(grep -c -- '--in-diff .* --shard 1/3' "$scratch/cargo.log")" -eq 2 ]; then
    passed=$((passed + 1)); echo "  ok    and both the listing and the run keep the diff and take the shard"
else
    failed=$((failed + 1)); echo "  FAIL  the shard lost the diff or was not passed on: $(cat "$scratch/cargo.log")"
fi
expect 1 "a shard that is not k/n with k below n is refused" "MUTANTS_IN_DIFF_SHARD" -- \
    FAKE_LIST=1 MUTANTS_IN_DIFF_SHARD=3/3
expect 0 "a cloud shard over the local cap still runs: the cap is for this machine" "65 caught" -- \
    FAKE_LIST=65 FAKE_TOTAL=65 FAKE_STATUS=0 MUTANTS_IN_DIFF_SHARD=0/2

echo "mutants-in-diff.test: re-checking only the survivors"
survivors="$scratch/survivors"
echo 'crates/core/src/lib.rs:1:1: replace answer -> u32 with 0' > "$survivors"
expect 0 "a re-check runs only the named survivors, wherever their line has moved" "caught" -- \
    FAKE_LIST=1 FAKE_LIST_NAME="replace answer -> u32 with 0" FAKE_TOTAL=1 FAKE_STATUS=0 \
    MUTANTS_IN_DIFF_ONLY="$survivors"
if grep -q -F -- '--re ^crates/core/src/lib\.rs:[0-9]+:[0-9]+: replace answer \-> u32 with 0$' "$scratch/cargo.log"; then
    passed=$((passed + 1)); echo "  ok    and each survivor is one anchored --re"
else
    failed=$((failed + 1)); echo "  FAIL  no anchored --re for the survivor: $(cat "$scratch/cargo.log")"
fi
gone="$scratch/survivors-one-gone"
{ cat "$survivors"; echo 'crates/core/src/lib.rs:9:1: replace answer -> u32 with 1'; } > "$gone"
expect 1 "a survivor that matches no mutant on the diff now is named, not silently dropped" \
    "replace answer -> u32 with 1" -- \
    FAKE_LIST=1 FAKE_LIST_NAME="replace answer -> u32 with 0" MUTANTS_IN_DIFF_ONLY="$gone"
if ran_list && ! ran_mutants; then
    passed=$((passed + 1)); echo "  ok    and no mutation run starts"
else
    failed=$((failed + 1)); echo "  FAIL  a mutation run started with a survivor unaccounted for"
fi
all_gone="$scratch/survivors-all-gone"
echo 'crates/core/src/lib.rs:9:1: replace answer -> u32 with 1' > "$all_gone"
expect 1 "a re-check whose survivors are all gone is refused, not reported as zero mutants" \
    "replace answer -> u32 with 1" -- FAKE_LIST=0 MUTANTS_IN_DIFF_ONLY="$all_gone"
: > "$scratch/no-survivors"
expect 1 "a re-check with no survivors named is refused" "names no mutant" -- \
    FAKE_LIST=1 MUTANTS_IN_DIFF_ONLY="$scratch/no-survivors"

echo ""
if [ "$failed" -ne 0 ]; then
    echo "mutants-in-diff.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "mutants-in-diff.test: all $passed cases behaved as expected"
