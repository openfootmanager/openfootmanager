//! Mechanical enforcement of the crate boundaries described in `src-tauri/CLAUDE.md` §1 and
//! `docs/ARCHITECTURE.md`.
//!
//! Until now the project's most important architectural rule — "`engine` never imports `domain`"
//! — was enforced by prose plus a review agent reading that prose. That is weak in a specific way:
//! the same document also claimed `ofm_core` depends on `db`, when the manifests say `db` depends
//! on `ofm_core`. A wrong diagram cannot catch a wrong dependency, and nobody noticed for a long
//! time. These tests check the manifests themselves, so they cannot drift from the code they
//! describe.
//!
//! # Why `cargo metadata` and not a grep over `Cargo.toml`
//!
//! A crate cannot `use domain::` without `domain` appearing in its own manifest, so manifest
//! granularity really is a complete check — but only if the manifest is *parsed*. Every one of
//! these declares a real dependency edge while evading a naive line match:
//!
//! - a dotted table, `[dependencies.domain]` (the root manifest already declares five deps this
//!   way, so this is a live spelling in this repo, not a hypothetical)
//! - workspace inheritance, `domain.workspace = true`
//! - a rename, `foo = { package = "domain" }`, which mentions `domain` only as a value
//! - `[dev-dependencies]`, which would let engine's own tests import `domain` — precisely the
//!   "only for testing" rationalisation the rule exists to stop
//! - `[build-dependencies]`, coupling a build script to it
//! - `[target.'cfg(windows)'.dependencies]`, a violation that exists on one platform only
//!
//! `cargo metadata` reports the resolved package name (so renames are seen through), the
//! dependency kind, and the target, which is why the assertions below can cover all of them.

use std::collections::BTreeSet;
use std::process::Command;

use serde_json::Value;

/// The isolated crates, and the reason each one is isolated.
const ISOLATED: &[(&str, &str)] = &[
    (
        "engine",
        "The match engine defines its own mirror types (PlayerData, TeamData, Position, PlayStyle) \
         so it can be tested with synthetic data and evolved independently of the game's domain \
         model. `ofm_core/turn/` is the only place the conversion between them is allowed to live.",
    ),
    (
        "domain",
        "`domain` is the bottom of the graph: plain structs and enums that everything else may \
         depend on. If it gains a dependency, it has stopped being data and started being logic.",
    ),
];

fn metadata() -> Value {
    // `CARGO` rather than a bare "cargo": cargo sets this for the test process, and it points at
    // the toolchain currently running the tests. A bare invocation can resolve to a different one.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| String::from("cargo"));
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");

    let output = Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            manifest,
        ])
        .output()
        .expect("could not run `cargo metadata`");

    assert!(
        output.status.success(),
        "`cargo metadata` failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("`cargo metadata` emitted invalid JSON")
}

fn packages(meta: &Value) -> &Vec<Value> {
    meta["packages"]
        .as_array()
        .expect("`cargo metadata` output has no packages array")
}

/// Every crate in this workspace. `--no-deps` limits the output to workspace members, so this is
/// exactly the set a boundary rule can talk about.
fn workspace_crate_names(meta: &Value) -> BTreeSet<String> {
    packages(meta)
        .iter()
        .filter_map(|pkg| pkg["name"].as_str())
        .map(String::from)
        .collect()
}

fn package<'a>(meta: &'a Value, name: &str) -> &'a Value {
    packages(meta)
        .iter()
        .find(|pkg| pkg["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("no crate named `{name}` in this workspace"))
}

/// Describes a dependency edge well enough to fix it, including the ones a grep would miss.
fn describe(dep: &Value) -> String {
    let name = dep["name"].as_str().unwrap_or("<unnamed>");

    // null = a normal dependency; otherwise "dev" or "build".
    let kind = match dep["kind"].as_str() {
        Some(kind) => format!("[{kind}-dependencies]"),
        None => String::from("[dependencies]"),
    };

    let mut description = format!("{name} in {kind}");

    if let Some(rename) = dep["rename"].as_str() {
        description.push_str(&format!(" (declared under the alias `{rename}`)"));
    }
    if let Some(target) = dep["target"].as_str() {
        description.push_str(&format!(" (only for target `{target}`)"));
    }
    if dep["optional"].as_bool() == Some(true) {
        description.push_str(" (optional)");
    }

    description
}

/// Workspace dependencies of `crate_name`, across **all** dependency kinds.
fn workspace_dependencies_of(meta: &Value, crate_name: &str) -> Vec<String> {
    let workspace = workspace_crate_names(meta);

    package(meta, crate_name)["dependencies"]
        .as_array()
        .expect("package has no dependencies array")
        .iter()
        .filter(|dep| {
            dep["name"]
                .as_str()
                .is_some_and(|name| workspace.contains(name))
        })
        .map(describe)
        .collect()
}

#[test]
fn isolated_crates_declare_no_workspace_dependencies() {
    let meta = metadata();

    for (crate_name, rationale) in ISOLATED {
        let violations = workspace_dependencies_of(&meta, crate_name);

        assert!(
            violations.is_empty(),
            "`{crate_name}` must not depend on any other crate in this workspace, but declares:\n\
             \n  {}\n\n\
             {rationale}\n\n\
             Adding this dependency \"to avoid duplication\" is the single most damaging change \
             available in this codebase. If `engine` needs a new field, add it to the engine's own \
             type and extend the bridge in `ofm_core/turn/`.\n\
             See docs/ARCHITECTURE.md \"Engine Isolation\" and src-tauri/CLAUDE.md §1.",
            violations.join("\n  "),
        );
    }
}

#[test]
fn the_isolated_crates_still_exist() {
    // Guards the assertion above against silently passing if a crate is renamed or removed: an
    // empty violation list is also what "this crate is gone" looks like.
    let names = workspace_crate_names(&metadata());

    for (crate_name, _) in ISOLATED {
        assert!(
            names.contains(*crate_name),
            "`{crate_name}` is no longer a workspace member, so the boundary test above is \
             vacuously passing. Either restore the crate or update ISOLATED to match the new \
             architecture — deliberately, not by deleting the test.",
        );
    }
}
