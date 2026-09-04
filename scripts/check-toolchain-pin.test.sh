#!/usr/bin/env bash
#
# Drives `check-toolchain-pin.sh` against deliberately broken repositories.
#
# The point is not coverage for its own sake. This project's rule is that a gate nobody has
# watched go red is not a gate, and a shell script that greps YAML is exactly the kind of check
# that rots quietly: a regex loosened during a refactor still exits 0 on the real repository,
# because the real repository is correct. Each fixture below names a way the toolchain could be
# selected wrongly, and asserts this script notices.
#
# Every "must fail" case here was a real bypass at some point.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/check-toolchain-pin.sh"
fixtures="$here/tests/toolchain-pin"

passed=0
failed=0

# expect <expected exit status> <fixture> <what this proves>
expect() {
    local want="$1" fixture="$2" description="$3"
    local output status

    output="$("$script" "$fixtures/$fixture/workflows" "$fixtures/$fixture/rust-toolchain.toml" 2>&1)"
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

echo "check-toolchain-pin.test: accepted repositories"
expect 0 agreeing \
    "a workflow pinned to the toolchain file's channel, beside one that builds no Rust"
expect 0 cargo-only-in-a-comment \
    "the word cargo in a comment is not a Rust build"
expect 0 cargo-deny-without-a-toolchain \
    "cargo deny and cargo machete compile nothing, so they need no toolchain step"

echo "check-toolchain-pin.test: rejected repositories"
expect 1 mismatched-action-pin \
    "an action pin naming a different version from rust-toolchain.toml"
expect 1 moving-channel \
    "rust-toolchain.toml pinning a moving channel"
expect 1 cargo-plus-toolchain \
    "cargo +nightly, which overrides rust-toolchain.toml outright"
expect 1 unpinned-cargo-build \
    "a cargo job with no toolchain step, in a repository whose other workflow is pinned"
expect 1 unpinned-tauri-cli-build \
    "a Rust build spelled as npx tauri build rather than cargo"
expect 1 cargo-deny-hiding-a-build \
    "a real cargo build in a workflow that also runs the exempt tools, including builds chained onto their own lines"

echo ""
if [ "$failed" -ne 0 ]; then
    echo "check-toolchain-pin.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "check-toolchain-pin.test: all $passed cases behaved as expected"
