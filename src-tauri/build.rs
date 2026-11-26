include!("build_revision.rs");

fn main() {
    tauri_build::build();
    embed_source_revision();
    delay_load_comctl32();
}

/// Test binaries on Windows import `TaskDialogIndirect` from comctl32 through the dialog stack.
/// Without an application manifest the loader picks comctl32 v5.82, which does not export it, and
/// the process dies with STATUS_ENTRYPOINT_NOT_FOUND before the test harness starts (#1458). The
/// app binary gets a v6 manifest from Tauri; the lib unit-test binary cannot (a global manifest
/// collides with that one), so comctl32 is delay-loaded instead: a test that never opens a dialog
/// never loads it, and the app still resolves v6 through its own manifest when it does.
fn delay_load_comctl32() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    println!("cargo:rustc-link-arg=/DELAYLOAD:comctl32.dll");
    println!("cargo:rustc-link-arg=delayimp.lib");
}

/// `OFM_BUILD_REVISION` names the sources this binary was built from (empty when git cannot say).
/// The MCP server reports it so a test run can tell a stale binary from a current one.
fn embed_source_revision() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let revision = source_revision(&manifest_dir).unwrap_or_default();
    println!("cargo:rustc-env=OFM_BUILD_REVISION={revision}");

    // Everything the revision digests, so the build reruns when any of it changes. A path that does
    // not exist is left out: cargo reruns a build script on every build for a missing one.
    for path in app_paths() {
        if manifest_dir.join(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}
