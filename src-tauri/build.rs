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

    // The revision depends on the sources and on what git thinks of them, so rerun when either moves.
    // The crates the app is built from; `crates/mcp-e2e` is not one of them.
    for path in [
        "src",
        "crates/domain",
        "crates/engine",
        "crates/ofm_core",
        "crates/db",
        "crates/mcp-results",
        "Cargo.toml",
        "Cargo.lock",
        "tauri.conf.json",
        "build.rs",
        "build_revision.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    for git_path in ["HEAD", "index"] {
        if let Ok(output) = std::process::Command::new("git")
            .arg("-C")
            .arg(&manifest_dir)
            .args([
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                git_path,
            ])
            .output()
        {
            let path = String::from_utf8_lossy(&output.stdout);
            if output.status.success() {
                println!("cargo:rerun-if-changed={}", path.trim());
            }
        }
    }
}
