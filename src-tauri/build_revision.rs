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
/// It digests the committed files under `src-tauri` (not HEAD itself: a commit that touches
/// nothing there, a doc or a frontend change, leaves the revision alone and does not make a binary
/// stale), then any uncommitted changes and untracked files under it. The test crate
/// `crates/mcp-e2e` is left out of all three: the app does not contain it, and editing a test
/// must not demand a rebuild of the app. `None` when git cannot say (no git, not a checkout).
pub fn source_revision(src_tauri: &std::path::Path) -> Option<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut app_sources: Vec<String> = APP_PATHS.iter().map(|path| path.to_string()).collect();
    app_sources.push("build_revision.rs".to_string());
    app_sources.push(format!(":(exclude){NOT_THE_APP}"));
    let app_sources: Vec<&str> = app_sources.iter().map(String::as_str).collect();

    let git = |args: &[&str]| -> Option<Vec<u8>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(src_tauri)
            .args(args)
            .output()
            .ok()?;
        output.status.success().then_some(output.stdout)
    };
    let digest = |bytes: &[u8]| -> Option<String> {
        let mut hasher = Command::new("git")
            .arg("-C")
            .arg(src_tauri)
            .args(["hash-object", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .ok()?;
        hasher.stdin.take()?.write_all(bytes).ok()?;
        let output = hasher.wait_with_output().ok()?;
        let digest = String::from_utf8(output.stdout).ok()?;
        output.status.success().then(|| digest.trim().get(..12).map(str::to_string))?

    };

    // `ls-tree` has no exclude pathspec, so the test crate is filtered out here.
    let listing = git(&[&["ls-tree", "-r", "HEAD", "--"][..], &app_sources[..app_sources.len() - 1]].concat())?;
    let app_files: Vec<u8> = listing
        .split_inclusive(|byte| *byte == b'\n')
        .filter(|line| {
            let path = line.split(|byte| *byte == b'\t').nth(1).unwrap_or_default();
            !path.starts_with(format!("{NOT_THE_APP}/").as_bytes())
        })
        .flatten()
        .copied()
        .collect();
    let committed = digest(&app_files)?;

    let mut changes = git(&[&["diff", "HEAD", "--no-ext-diff", "--"][..], &app_sources[..]].concat())?;
    let untracked = git(
        &[
            &["ls-files", "--others", "--exclude-standard", "-z", "--"][..],
            &app_sources[..],
        ]
        .concat(),
    )?;
    for name in untracked
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        changes.extend_from_slice(name);
        let path = src_tauri.join(String::from_utf8_lossy(name).as_ref());
        changes.extend(std::fs::read(path).unwrap_or_default());
    }
    if changes.is_empty() {
        return Some(committed);
    }
    Some(format!("{committed}.{}", digest(&changes)?))
}
