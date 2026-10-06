#!/usr/bin/env bash
#
# Drives `mutation-slot.sh`, which decides what each weekly mutation-testing job mutates.
#
# The scheduled job only runs once a week and takes hours, so a slip in this arithmetic costs
# a week to notice: two jobs on one slot, a slot nobody ever reaches, or a slot too big to
# finish in time. That last one is not hypothetical. On 4 Oct 2026 the fixed 13-way split of
# `ofm_core` handed a job 643 mutants that needed about twenty hours, and the step timed out
# after 141 of them. These cases pin the planner so that cannot come back unnoticed.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/mutation-slot.sh"

passed=0
failed=0

record() {
    local ok="$1" description="$2" detail="$3"
    if [ "$ok" -eq 1 ]; then
        passed=$((passed + 1))
        printf '  ok    %s\n' "$description"
    else
        failed=$((failed + 1))
        printf '  FAIL  %s\n' "$description"
        printf '        %s\n' "$detail"
    fi
}

# expect_output <expected stdout> <what this proves> -- <arguments...>
expect_output() {
    local want="$1" description="$2"
    shift 3
    local output status
    output="$("$script" "$@" 2>&1)"
    status=$?
    if [ "$status" -eq 0 ] && [ "$output" = "$want" ]; then
        record 1 "$description" ""
    else
        record 0 "$description" "expected exit 0 and '$want', got exit $status and '$output'"
    fi
}

# expect_rejected <what this proves> -- <arguments...>
expect_rejected() {
    local description="$1"
    shift 2
    local output status
    output="$("$script" "$@" 2>&1)"
    status=$?
    if [ "$status" -ne 0 ]; then
        record 1 "$description" ""
    else
        record 0 "$description" "expected a non-zero exit, got 0 and '$output'"
    fi
}

# The counts on `develop` when this was written. Real numbers, so the size of the real rotation
# is pinned too: if the per-mutant costs in the script change, the case below says by how much.
counts=(domain=367 db=488 engine=1547 ofm_core=8360)

echo "mutation-slot.test: the rotation"

rotation="$("$script" rotation "${counts[@]}")"
slot_total="$(printf '%s\n' "$rotation" | wc -l)"
if [ "$slot_total" -eq 32 ]; then
    record 1 "today's counts make a 32-slot rotation" ""
else
    record 0 "today's counts make a 32-slot rotation" "got $slot_total slots:
$rotation"
fi

first_lines="$(printf '%s\n' "$rotation" | head -n 5)"
want_first='domain 0/1
db 0/1
engine 0/2
engine 1/2
ofm_core 0/28'
if [ "$first_lines" = "$want_first" ]; then
    record 1 "crates come in a fixed order, each shard of a crate in turn" ""
else
    record 0 "crates come in a fixed order, each shard of a crate in turn" "got:
$first_lines"
fi

expect_output "$rotation" "the order of the arguments does not change the rotation" -- \
    rotation ofm_core=8360 engine=1547 db=488 domain=367

# 40 seconds a mutant and a 12,000-second budget: 300 mutants fill a slot exactly, 301 do not.
expect_output "ofm_core 0/1" "a crate that exactly fills one slot gets one" -- \
    first ofm_core domain=1 db=1 engine=1 ofm_core=300
expect_output "ofm_core 0/2" "one mutant over the budget is a second slot, not an overrun" -- \
    first ofm_core domain=1 db=1 engine=1 ofm_core=301

echo "mutation-slot.test: picking a week's slots"

# With these counts the rotation is: domain 0/1, db 0/1, engine 0/1, ofm_core 0/3, 1/3, 2/3.
small=(domain=1 db=1 engine=1 ofm_core=900)

expect_output "domain 0/1" "week 0, job 0 is the first slot" -- pick 0 0 2 "${small[@]}"
expect_output "db 0/1" "the jobs of one week take consecutive slots" -- pick 0 1 2 "${small[@]}"
expect_output "engine 0/1" "next week carries on where this week stopped" -- pick 1 0 2 "${small[@]}"
expect_output "domain 0/1" "the rotation wraps round to the start" -- pick 3 0 2 "${small[@]}"
expect_output "ofm_core 1/3" "a large week number still lands inside the rotation" -- \
    pick 2951 0 2 "${small[@]}"

expect_rejected "more jobs a week than slots would test one slot twice" -- \
    pick 0 0 7 "${small[@]}"
expect_rejected "a job number outside the week's jobs" -- pick 0 2 2 "${small[@]}"
# "2+1" and "367+0" rather than a word: bash arithmetic already rejects a word, so a word would
# pass whether or not the script checks. These two are the ones it would quietly evaluate.
expect_rejected "a week that is not a plain number" -- pick 2+1 0 2 "${small[@]}"

echo "mutation-slot.test: a manual run of one crate"

expect_output "ofm_core 0/28" "a named crate starts at its own first shard" -- \
    first ofm_core "${counts[@]}"
expect_output "domain 0/1" "a small crate is a single shard" -- first domain "${counts[@]}"
expect_rejected "a crate that is not in the rotation" -- first ofm-cli "${counts[@]}"

echo "mutation-slot.test: counts that cannot be trusted"

# A listing that failed prints nothing, and nothing counts as zero. Planning on that would
# drop a crate from the rotation, and the job would go green having never looked at it.
expect_rejected "a zero count, which is what a failed listing looks like" -- \
    rotation domain=0 db=488 engine=1547 ofm_core=8360
expect_rejected "a count that is not a plain number" -- \
    rotation domain=367+0 db=488 engine=1547 ofm_core=8360
expect_rejected "a crate with no count at all" -- rotation domain=367 db=488 engine=1547
expect_rejected "a crate the script has no cost for" -- \
    rotation domain=367 db=488 engine=1547 ofm_core=8360 sim-bench=40
expect_rejected "an unknown command" -- shuffle "${counts[@]}"

echo ""
if [ "$failed" -ne 0 ]; then
    echo "mutation-slot.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "mutation-slot.test: all $passed cases behaved as expected"
