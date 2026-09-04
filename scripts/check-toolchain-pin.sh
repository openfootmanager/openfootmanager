#!/usr/bin/env bash
#
# Assert that nothing in the repository can select a Rust toolchain other than the one
# `rust-toolchain.toml` names.
#
# Why this exists: the local default toolchain and CI's pinned one used to drift, so a change
# could be clean locally and red on CI (1.95's clippy carries lints 1.94 lacks). `rust-toolchain.toml`
# fixes that for anyone running cargo — but only while nothing overrides or contradicts it.
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
#
# Usage: check-toolchain-pin.sh [workflow_dir] [toolchain_file]
# The arguments exist so the fixtures in scripts/tests/toolchain-pin/ can drive this script
# against deliberately broken repositories. Nothing else should pass them.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workflow_dir="${1:-$repo_root/.github/workflows}"
toolchain_file="${2:-$repo_root/rust-toolchain.toml}"

# Report paths relative to the repository so the default run reads the way it always has.
relative() { printf '%s' "${1#"$repo_root"/}"; }

fail() {
    echo "check-toolchain-pin: $1" >&2
    exit 1
}

[ -f "$toolchain_file" ] || fail "$(relative "$toolchain_file") is missing. It is what makes local cargo match CI."
[ -d "$workflow_dir" ] || fail "$(relative "$workflow_dir") is missing, so no workflow could be checked."

# channel = "1.95.0"  ->  1.95.0
channel="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$toolchain_file")"
[ -n "$channel" ] || fail "no channel found in $(relative "$toolchain_file")"

case "$channel" in
    stable | beta | nightly | nightly-*)
        fail "$(relative "$toolchain_file") pins '$channel', which is a moving target. Pin an exact version."
        ;;
esac

status=0

# ── 1. Every action pin names the same version ────────────────────────────────────────────────
while IFS= read -r line; do
    file="${line%%:*}"
    rest="${line#*:}"
    lineno="${rest%%:*}"
    pin="$(printf '%s\n' "$line" | sed 's/.*dtolnay\/rust-toolchain@//; s/[[:space:]].*//')"

    if [ "$pin" != "$channel" ]; then
        echo "$(relative "$file"):$lineno: pins dtolnay/rust-toolchain@$pin but $(relative "$toolchain_file") says $channel" >&2
        status=1
    fi
done < <(grep -rn "dtolnay/rust-toolchain@" "$workflow_dir" || true)

# ── 2. Nothing overrides the toolchain on the cargo command line ──────────────────────────────
# `cargo +nightly build` beats rust-toolchain.toml outright: rustup honours the `+toolchain`
# argument above the file. It is the one spelling that defeats the pin rather than merely
# ignoring it, so it is banned rather than pin-checked — there is no version it could name that
# would be safe, because the whole point of the file is to be the single answer.
while IFS= read -r line; do
    file="${line%%:*}"
    rest="${line#*:}"
    lineno="${rest%%:*}"

    echo "$(relative "$file"):$lineno: uses \`cargo +<toolchain>\`, which overrides $(relative "$toolchain_file")" >&2
    status=1
done < <(grep -rnE '(^|[^[:alnum:]_-])cargo[[:space:]]+\+' "$workflow_dir" || true)

# ── 3. A workflow that builds Rust installs the toolchain through the action ──────────────────
# Not because an unpinned job would otherwise get the runner's default — `rust-toolchain.toml`
# already prevents that, since rustup walks up from the checkout and installs the channel it
# names. What the action supplies that the file deliberately does not is `targets`: the macOS
# release matrices build `--target aarch64-apple-darwin`, and a job that skips the action has no
# Apple std to build against. It also warms the toolchain and component download rather than
# paying for it mid-build.
#
# Full-line comments are stripped first: the previous version of this matched the word `cargo`
# in a comment and demanded a pin from a workflow that builds nothing.
builds_rust='(^|[^[:alnum:]_-])cargo[[:space:]]+[+a-z]|uses:[[:space:]]*tauri-apps/tauri-action|(^|[^[:alnum:]_-])tauri[[:space:]]+build'

# `cargo deny` and `cargo machete` are not Rust builds. Both are prebuilt binaries — one shells
# out to `cargo metadata`, the other only parses manifests and greps sources — so they compile
# nothing, need no `targets`, and have no toolchain download to warm, which is the whole of what
# the rule above exists to guarantee. `rust-toolchain.toml` still governs any `cargo` they call,
# so they are pinned; they just do not need the action to be.
#
# Blanked token by token rather than line by line, so `cargo deny check && cargo build` still
# trips the rule on its second half. That matters more than it looks: a whole-line exclusion
# would turn this into a one-line bypass for any job willing to write both on one line.
#
# The trailing delimiter is load-bearing for the same reason. Matching a bare prefix would
# exempt `cargo deny-audit` — any future subcommand merely *starting* with an exempt name —
# and the exemption is meant to name two specific tools, not a namespace. Both delimiters are
# captured and put back so two exempt calls on one line still both blank.
strip_non_builders() {
    grep -v '^[[:space:]]*#' "$1" |
        sed -E 's/(^|[^[:alnum:]_-])cargo[[:space:]]+(deny|machete)([[:space:]]|$)/\1cargo-\2\3/g'
}

for workflow in "$workflow_dir"/*.yml "$workflow_dir"/*.yaml; do
    [ -e "$workflow" ] || continue

    strip_non_builders "$workflow" | grep -qE "$builds_rust" || continue
    grep -q "dtolnay/rust-toolchain@" "$workflow" && continue

    echo "$(relative "$workflow"): builds Rust but never installs the toolchain through dtolnay/rust-toolchain@$channel" >&2
    status=1
done

if [ "$status" -ne 0 ]; then
    echo "" >&2
    fail "toolchain selection is not consistent. Fix the lines above, or the release matrices drift silently."
fi

echo "check-toolchain-pin: every Rust toolchain selection agrees on $channel"
