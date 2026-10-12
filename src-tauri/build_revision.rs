// Shared by `build.rs`, which embeds the revision in the app, and by the `mcp-e2e` fixture, which
// computes it for the working tree and compares. One definition, so the two cannot disagree about
// what "the same sources" means. Standard library only: `build.rs` has no other dependencies.

/// What goes into the app binary. Tests, docs, generated schemas and tooling config do not, so
/// changing them never makes a binary stale. `crates/mcp-e2e` is under `crates` but is not part of
/// the app either, and is left out wherever this list is used.
pub const APP_PATHS: [&str; 11] = [
    "src",
    "crates",
    "capabilities",
    "icons",
    "assets",
    "data",
    "databases",
    "Cargo.toml",
    "Cargo.lock",
    "tauri.conf.json",
    "build.rs",
];

/// The test crate, which lives under `crates` but is not built into the app.
pub const NOT_THE_APP: &str = "crates/mcp-e2e";

/// Names the sources of the app, so a binary can be told apart from the tree it is run against.
///
/// It is a digest of the working-tree *content* of the files in [`APP_PATHS`] that git lists
/// (tracked, or untracked and not ignored), so it follows what would be compiled and nothing else:
/// committing, staging or switching branches over identical files leaves it alone, and so does a
/// change to docs, tests, the frontend or the test crate. `None` when git cannot list the files
/// (no git, a source tarball), in which case the build has no revision.
pub fn source_revision(src_tauri: &std::path::Path) -> Option<String> {
    use std::collections::BTreeSet;
    use std::process::Command;

    let mut args: Vec<String> = [
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
        "--",
    ]
    .iter()
    .map(|arg| arg.to_string())
    .collect();
    args.extend(APP_PATHS.iter().map(|path| path.to_string()));
    args.push("build_revision.rs".to_string());
    args.push(format!(":(exclude){NOT_THE_APP}"));

    let output = Command::new("git")
        .arg("-C")
        .arg(src_tauri)
        .args(&args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let files: BTreeSet<&[u8]> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .collect();

    // FNV-1a over each path and its bytes: stable across toolchains, and no hashing crate.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for name in files {
        // A tracked file that was deleted is listed but gone; its absence is part of the content.
        let Ok(content) = std::fs::read(src_tauri.join(String::from_utf8_lossy(name).as_ref()))
        else {
            continue;
        };
        feed(name);
        feed(&[0]);
        feed(&(content.len() as u64).to_le_bytes());
        feed(&content);
    }
    Some(format!("{hash:016x}"))
}
