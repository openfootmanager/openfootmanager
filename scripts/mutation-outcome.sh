#!/usr/bin/env bash
#
# Decides whether a cargo-mutants run failed, or only found what it exists to find.
#
# Usage: mutation-outcome.sh <cargo-mutants exit status> <path to outcomes.json>
#
# Exits 0 when the run did its job and left a report. Otherwise exits with cargo-mutants' own
# status, or 1 when the status was fine but the report is missing.
#
# Surviving mutants are the *output* of the weekly job, not a failure — a red tick on a job
# nobody can act on today is how a scheduled job becomes invisible. So exactly the statuses that
# mean "the run worked" are accepted, and nothing else: a bare `|| true` would turn a missing
# tool or a compilation failure into a green job with no report, the silent-pass shape this
# whole programme exists to stop.
#
# The statuses, from `src/exit_code.rs` in cargo-mutants 25.3.1 (the version the workflow pins):
#
#   0  every mutant was caught
#   2  some mutants survived
#   3  some mutant's tests timed out — usually a mutation that loops forever. cargo-mutants
#      ranks this *above* 2 (`LabOutcome::exit_code`), so a slot with one slow mutant reports 3
#      however many survivors it also found. Accepting only 0 and 2 failed the 6 Oct 2026 run
#      after it had finished all 140 of its mutants.
#   1  bad arguments            4  the tests fail on the unmutated tree
#   5, 6  a `--in-diff` problem  70 an internal error
#
# Re-check the list when the pinned version changes.

set -uo pipefail

status="${1-}"
report="${2-}"

if [[ ! "$status" =~ ^[0-9]{1,3}$ ]]; then
    echo "mutation-outcome: '$status' is not an exit status" >&2
    exit 1
fi

case "$status" in
    0 | 2 | 3) ;;
    *)
        echo "cargo-mutants failed with status $status — a tool or build failure," >&2
        echo "not a surviving mutant." >&2
        exit "$status"
        ;;
esac

# A run that produced no report produced nothing, whatever it exited with.
if [ ! -s "$report" ]; then
    echo "cargo-mutants exited $status but wrote no report at '$report'." >&2
    exit 1
fi
