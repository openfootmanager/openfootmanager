// Shared by `build.rs`, which embeds the revision in the app, and by the `mcp-e2e` fixture, which
// computes it for the working tree and compares. One definition, so the two cannot disagree about
// what "the same sources" means. Standard library only: `build.rs` has no other dependencies.

/// The workspace crates the app links. `sim-bench`, `ofm-cli` and `mcp-e2e` are not among them, and
/// of the crates that are, only `src`, `data` and the manifest are compiled in: their tests and
/// benches are not.
pub const LINKED_CRATES: [&str; 5] = ["domain", "engine", "ofm_core", "db", "mcp-results"];

/// What goes into the app binary, relative to `src-tauri`. Tests, docs, generated schemas, tooling
/// config and the crates the app does not link are not in it, so changing them never makes a
/// binary stale. The embedded frontend (`../dist`) is outside `src-tauri` and not covered either.
pub fn app_paths() -> Vec<String> {
    let mut paths: Vec<String> = [
        "src",
        "capabilities",
        "icons",
        "assets",
        "data",
        "databases",
        "Cargo.toml",
        "Cargo.lock",
        "tauri.conf.json",
        "build.rs",
        "build_revision.rs",
    ]
    .iter()
    .map(|path| path.to_string())
    .collect();
    for krate in LINKED_CRATES {
        for part in ["src", "data", "Cargo.toml"] {
            paths.push(format!("crates/{krate}/{part}"));
        }
    }
    paths
}

/// Names the sources of the app, so a binary can be told apart from the tree it is run against.
///
/// It is a digest of the working-tree *content* of the files in [`app_paths`] that git lists
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
    args.extend(app_paths());

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
