//! A player's contract has one writer.
//!
//! The current contract is the latest entry in `Player::movement_history`, and the
//! wage and dates a player carries are a projection that `Player::record_movement`
//! keeps in step with it. That only holds if nothing else adds to the ledger or
//! writes the projection, so this reads the source and fails on anything that does.
//!
//! What may name the `stored_*` fields: `domain` (which owns them), the player
//! repository (which saves and loads them), and world generation (which has no
//! history yet, so stages the contract it rolls; opening a career turns it into an
//! entry). Test code is free to stage fixtures. Everything else reads the contract
//! through `wage()`, `contract_start()` and `contract_end()` and changes it by
//! recording a movement.

use std::fs;
use std::path::{Path, PathBuf};

const STAGING_ALLOWED: &[&str] = &[
    "crates/domain/src/player.rs",
    "crates/domain/src/contract_ledger.rs",
    "crates/db/src/repositories/player_repo.rs",
    "crates/ofm_core/src/generator/generation.rs",
    "crates/ofm_core/src/generator/mod.rs",
];

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // `tests` directories are test code; `target` is build output.
            if name != "target" && name != "tests" {
                rust_sources(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The non-test part of a file: a whole `tests.rs` is test code, and in any other
/// file everything from its `#[cfg(test)] mod` onwards is.
fn production_code(path: &Path) -> String {
    if path.file_name().is_some_and(|name| name == "tests.rs") {
        return String::new();
    }
    let text = fs::read_to_string(path).unwrap_or_default();
    // Only the test *module*: an earlier `#[cfg(test)]` item (a helper, an import)
    // must not hide the production code that follows it.
    let mut rest = text.as_str();
    let mut offset = 0;
    while let Some(at) = rest.find("#[cfg(test)]") {
        let after = &rest[at + "#[cfg(test)]".len()..];
        // Other attributes may sit between the gate and the `mod`.
        let mut item = after.trim_start();
        while item.starts_with("#[") {
            item = item
                .split_once('\n')
                .map_or("", |(_, rest)| rest)
                .trim_start();
        }
        if item.starts_with("mod ") {
            return text[..offset + at].to_string();
        }
        offset += at + "#[cfg(test)]".len();
        rest = after;
    }
    text
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn sources() -> (PathBuf, Vec<PathBuf>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_sources(&root.join("crates"), &mut files);
    rust_sources(&root.join("src"), &mut files);
    (root, files)
}

#[test]
fn only_staging_code_names_the_stored_contract_fields() {
    let (root, files) = sources();
    let mut offenders = Vec::new();
    for file in files {
        let rel = relative(&file, &root);
        if STAGING_ALLOWED.contains(&rel.as_str()) {
            continue;
        }
        let code = production_code(&file);
        for name in [
            "stored_wage",
            "stored_contract_start",
            "stored_contract_end",
        ] {
            if code.contains(name) {
                offenders.push(format!("{rel} names {name}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "read the contract through wage()/contract_start()/contract_end() and change it with \
         Player::record_movement:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn nothing_but_the_ledger_method_pushes_to_a_movement_history() {
    let (root, files) = sources();
    let mut offenders = Vec::new();
    for file in files {
        let rel = relative(&file, &root);
        if rel == "crates/domain/src/contract_ledger.rs" {
            continue;
        }
        if production_code(&file).contains("movement_history.push") {
            offenders.push(rel);
        }
    }
    assert!(
        offenders.is_empty(),
        "every entry must go through Player::record_movement, which keeps the contract in \
         step with the ledger:\n{}",
        offenders.join("\n")
    );
}
