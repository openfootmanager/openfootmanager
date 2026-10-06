#!/usr/bin/env bash
#
# Drives `mutation-outcome.sh`, which decides whether a cargo-mutants run went red because the
# *tool* failed or merely because it found what it exists to find.
#
# This one check has been wrong three times, each time costing a scheduled run of hours:
#
#   * 27 Sep 2026 — exit 2 (survivors) killed the step before the line meant to accept it.
#   * 27 Sep 2026 — the report was looked for one directory above where cargo-mutants writes it.
#   * 6 Oct 2026  — exit 3 (a mutant timed out) was treated as a tool failure. cargo-mutants ranks
#                   it above exit 2, so one slow mutant reds a slot whatever else it found.
#
# Every case below pins one of those, or the failures this check must still catch.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/mutation-outcome.sh"

passed=0
failed=0

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

report="$scratch/outcomes.json"
printf '{"outcomes": []}\n' > "$report"
empty_report="$scratch/empty.json"
: > "$empty_report"
missing_report="$scratch/missing.json"

# expect <expected exit status> <cargo-mutants status> <report path> <what this proves>
expect() {
    local want="$1" status="$2" path="$3" description="$4"
    local output got
    output="$("$script" "$status" "$path" 2>&1)"
    got=$?
    if [ "$got" -eq "$want" ]; then
        passed=$((passed + 1))
        printf '  ok    %s\n' "$description"
    else
        failed=$((failed + 1))
        printf '  FAIL  %s\n' "$description"
        printf '        expected exit %s, got %s. Output was:\n' "$want" "$got"
        printf '        %s\n' "$output"
    fi
}

echo "mutation-outcome.test: runs that did their job"
expect 0 0 "$report" "every mutant caught"
expect 0 2 "$report" "some mutants survived — the report is the output, not a failure"
expect 0 3 "$report" "a mutant timed out, which cargo-mutants reports ahead of survivors"

echo "mutation-outcome.test: runs that did not"
expect 1 1 "$report" "a usage error"
expect 4 4 "$report" "the tests already fail on the unmutated tree"
expect 70 70 "$report" "an internal error in cargo-mutants"
expect 124 124 "$report" "killed from outside, as by a timeout wrapper"
expect 1 2 "$missing_report" "survivors claimed, but no report was written"
expect 1 0 "$empty_report" "a clean exit with an empty report"
expect 1 3 "$missing_report" "a timeout claimed, but no report was written"

echo "mutation-outcome.test: arguments that cannot be trusted"
expect 1 "" "$report" "no status at all"
expect 1 two "$report" "a status that is not a number"

echo ""
if [ "$failed" -ne 0 ]; then
    echo "mutation-outcome.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "mutation-outcome.test: all $passed cases behaved as expected"
