#!/usr/bin/env bash
#
# Assert that every Rust toolchain pin in the repository names the same version.
#
# Why this exists: the local default toolchain and CI's pinned one used to drift, so a change
# could be clean locally and red on CI (1.95's clippy carries lints 1.94 lacks). `rust-toolchain.toml`
# fixes that for anyone running cargo, but only while it agrees with what the workflows install.
#
# The macOS release builds make the agreement load-bearing rather than cosmetic. Both release
# workflows pass `targets: aarch64-apple-darwin,x86_64-apple-darwin` to the action, which installs
# those targets *into the toolchain the action selects*. If rust-toolchain.toml then selects a
# different toolchain, cargo picks one that has no Apple targets and the macOS build fails — on a
# workflow that only runs at release time, where nobody is watching.
#
# Deliberately enumerates rather than checking a known list of files: a script that verifies the
# four pins it already knows about passes happily on the day someone adds a fifth, which is the
# only day it matters.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

toolchain_file="rust-toolchain.toml"
workflow_dir=".github/workflows"

fail() {
    echo "check-toolchain-pin: $1" >&2
    exit 1
}

[ -f "$toolchain_file" ] || fail "$toolchain_file is missing. It is what makes local cargo match CI."

# channel = "1.95.0"  ->  1.95.0
channel="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$toolchain_file")"
[ -n "$channel" ] || fail "no channel found in $toolchain_file"

case "$channel" in
    stable | beta | nightly | nightly-*)
        fail "$toolchain_file pins '$channel', which is a moving target. Pin an exact version."
        ;;
esac

status=0

# Every pin found anywhere under the workflow directory must name the same version.
while IFS= read -r line; do
    file="${line%%:*}"
    rest="${line#*:}"
    lineno="${rest%%:*}"
    pin="$(printf '%s\n' "$line" | sed 's/.*dtolnay\/rust-toolchain@//; s/[[:space:]].*//')"

    if [ "$pin" != "$channel" ]; then
        echo "$file:$lineno: pins dtolnay/rust-toolchain@$pin but $toolchain_file says $channel" >&2
        status=1
    fi
done < <(grep -rn "dtolnay/rust-toolchain@" "$workflow_dir" || true)

# A workflow that builds Rust without pinning a toolchain silently gets whatever the runner ships.
for workflow in "$workflow_dir"/*.yml "$workflow_dir"/*.yaml; do
    [ -e "$workflow" ] || continue
    # Match how Rust actually gets built — a cargo subcommand, or the action that shells out to
    # cargo — not the bare word. `nightly-release-manifest.yml` names "nightly-tauri-action.yml"
    # in an env var and builds nothing.
    grep -qE '(^|[^[:alnum:]_-])cargo[[:space:]]+[a-z]|uses:[[:space:]]*tauri-apps/tauri-action' "$workflow" || continue
    grep -q "dtolnay/rust-toolchain@" "$workflow" && continue

    echo "$workflow: builds Rust but installs no pinned toolchain" >&2
    status=1
done

if [ "$status" -ne 0 ]; then
    echo "" >&2
    fail "toolchain pins disagree. Update them together, or the release matrices drift silently."
fi

echo "check-toolchain-pin: all Rust toolchain pins agree on $channel"
