#!/usr/bin/env bash
#
# Fails when a job in one of the given workflow files has no job-level `timeout-minutes`.
#
# Without one, GitHub lets a job run for six hours. A test that hangs (a mutation of the
# stand-in id loop showed one can) would then hold the merge queue for six hours instead of
# failing within the hour. A timeout on a single step does not count: it is the job's that bounds
# the queue.
#
# Reads YAML with awk, so it relies on the layout every workflow here uses: `jobs:` at the left
# margin, each job's key indented two spaces, the job's own settings four. It fails on a file
# with no jobs at all, so a layout it cannot read is a red build, not a silent pass.
#
# Usage: check-job-timeouts.sh <workflow.yml>...

set -euo pipefail

if [ "$#" -eq 0 ]; then
    echo "usage: $0 <workflow.yml>..." >&2
    exit 2
fi

status=0
for file in "$@"; do
    report="$(awk '
        function close_job() {
            if (job != "" && !has_timeout) print "missing " job
            job = ""
        }
        /^jobs:[[:space:]]*$/ { in_jobs = 1; next }
        in_jobs && /^[^[:space:]#]/ { close_job(); in_jobs = 0 }
        in_jobs && /^  [A-Za-z0-9_-]+:[[:space:]]*$/ {
            close_job()
            job = $1
            sub(/:$/, "", job)
            has_timeout = 0
            jobs_seen++
            next
        }
        in_jobs && job != "" && /^    timeout-minutes:[[:space:]]*[0-9]+[[:space:]]*(#.*)?$/ {
            has_timeout = 1
        }
        END {
            close_job()
            if (jobs_seen == 0) print "nojobs"
        }
    ' "$file")"

    if [ "$report" = "nojobs" ]; then
        echo "$file: no jobs found; is \`jobs:\` at the left margin with two-space job keys?" >&2
        status=1
        continue
    fi
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        echo "$file: job '${line#missing }' has no job-level timeout-minutes" >&2
        status=1
    done <<< "$report"
done

exit "$status"
