#!/usr/bin/env bash
#
# Fail if the Linux AppImage bundles its own libwayland-client.
#
# Why this exists (#281): the AppImage launcher puts its bundled libraries ahead of the host's.
# A libwayland-client taken from the Ubuntu 22.04 build runner then shadows the player's own, and
# current Mesa's EGL driver needs libwayland 1.23 or newer. It fails to load, WebKit can't create
# an EGL display, its web process aborts, and the window stays white. Every machine using Mesa
# with a newer libwayland is hit: AMD, Intel, and any rolling distribution. NVIDIA's own EGL
# driver doesn't care, which is how it shipped unnoticed.
#
# `@tauri-apps/cli` 2.12 (tauri-bundler 2.10) stopped bundling it. This check keeps it that way:
# a lockfile downgrade or a bundler regression would bring the white window back, and nothing
# else in CI would notice. The full story is in docs/LINUX_GRAPHICS.md.
#
# Only the client library. The same bundler still ships libwayland-cursor, -egl and -server from
# the build runner. Mesa's EGL driver resolves against those without a missing symbol, and the
# app starts with them present, so failing on them would block every release over nothing. If a
# later Mesa does need something newer from one of them, add it here with the evidence.
#
# Usage: check-appimage-libs.sh <bundle dir with exactly one .AppImage | extracted AppDir>
# Exit:  0 clean, 1 libwayland-client bundled, 2 nothing to check (a wrong path must never pass)

set -euo pipefail

target="${1:?usage: check-appimage-libs.sh <bundle dir | extracted AppDir>}"

if [ ! -d "$target" ]; then
    echo "check-appimage-libs: $target is not a directory" >&2
    exit 2
fi

if [ -e "$target/AppRun" ]; then
    appdir="$target"
else
    shopt -s nullglob
    images=("$target"/*.AppImage)
    shopt -u nullglob
    if [ "${#images[@]}" -ne 1 ]; then
        echo "check-appimage-libs: expected exactly one .AppImage in $target, found ${#images[@]}" >&2
        exit 2
    fi
    image="$(cd "$(dirname "${images[0]}")" && pwd)/$(basename "${images[0]}")"

    scratch="$(mktemp -d)"
    trap 'rm -rf "$scratch"' EXIT
    # The runtime unpacks into ./squashfs-root. It needs no FUSE for this.
    (cd "$scratch" && "$image" --appimage-extract >/dev/null)
    appdir="$scratch/squashfs-root"
    if [ ! -d "$appdir" ]; then
        echo "check-appimage-libs: $image did not extract to squashfs-root" >&2
        exit 2
    fi
fi

# Captured rather than piped into `grep -q`: with pipefail, a match that stops grep early reads
# as a failed pipeline, and the check would invert.
bundled="$(find "$appdir" -name 'libwayland-client.so*' -print)"

if [ -n "$bundled" ]; then
    echo "check-appimage-libs: the AppImage bundles libwayland-client, which breaks Mesa's EGL on newer hosts (#281):" >&2
    echo "$bundled" | sed "s|^$appdir/|  |" >&2
    exit 1
fi

echo "check-appimage-libs: no bundled libwayland-client in $target"
