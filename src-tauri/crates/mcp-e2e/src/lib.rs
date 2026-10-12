//! Drives the built app the way an agent does: one real process per scenario, spoken to over
//! MCP, with the results read back as the typed structs the server itself serializes.
//!
//! - [`App`] starts and stops the process, and says what broke when it won't start.
//! - [`Client`] speaks the protocol. It never retries: a call that timed out may have run.
//! - [`tools`] is one typed method per tool.
//! - [`assert_refusal_changes_nothing`] proves a refused call left every layer as it was.
//! - [`expect_bug!`] records a known-broken route and tells you when it is fixed.

mod app;
mod bug;
pub mod career;
mod revision {
    include!("../../../build_revision.rs");
}
mod client;
mod snapshot;
pub mod tools;

pub use app::{real_startup_failures_file, App, AppConfig, LaunchError, Mode, Teardown};
pub use bug::expect_bug;
pub use client::{CallError, Client, Refusal};
pub use snapshot::{assert_refusal_changes_nothing, Snapshot};

pub use mcp_results as results;

#[cfg(test)]
mod revision_tests {
    use std::path::Path;
    use std::process::Command;

    use super::revision::source_revision;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    /// Given a checkout with `src-tauri` and a sibling directory
    /// When files change, are staged, committed or added
    /// Then the revision follows exactly the sources under `src-tauri`, and a clean tree is a bare tree hash.
    #[test]
    fn the_revision_follows_the_sources_under_src_tauri() {
        let repo = tempfile::tempdir().unwrap();
        let root = repo.path();
        let app = root.join("src-tauri");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        git(root, &["init", "-q"]);
        std::fs::write(app.join("lib.rs"), "fn a() {}").unwrap();
        std::fs::write(root.join("docs/readme.md"), "one").unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "one"]);

        let clean = source_revision(&app).unwrap();
        assert_eq!(clean.len(), 12, "a clean tree is a bare digest: {clean}");

        // The test crate is not part of the app, so editing it, tracked or new, changes nothing.
        std::fs::create_dir_all(app.join("crates/mcp-e2e")).unwrap();
        std::fs::write(app.join("crates/mcp-e2e/test.rs"), "fn t() {}").unwrap();
        assert_eq!(source_revision(&app).unwrap(), clean);
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "tests"]);
        assert_eq!(source_revision(&app).unwrap(), clean);
        std::fs::write(app.join("crates/mcp-e2e/test.rs"), "fn t2() {}").unwrap();
        assert_eq!(source_revision(&app).unwrap(), clean);

        // A change outside src-tauri, committed or not, is not a change to the app.
        std::fs::write(root.join("docs/readme.md"), "two").unwrap();
        git(root, &["commit", "-q", "-am", "docs"]);
        assert_eq!(source_revision(&app).unwrap(), clean);

        // An edit under src-tauri, an untracked file, and another edit each give another revision.
        std::fs::write(app.join("lib.rs"), "fn b() {}").unwrap();
        let edited = source_revision(&app).unwrap();
        assert_ne!(edited, clean);
        assert!(
            edited.starts_with(&clean) && edited.contains('.'),
            "{edited}"
        );
        std::fs::write(app.join("new.rs"), "fn c() {}").unwrap();
        let with_new_file = source_revision(&app).unwrap();
        assert_ne!(with_new_file, edited);
        std::fs::write(app.join("new.rs"), "fn d() {}").unwrap();
        assert_ne!(source_revision(&app).unwrap(), with_new_file);

        // Committing the edits turns them into a new bare tree hash.
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "app"]);
        let committed = source_revision(&app).unwrap();
        assert_eq!(committed.len(), 12);
        assert_ne!(committed, clean);
    }
}
