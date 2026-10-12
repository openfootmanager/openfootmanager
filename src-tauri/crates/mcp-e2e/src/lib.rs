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
pub mod windows;

pub use app::{
    gui_launch_maps_a_window, real_startup_failures_file, App, AppConfig, GuiLaunch, LaunchError,
    Mode, Teardown,
};
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

    /// Given a checkout with `src-tauri`, a sibling directory and a test crate
    /// When files change, are staged, committed, added or deleted
    /// Then the revision follows exactly the app's sources: not the commit, the index, the docs or
    /// the test crate, but any edit, new or deleted file among the sources.
    #[test]
    fn the_revision_follows_the_content_of_the_apps_sources() {
        let repo = tempfile::tempdir().unwrap();
        let root = repo.path();
        let app = root.join("src-tauri");
        for dir in [
            "src",
            "tests",
            "crates/mcp-e2e",
            "crates/sim-bench/src",
            "crates/ofm_core/src",
            "crates/ofm_core/tests",
        ] {
            std::fs::create_dir_all(app.join(dir)).unwrap();
        }
        std::fs::create_dir_all(root.join("docs")).unwrap();
        git(root, &["init", "-q"]);
        std::fs::write(app.join("src/lib.rs"), "fn a() {}").unwrap();
        std::fs::write(root.join("docs/readme.md"), "one").unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "one"]);

        let clean = source_revision(&app).unwrap();
        assert_eq!(clean.len(), 16, "{clean}");

        // None of these are the app: docs, other tests, the e2e crate, a commit that changes only them.
        std::fs::write(root.join("docs/readme.md"), "two").unwrap();
        std::fs::write(app.join("tests/architecture.rs"), "fn t() {}").unwrap();
        std::fs::write(app.join("crates/mcp-e2e/test.rs"), "fn t() {}").unwrap();
        // A linked crate's integration tests, and a crate the app does not link, are not the app.
        std::fs::write(app.join("crates/ofm_core/tests/world.rs"), "fn t() {}").unwrap();
        std::fs::write(app.join("crates/sim-bench/src/lib.rs"), "fn t() {}").unwrap();
        assert_eq!(source_revision(&app).unwrap(), clean);
        git(root, &["add", "-A"]);
        assert_eq!(source_revision(&app).unwrap(), clean, "staging");
        git(root, &["commit", "-q", "-m", "not the app"]);
        assert_eq!(source_revision(&app).unwrap(), clean, "committing");

        // A linked crate's sources are the app.
        std::fs::write(app.join("crates/ofm_core/src/lib.rs"), "fn core() {}").unwrap();
        let core_edited = source_revision(&app).unwrap();
        assert_ne!(core_edited, clean);
        std::fs::write(app.join("crates/ofm_core/tests/world.rs"), "fn t2() {}").unwrap();
        assert_eq!(
            source_revision(&app).unwrap(),
            core_edited,
            "an ofm_core test edit"
        );
        std::fs::remove_file(app.join("crates/ofm_core/src/lib.rs")).unwrap();
        assert_eq!(source_revision(&app).unwrap(), clean);

        // Edits, new files and deleted files are the app.
        std::fs::write(app.join("src/lib.rs"), "fn b() {}").unwrap();
        let edited = source_revision(&app).unwrap();
        assert_ne!(edited, clean);
        std::fs::write(app.join("src/new.rs"), "fn c() {}").unwrap();
        let with_new_file = source_revision(&app).unwrap();
        assert_ne!(with_new_file, edited);
        std::fs::write(app.join("src/new.rs"), "fn d() {}").unwrap();
        let new_file_edited = source_revision(&app).unwrap();
        assert_ne!(new_file_edited, with_new_file);

        // The same content, committed, is the same revision: a commit does not make a binary stale.
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "app"]);
        assert_eq!(source_revision(&app).unwrap(), new_file_edited);

        std::fs::remove_file(app.join("src/new.rs")).unwrap();
        assert_ne!(source_revision(&app).unwrap(), new_file_edited);
    }

    /// Given a source tarball (a directory that is not a git repository)
    /// When the revision is asked for
    /// Then git cannot say and the answer is `None`, never a panic, so the build still works.
    #[test]
    fn a_tree_with_no_git_has_no_revision() {
        let tarball = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tarball.path().join("src")).unwrap();
        std::fs::write(tarball.path().join("src/lib.rs"), "fn a() {}").unwrap();

        assert_eq!(source_revision(tarball.path()), None);
    }
}
