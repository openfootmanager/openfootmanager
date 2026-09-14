#!/usr/bin/env bash
#
# Drives `check-blame-ignore-revs.sh` against deliberately broken files.
#
# This check guards something nobody can review by reading it — a 40-character hash — so the
# only way to know it still works is to watch it reject things. And it has a failure mode its
# subject does not: being *stricter than git*. A wrong SHA makes blame quietly useless, which is
# bad; a check that reds the build over a line git accepts is worse, because that is the kind of
# gate people switch off. Both directions are covered below.
#
# Fixtures are generated rather than committed. Each is one or two lines, and the accepted case
# has to name a commit that actually exists, which no checked-in file can promise.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/check-blame-ignore-revs.sh"

passed=0
failed=0

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

head_sha="$(git -C "$here/.." rev-parse HEAD)"
upper_sha="$(printf '%s' "$head_sha" | tr '[:lower:]' '[:upper:]')"

# expect <expected exit status> <name> <file contents> <what this proves>
expect() {
    local want="$1" name="$2" contents="$3" description="$4"
    local file="$scratch/$name" output status

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

echo "check-blame-ignore-revs.test: files git itself accepts"

# Every shape here was verified against `git -c blame.ignoreRevsFile=… blame`, which exits 0 on
# all of them. This check may not be stricter than the thing it is guarding.
expect 0 tolerated "# a comment
   # an indented comment


$head_sha
" "the line shapes git tolerates — blank, whitespace-only, indented comment, real SHA"

expect 0 uppercase "$upper_sha
" "an uppercase object ID, which git resolves and this used to call the wrong length"

expect 0 padded "  $head_sha
" "a SHA padded with whitespace at both ends"

echo "check-blame-ignore-revs.test: files that would silently do nothing"

expect 1 not-a-commit "0000000000000000000000000000000000000000
" "a well-formed SHA that is not a commit here"

expect 1 short "$(printf '%s' "$head_sha" | cut -c1-7)
" "a short SHA, which git ignores without saying so"

expect 1 empty "# nothing but prose
" "a file that names no commits, so it cannot be doing its job"

expect 1 truncated "$(printf '%s' "$head_sha" | cut -c1-39)
" "a SHA one character short, the shape a bad copy-paste actually makes"

echo ""
if [ "$failed" -ne 0 ]; then
    echo "check-blame-ignore-revs.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "check-blame-ignore-revs.test: all $passed cases behaved as expected"
