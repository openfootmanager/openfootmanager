#!/usr/bin/env bash
#
# Plans the weekly mutation-testing rotation, says which slot of it a given job should run, and
# turns a slot into the `--re` filters that make cargo-mutants test exactly that slot's mutants.
#
# A slot is one *bucket* of one crate. Every mutant belongs to exactly one bucket of its crate,
# chosen by hashing its name with the line and column taken out, so editing other code never
# moves it. The number of buckets per crate is a constant below, not something worked out from
# the day's counts. Both halves replace a rotation of `--shard` slots that never added up:
#
#   * The shard count was recomputed every run from the live mutant counts, so the week -> slot
#     mapping jumped whenever a count crossed a shard boundary. `ofm_core` went from 29 to 32
#     shards between 11 and 18 Oct 2026, and the rotation went from shards 19-24 back to 13-18.
#   * A shard is a position in cargo-mutants' list (25.3.1 deals mutants round-robin by index),
#     so one added or removed mutant reshuffled every shard after it. Consecutive shards across
#     weeks were never consecutive sets of mutants: some were drawn twice and some never.
#
# With fixed buckets, a cycle of the rotation tests every mutant once.
#
# Usage:
#   mutation-slot.sh rotation
#       Print every slot in the rotation, one "<crate> <bucket>/<buckets>" per line.
#   mutation-slot.sh pick <week> <job> <jobs-per-week>
#       Print the slot that job number <job> of week <week> runs. A week's jobs take
#       consecutive slots, the next week carries on after them, and the end wraps to the start.
#   mutation-slot.sh first <crate>
#       Print the first slot of one crate: what a manual run that names no bucket gets.
#   mutation-slot.sh select <crate> <bucket>/<buckets> < listing
#       Read `cargo mutants --list` output for <crate> and print one anchored regex per mutant in
#       that bucket, to pass to cargo-mutants as `--re`. Refuses a bucket that would select
#       nothing, and one too big to finish inside a slot.
#
# Buckets are zero-based.

set -euo pipefail

# What one mutant costs on a GitHub-hosted runner, build and test together, in seconds. These
# are the numbers to revisit when a run overruns — each one says where it came from.
#
#   domain    no measurement of its own yet. A leaf crate with nothing depending on it inside
#             the package under test, so it is assumed no dearer than `db`.
#   db        17.8 s, measured: 495 mutants in 2h 27m on 27 Sep 2026.
#   engine    no CI measurement yet. Locally a mutant builds in ~2 s and tests in ~2.4 s, but a
#             workstation figure is exactly what went wrong for `ofm_core`, so it is planned at
#             the same 20 s as its neighbours until a slot measures it.
#   ofm_core  34.2 s, measured: 140 mutants on 6 Oct 2026 (16.4 s to build, 17.7 s to test,
#             one 300 s timeout included), with the `mutants` profile and incremental builds.
#             Before those two changes it was 115 s (4 Oct 2026). 40 s leaves room for a slot
#             that draws more timeouts than that sample did.
declare -A SECONDS_PER_MUTANT=(
    [domain]=20
    [db]=20
    [engine]=20
    [ofm_core]=40
)

# How many buckets each crate is cut into. Chosen so the largest bucket of each crate, at the
# counts on develop on 21 Oct 2026, plans for no more than about 80% of the slot budget below,
# which leaves room to grow before `select` starts refusing:
#
#   crate      mutants  buckets  largest bucket  planned
#   domain         420        1             420   8,400 s  (70%)
#   db             553        2             285   5,700 s  (48%; one bucket would be 92%)
#   engine       1,648        4             425   8,500 s  (71%)
#   ofm_core     9,524       44             238   9,520 s  (79%)
#
# 51 slots, so eight jobs a week cover every mutant once in a little under seven Sundays.
#
# When `select` refuses a bucket as over budget, raise that crate's number here and move
# ROTATION_START_WEEK to the week of the change: a new bucket count is a new set of buckets, and
# the cycle starts again from the first slot.
declare -A BUCKETS=(
    [domain]=1
    [db]=2
    [engine]=4
    [ofm_core]=44
)

# The order slots run in. Fixed, so the rotation is the same whatever happens to the code.
CRATES=(domain db engine ofm_core)

# Weeks are counted from the Unix epoch. Week 2964 holds Sunday 25 Oct 2026, the first scheduled
# run of these buckets, so the cycle starts at the first slot there.
ROTATION_START_WEEK=2964

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

in_rotation() {
    [ -n "${BUCKETS[$1]+set}" ]
}

rotation() {
    local crate bucket
    for crate in "${CRATES[@]}"; do
        for ((bucket = 0; bucket < BUCKETS[$crate]; bucket++)); do
            echo "$crate $bucket/${BUCKETS[$crate]}"
        done
    done
}

