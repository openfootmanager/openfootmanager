#!/usr/bin/env bash
#
# Plans the weekly mutation-testing rotation, and says which slot of it a given job should run.
#
# A slot is one crate, or one `--shard` of a crate, sized to finish inside the job's time limit.
# The shard count for each crate is worked out from how many mutants it has *today* and what one
# of its mutants costs, rather than written down once. A fixed split is what failed on
# 4 Oct 2026: `ofm_core` had been cut into 13 shards on the strength of an 18-second mutant,
# measured on `db`, and one of its mutants actually costs nearer two minutes. The slot needed
# about twenty hours, and the step timed out after 141 of its 643 mutants.
#
# Usage:
#   mutation-slot.sh rotation <crate>=<mutants>...
#       Print every slot in the rotation, one "<crate> <shard>/<shards>" per line.
#   mutation-slot.sh pick <week> <job> <jobs-per-week> <crate>=<mutants>...
#       Print the slot that job number <job> of week <week> runs. A week's jobs take
#       consecutive slots, the next week carries on after them, and the end wraps to the start.
#   mutation-slot.sh first <crate> <crate>=<mutants>...
#       Print the first slot of one crate: what a manual run that names no shard gets.
#
# Every crate in the rotation needs a count. Shards are zero-based, as `cargo mutants --shard`
# expects them.

set -euo pipefail

# What one mutant costs on a GitHub-hosted runner, build and test together, in seconds. These
# are the numbers to revisit when a run overruns — each one says where it came from.
#
#   domain    no measurement of its own yet. A leaf crate with nothing depending on it inside
#             the package under test, so it is assumed no dearer than `db`.
#   db        17.8 s, measured: 495 mutants in 2h 27m on 27 Sep 2026.
#   engine    no CI measurement yet. Locally a mutant builds in ~2 s and tests in ~2.4 s, so
#             15 s leaves the runner plenty of room for being slower than a workstation.
#   ofm_core  34.2 s, measured: 140 mutants on 6 Oct 2026 (16.4 s to build, 17.7 s to test,
#             one 300 s timeout included), with the `mutants` profile and incremental builds.
#             Before those two changes it was 115 s (4 Oct 2026). 40 s leaves room for a slot
#             that draws more timeouts than that sample did.
declare -A SECONDS_PER_MUTANT=(
    [domain]=20
    [db]=20
    [engine]=15
    [ofm_core]=40
)

# The order slots run in. Fixed, so the rotation is the same whatever order the counts arrive in.
CRATES=(domain db engine ofm_core)

# Mutant-seconds one slot may plan for. The mutation step's limit is 270 minutes (16,200 s); the
# unmutated baseline build and test come out of that first, and the per-mutant costs above are
# averages, so a quarter of the limit is left unplanned.
SLOT_BUDGET_SECONDS=12000

die() {
    echo "mutation-slot: $*" >&2
    exit 1
}

is_count() {
    [[ "$1" =~ ^[0-9]{1,9}$ ]]
}

declare -A mutants=()

# Reads "<crate>=<mutants>" arguments into `mutants`, refusing anything that could plan a
# rotation which silently skips a crate.
read_counts() {
    local pair crate count
    for pair in "$@"; do
        crate="${pair%%=*}"
        count="${pair#*=}"
        [ -n "${SECONDS_PER_MUTANT[$crate]+set}" ] || die "no per-mutant cost for crate '$crate'"
        is_count "$count" || die "'$count' is not a mutant count for $crate"
        # A listing that failed prints nothing, and nothing counts as zero. Planning on that
        # would drop the crate from the rotation without a word.
        [ "$((10#$count))" -gt 0 ] || die "$crate has no mutants, which means its listing failed"
        mutants[$crate]="$((10#$count))"
    done
    for crate in "${CRATES[@]}"; do
        [ -n "${mutants[$crate]+set}" ] || die "no mutant count for $crate"
    done
}

shards_for() {
    local crate="$1"
    local seconds=$((mutants[$crate] * SECONDS_PER_MUTANT[$crate]))
    echo $(((seconds + SLOT_BUDGET_SECONDS - 1) / SLOT_BUDGET_SECONDS))
}

rotation() {
    local crate shards shard
    for crate in "${CRATES[@]}"; do
        shards="$(shards_for "$crate")"
        for ((shard = 0; shard < shards; shard++)); do
            echo "$crate $shard/$shards"
        done
    done
}

[ "$#" -ge 1 ] || die "usage: mutation-slot.sh rotation|pick|first ..."
command="$1"
shift

case "$command" in
    rotation)
        read_counts "$@"
        rotation
        ;;

    pick)
        [ "$#" -ge 3 ] || die "usage: mutation-slot.sh pick <week> <job> <jobs-per-week> <crate>=<mutants>..."
        week="$1" job="$2" jobs="$3"
        shift 3
        is_count "$week" || die "'$week' is not a week number"
        is_count "$job" || die "'$job' is not a job number"
        is_count "$jobs" || die "'$jobs' is not a number of jobs"
        week=$((10#$week)) job=$((10#$job)) jobs=$((10#$jobs))
        [ "$jobs" -gt 0 ] || die "a week needs at least one job"
        [ "$job" -lt "$jobs" ] || die "job $job does not exist in a week of $jobs jobs"

        read_counts "$@"
        mapfile -t slots < <(rotation)
        # More jobs than slots would hand two jobs of the same week the same slot.
        [ "$jobs" -le "${#slots[@]}" ] ||
            die "$jobs jobs a week, but the rotation only has ${#slots[@]} slots"
        echo "${slots[$(((week * jobs + job) % ${#slots[@]}))]}"
        ;;

    first)
        [ "$#" -ge 1 ] || die "usage: mutation-slot.sh first <crate> <crate>=<mutants>..."
        crate="$1"
        shift
        [ -n "${SECONDS_PER_MUTANT[$crate]+set}" ] || die "'$crate' is not in the rotation"
        read_counts "$@"
        echo "$crate 0/$(shards_for "$crate")"
        ;;

    *)
        die "unknown command '$command'; expected rotation, pick or first"
        ;;
esac
