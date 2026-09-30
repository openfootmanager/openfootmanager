//! A player's contract has one writer.
//!
//! A player has no wage or contract-date fields: the current contract is the latest
//! entry in `Player::movement_history`, and only `Player::record_movement` can add to
//! that (the ledger's entries are private, so the compiler holds that line).
//!
//! What the compiler cannot hold is the two ways a contract enters a player without an
//! entry yet: *staging* (world generation rolls a contract before there is any
//! history; opening a career turns it into an entry) and *restoring* a ledger from a
//! save. This reads the source and fails if anything but world generation stages, or
//! anything but the player repository restores. Test code is free to stage fixtures.

use std::fs;
use std::path::{Path, PathBuf};

const STAGERS: &[&str] = &[
    "stage_contract(",
    "stage_wage(",
    "stage_contract_start(",
    "stage_contract_end(",
];

/// Where a contract may be staged: the ledger's own module, and world generation.
const STAGING_ALLOWED: &[&str] = &[
    "crates/domain/src/contract_ledger.rs",
    "crates/ofm_core/src/generator/generation.rs",
    "crates/ofm_core/src/generator/mod.rs",
];

/// Where a ledger may be rebuilt from a save.
const RESTORE_ALLOWED: &[&str] = &[
    "crates/domain/src/contract_ledger.rs",
    "crates/db/src/repositories/player_repo.rs",
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
fn only_world_generation_stages_a_contract() {
    let (root, files) = sources();
    let mut offenders = Vec::new();
    for file in files {
        let rel = relative(&file, &root);
        if STAGING_ALLOWED.contains(&rel.as_str()) {
            continue;
        }
        let code = production_code(&file);
        for call in STAGERS {
            if code.contains(call) {
                offenders.push(format!("{rel} calls {call}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a contract made in the game is recorded with Player::record_movement; only world \
         generation stages one:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn only_the_player_repository_restores_a_ledger() {
    let (root, files) = sources();
    let mut offenders = Vec::new();
    for file in files {
        let rel = relative(&file, &root);
        if RESTORE_ALLOWED.contains(&rel.as_str()) {
            continue;
        }
        if production_code(&file).contains("MovementLedger::restore") {
            offenders.push(rel);
        }
    }
    assert!(
        offenders.is_empty(),
        "a ledger is rebuilt from a save in one place, which also decides what the old \
         contract columns mean:\n{}",
        offenders.join("\n")
    );
}
