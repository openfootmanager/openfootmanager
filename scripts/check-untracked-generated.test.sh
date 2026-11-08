#!/usr/bin/env bash
#
# Drives `check-untracked-generated.sh` against scratch repositories.
#
# The real repository passes, which proves nothing about whether the check still notices a tracked
# file: these fixtures do. Each builds a throwaway repository, so nothing here depends on the
# state of the checkout it runs in.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/check-untracked-generated.sh"
generated="src/dev/pageHarness/fixture.generated.ts"

passed=0
failed=0
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# A fresh repository with one ordinary committed file and .gitignore ignoring the generated one.
new_repo() {
    local repo="$scratch/$1"
    git init -q "$repo"
    git -C "$repo" config user.email "check@example.invalid"
    git -C "$repo" config user.name "check"
    mkdir -p "$repo/src/dev/pageHarness"
    printf 'export {};\n' > "$repo/src/dev/pageHarness/fixtureAnswers.ts"
    printf '%s\n' "$generated" > "$repo/.gitignore"
    git -C "$repo" add .gitignore src/dev/pageHarness/fixtureAnswers.ts
    git -C "$repo" commit -q -m "base"
    printf '%s' "$repo"
}

# expect <expected exit status> <repo> <what this proves>
expect() {
    local want="$1" repo="$2" description="$3" output status
    output="$("$script" "$repo" "$generated" 2>&1)"
    status=$?
    if [ "$status" -eq "$want" ]; then
        passed=$((passed + 1))
        printf '  ok    %s\n' "$description"
    else
        failed=$((failed + 1))
        printf '  FAIL  %s\n' "$description"
        printf '        expected exit %s, got %s. Output was:\n        %s\n' "$want" "$status" "$output"
    fi
}

echo "check-untracked-generated.test"

repo="$(new_repo absent)"
expect 0 "$repo" "passes when the generated file does not exist"

repo="$(new_repo ignored)"
printf 'export const gameState = {};\n' > "$repo/$generated"
expect 0 "$repo" "passes when the generated file is on disk but untracked"

repo="$(new_repo forced)"
printf 'export const gameState = {};\n' > "$repo/$generated"
git -C "$repo" add -f "$generated"
expect 1 "$repo" "fails when the generated file is force-added to the index"

repo="$(new_repo committed)"
printf 'export const gameState = {};\n' > "$repo/$generated"
git -C "$repo" add -f "$generated"
git -C "$repo" commit -q -m "commit the generated file"
expect 1 "$repo" "fails when the generated file is committed"

echo "check-untracked-generated.test: $passed passed, $failed failed"
[ "$failed" -eq 0 ]
