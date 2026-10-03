#!/usr/bin/env bash
#
# Drives `check-appimage-libs.sh` against AppImages built to fail it.
#
# The real AppImage only exists on a release run, and a correct one passes, so the release run
# alone can't show that this check still bites. Each case below builds a fake AppImage, a shell
# script that answers `--appimage-extract` the way the real runtime does, or an AppDir that is
# already extracted, and asserts the exit status.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/check-appimage-libs.sh"

passed=0
failed=0

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# expect <expected exit status> <path> <what this proves>
expect() {
    local want="$1" target="$2" description="$3"
    local output status

    output="$("$script" "$target" 2>&1)"
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

# appdir <name> <library path inside the AppDir>...  ->  echoes an extracted AppDir
appdir() {
    local dir="$scratch/$1"
    shift
    mkdir -p "$dir/usr/lib"
    printf '#!/bin/sh\n' > "$dir/AppRun"
    local library
    for library in "$@"; do
        mkdir -p "$(dirname "$dir/$library")"
        : > "$dir/$library"
    done
    echo "$dir"
}

# bundle <name> <library path inside the AppDir>...  ->  echoes a bundle directory holding one
# fake AppImage that unpacks those libraries into ./squashfs-root, as the real runtime does
bundle() {
    local dir="$scratch/$1"
    shift
    mkdir -p "$dir"
    {
        printf '#!/bin/sh\n'
        printf '[ "$1" = --appimage-extract ] || exit 64\n'
        printf 'mkdir -p squashfs-root/usr/lib\n'
        printf ': > squashfs-root/AppRun\n'
        local library
        for library in "$@"; do
            printf 'mkdir -p "squashfs-root/%s" && : > "squashfs-root/%s"\n' \
                "$(dirname "$library")" "$library"
        done
    } > "$dir/Openfoot.Manager_0.0.0_amd64.AppImage"
    chmod +x "$dir/Openfoot.Manager_0.0.0_amd64.AppImage"
    echo "$dir"
}

expect 0 "$(appdir clean usr/lib/libgtk-3.so.0 usr/lib/libwebkit2gtk-4.1.so.0)" \
    "an AppDir with WebKit and GTK but no libwayland passes"

expect 1 "$(appdir client usr/lib/libwayland-client.so.0)" \
    "a bundled libwayland-client fails: the library behind #281"

# tauri-bundler 2.10 still bundles these three. Mesa's EGL driver resolves against all of them
# without a missing symbol, and the app starts on Fedora 44's Mesa with them present, so
# rejecting them would fail every release over libraries that do no harm.
expect 0 "$(appdir siblings usr/lib/libwayland-cursor.so.0 usr/lib/libwayland-egl.so.1 usr/lib/libwayland-server.so.0)" \
    "the other libwayland libraries pass: only the client breaks Mesa's EGL"

expect 1 "$(appdir nested usr/lib/x86_64-linux-gnu/libwayland-client.so.0)" \
    "a libwayland-client in a multiarch subdirectory still fails"

expect 1 "$(appdir versioned usr/lib/libwayland-client.so.0.20.0)" \
    "a fully versioned libwayland-client still fails, not only the soname link"

expect 0 "$(bundle bundle-clean usr/lib/libgtk-3.so.0)" \
    "a bundle directory holding a clean AppImage passes"

expect 1 "$(bundle bundle-wayland usr/lib/libwayland-client.so.0)" \
    "a bundle directory holding an AppImage with libwayland fails"

empty="$scratch/empty"
mkdir -p "$empty"
expect 2 "$empty" \
    "a directory with no AppImage is an error, never a pass: a wrong path must not look clean"

twice="$(bundle twice usr/lib/libgtk-3.so.0)"
cp "$twice/Openfoot.Manager_0.0.0_amd64.AppImage" "$twice/Openfoot.Manager_0.0.1_amd64.AppImage"
expect 2 "$twice" \
    "two AppImages is an error: which one shipped is ambiguous"

expect 2 "$scratch/does-not-exist" \
    "a missing path is an error, never a pass"

echo ""
if [ "$failed" -ne 0 ]; then
    echo "check-appimage-libs.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "check-appimage-libs.test: all $passed cases behaved as expected"