# Reads a `cargo mutants --list` listing on stdin and prints the anchored, escaped name of every
# mutant in bucket <bucket> of <buckets>, one per line.
#
# A mutant's bucket comes from its name without the line and column ("<path>: <mutation>"), plus
# how many earlier mutants in the listing share that name. The same mutation often appears at
# several lines of one function, and without the count they would all land in one bucket, which
# made `ofm_core`'s largest bucket a quarter bigger. Lines moving, or mutants elsewhere coming
# and going, change neither part; only a repeat of the same mutation earlier in the same
# function shifts the count of the ones after it.
#
# The hash is a polynomial one over the name's bytes, modulo the prime 2^31 - 1, written for
# any POSIX awk: the runners' default awk is mawk, which has no bitwise operators. It only has
# to spread names evenly and give the same answer every week.
#
# cargo-mutants matches `--re` against the name *with* line and column, so each pattern is the
# exact current name. Every character the regex crate treats as a metacharacter is escaped, the
# same set as `regex::escape`; `<` and `>` are left alone, because `\<` and `\>` are word
# boundaries there, not literals.
bucket_names() {
    LC_ALL=C awk -v bucket="$1" -v buckets="$2" '
        BEGIN {
            for (i = 1; i < 256; i++) ord[sprintf("%c", i)] = i
            meta = "\\.+*?()|[]{}^$#&-~"
        }
        {
            if (!match($0, /:[0-9]+:[0-9]+: /) || RSTART == 1) {
                printf "line %d is not a mutant name: %s\n", NR, $0 > "/dev/stderr"
                bad = 1
                exit 1
            }
            key = substr($0, 1, RSTART - 1) ": " substr($0, RSTART + RLENGTH)
            key = key "#" (seen[key]++)
            hash = 0
            for (i = 1; i <= length(key); i++) {
                hash = (hash * 131 + ord[substr(key, i, 1)]) % 2147483647
            }
            if (hash % buckets != bucket) next
            pattern = "^"
            for (i = 1; i <= length($0); i++) {
                c = substr($0, i, 1)
                pattern = pattern (index(meta, c) ? "\\" c : c)
            }
            print pattern "$"
        }
        END { if (bad) exit 1 }
    '
}

[ "$#" -ge 1 ] || die "usage: mutation-slot.sh rotation|pick|first|select ..."
command="$1"
shift

case "$command" in
    rotation)
        [ "$#" -eq 0 ] || die "the rotation is fixed; it takes no arguments"
        rotation
        ;;

    pick)
        [ "$#" -eq 3 ] || die "usage: mutation-slot.sh pick <week> <job> <jobs-per-week>"
        week="$1" job="$2" jobs="$3"
        is_count "$week" || die "'$week' is not a week number"
        is_count "$job" || die "'$job' is not a job number"
        is_count "$jobs" || die "'$jobs' is not a number of jobs"
        week=$((10#$week)) job=$((10#$job)) jobs=$((10#$jobs))
        [ "$job" -lt "$jobs" ] || die "job $job does not exist in a week of $jobs jobs"
        [ "$week" -ge "$ROTATION_START_WEEK" ] ||
            die "week $week is before the rotation starts, in week $ROTATION_START_WEEK"

        mapfile -t slots < <(rotation)
        # More jobs than slots would hand two jobs of the same week the same slot.
        [ "$jobs" -le "${#slots[@]}" ] ||
            die "$jobs jobs a week, but the rotation only has ${#slots[@]} slots"
        echo "${slots[$((((week - ROTATION_START_WEEK) * jobs + job) % ${#slots[@]}))]}"
        ;;

    first)
        [ "$#" -eq 1 ] || die "usage: mutation-slot.sh first <crate>"
        in_rotation "$1" || die "'$1' is not in the rotation"
        echo "$1 0/${BUCKETS[$1]}"
        ;;

    select)
        [ "$#" -eq 2 ] || die "usage: mutation-slot.sh select <crate> <bucket>/<buckets> < listing"
        crate="$1" slot="$2"
        in_rotation "$crate" || die "'$crate' is not in the rotation"
        [[ "$slot" =~ ^([0-9]{1,9})/([0-9]{1,9})$ ]] || die "'$slot' is not <bucket>/<buckets>"
        bucket=$((10#${BASH_REMATCH[1]})) buckets=$((10#${BASH_REMATCH[2]}))
        # A dispatch that names a stale bucket count would select a set of mutants that is no
        # bucket of this rotation.
        [ "$buckets" -eq "${BUCKETS[$crate]}" ] ||
            die "$crate has ${BUCKETS[$crate]} buckets, not $buckets"
        [ "$bucket" -lt "$buckets" ] || die "bucket $bucket does not exist in $crate's $buckets"

        listing="$(cat)"
        # A listing that failed prints nothing. Selecting from nothing would find an empty bucket,
        # and the job would go green having never looked at the crate.
        [ -n "$listing" ] || die "the listing for $crate is empty, which means it failed"
        patterns="$(bucket_names "$bucket" "$buckets" <<< "$listing")" ||
            die "the listing for $crate is not cargo-mutants' --list output"
        # With no `--re` at all, cargo-mutants mutates the whole crate.
        [ -n "$patterns" ] || die "bucket $slot of $crate selects no mutants"

        count="$(grep -c . <<< "$patterns")"
        planned=$((count * SECONDS_PER_MUTANT[$crate]))
        [ "$planned" -le "$SLOT_BUDGET_SECONDS" ] ||
            die "bucket $slot of $crate has $count mutants, $planned s against a budget of" \
                "$SLOT_BUDGET_SECONDS s; raise BUCKETS[$crate] in this script"
        echo "mutation-slot: bucket $slot of $crate has $count mutants, about $planned s" >&2
        printf '%s\n' "$patterns"
        ;;

    *)
        die "unknown command '$command'; expected rotation, pick, first or select"
        ;;
esac
