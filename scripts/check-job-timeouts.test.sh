#!/usr/bin/env bash
#
# Drives `check-job-timeouts.sh` against deliberately broken workflows.
#
# The real build-check.yml is correct, so a check that had stopped working would pass on it.
# Only watching it reject things shows it still works. Fixtures are generated: each is a few
# lines.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/check-job-timeouts.sh"

passed=0
failed=0

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# expect <expected exit status> <name> <file contents> <what this proves>
expect() {
    local want="$1" name="$2" contents="$3" description="$4"
    local file="$scratch/$name.yml" output status

    printf '%s' "$contents" > "$file"

    output="$("$script" "$file" 2>&1)"
    status=$?

    if [ "$status" -eq "$want" ]; then
        passed=$((passed + 1))
        printf '  ok    %s\n' "$description"
    else
        failed=$((failed + 1))
        printf '  FAIL  %s\n' "$description"
        printf '        expected exit %s, got %s. Output was:\n' "$want" "$status"
        printf '        %s\n' "$output"
    fi
}

echo "check-job-timeouts.test"

expect 0 every-job "name: ci
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - run: true
  lint:
    runs-on: ubuntu-latest
    # a comment between the settings
    timeout-minutes: 10 # and one after
    steps:
      - run: true
" "every job with a job-level timeout passes"

expect 1 one-missing "name: ci
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - run: true
  lint:
    runs-on: ubuntu-latest
    steps:
      - run: true
" "a job with no timeout fails"

expect 1 step-only "name: ci
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: true
        timeout-minutes: 5
" "a timeout on a step is not the job's"

expect 1 commented-out "name: ci
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    # timeout-minutes: 30
    steps:
      - run: true
" "a commented-out timeout fails"

expect 1 comment-after-key "name: ci
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - run: true
  lint: # a note after the key
    runs-on: ubuntu-latest
    steps:
      - run: true
" "a job whose key carries a comment is still a job, and fails without a timeout"

expect 1 anchor-after-key "name: ci
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - run: true
  lint: &lint
    runs-on: ubuntu-latest
    steps:
      - run: true
" "a job whose key carries an anchor is still a job, and fails without a timeout"

expect 0 commented-key-with-timeout "name: ci
on: push
jobs:
  lint: # a note after the key
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: true
" "a commented job key with its own timeout passes"

expect 1 no-jobs "name: ci
on: push
" "a file with no jobs fails rather than passing on nothing"

echo
echo "check-job-timeouts.test: $passed passed, $failed failed"
[ "$failed" -eq 0 ]
