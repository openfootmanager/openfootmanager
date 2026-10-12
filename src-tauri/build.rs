include!("build_revision.rs");

fn main() {
    tauri_build::build();
    embed_source_revision();
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
