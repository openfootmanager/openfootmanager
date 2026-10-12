//! A player's contract has one writer.
//!
//! A player has no wage or contract-date fields: the current contract is the latest
//! entry in `Player::movement_history`, and only `Player::record_movement` can add to
//! that (the ledger's entries are private, so the compiler holds that line).
//!
//! The compiler already refuses `player.movement_history.push(..)` outside `domain`, so
//! a third test holds the rest of that line in the source as well, for the things it
//! does not refuse: replacing the whole ledger, or borrowing it mutably.
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

/// Methods that change a list. Any of them on a movement history, outside the ledger's
/// own module, is a writer that bypassed `Player::record_movement`.
const MUTATORS: &[&str] = &[
    "push", "extend", "insert", "append", "splice", "retain", "drain", "clear", "truncate",
    "remove", "swap", "sort", "pop", "resize", "dedup", "reverse", "rotate", "fill",
];

/// Where a ledger may be mutated: its own module. The player repository is let through
/// only because its upsert SQL names the column (`movement_history = excluded...`);
/// it builds a ledger with `MovementLedger::restore` and never changes one.
const LEDGER_ALLOWED: &[&str] = &[
    "crates/domain/src/contract_ledger.rs",
    "crates/db/src/repositories/player_repo.rs",
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

/// The non-test part of a file: a whole `tests.rs` or `test_fixtures.rs` is test code, and
/// in any other file everything from its inline `#[cfg(test)] mod name { .. }` onwards is.
fn production_code(path: &Path) -> String {
    if path
        .file_name()
        .is_some_and(|name| name == "tests.rs" || name == "test_fixtures.rs")
    {
        return String::new();
    }
    production_source(&fs::read_to_string(path).unwrap_or_default())
}

/// [`production_code`] for a file's text.
///
/// Only an inline test *module* ends the production code. An earlier `#[cfg(test)]` item
/// (a helper, an import) must not hide what follows it, and neither must a file-backed
/// `#[cfg(test)] mod tests;`, which declares test code that lives in another file and
/// can appear anywhere in the file.
fn production_source(text: &str) -> String {
    let mut rest = text;
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
        if let Some(declaration) = item.strip_prefix("mod ") {
            let after_name = declaration
                .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_')
                .trim_start();
            if after_name.starts_with('{') {
                return text[..offset + at].to_string();
            }
        }
        offset += at + "#[cfg(test)]".len();
        rest = after;
    }
    text.to_string()
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

/// Every way the source can change a `movement_history` other than through the
/// ledger: a mutating method, an assignment, or a `&mut` borrow.
fn ledger_mutations(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = code[from..].find("movement_history") {
        let start = from + at;
        let end = start + "movement_history".len();
        from = end;
        let after: String = code[end..]
            .chars()
            .filter(|c| !c.is_whitespace())
            .take(24)
            .collect();
        for method in MUTATORS {
            if after.starts_with(&format!(".{method}(")) {
                found.push(format!("calls .{method}(..)"));
            }
        }
        if after.starts_with('=') && !after.starts_with("==") {
            found.push("assigns it".to_string());
        }
        // `&mut x.movement_history`: walk back over the path to the borrow.
        let path_start = code[..start]
            .rfind(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.' | '[' | ']')))
            .map_or(0, |i| i + 1);
        if code[..path_start].trim_end().ends_with("&mut") {
            found.push("borrows it mutably".to_string());
        }
    }
    found
}

#[test]
fn nothing_outside_the_ledger_mutates_a_movement_history() {
    let (root, files) = sources();
    let mut offenders = Vec::new();
    for file in files {
        let rel = relative(&file, &root);
        if LEDGER_ALLOWED.contains(&rel.as_str()) {
            continue;
        }
        for what in ledger_mutations(&production_code(&file)) {
            offenders.push(format!("{rel} {what}"));
        }
    }
    assert!(
        offenders.is_empty(),
        "a movement history changes only through Player::record_movement:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_mutation_scan_recognises_what_it_is_meant_to_refuse() {
    for code in [
        "p.movement_history.push(e);",
        "p.movement_history\n    .extend(more);",
        "p.movement_history = Default::default();",
        "let h = &mut p.movement_history;",
        "std::mem::take(&mut player.movement_history)",
        "player.movement_history.retain(|e| keep(e));",
    ] {
        assert!(
            !ledger_mutations(code).is_empty(),
            "missed a mutation: {code}"
        );
    }
    for code in [
        "p.movement_history.iter().any(|e| e.contract.is_some());",
        "if p.movement_history == other.movement_history {}",
        "let n = p.movement_history.len();",
        "movement_history: MovementLedger::restore(a, b, c, 0),",
    ] {
        assert!(ledger_mutations(code).is_empty(), "flagged a read: {code}");
    }
}

#[test]
fn production_code_ends_only_at_an_inline_test_module() {
    // An inline test module ends it, attributes between the gate and the module or not.
    assert_eq!(
        production_source("fn a() {}\n#[cfg(test)]\nmod tests {\n fn t() {}\n}\n"),
        "fn a() {}\n"
    );
    assert_eq!(
        production_source("fn a() {}\n#[cfg(test)]\n#[allow(dead_code)]\nmod tests {}\n"),
        "fn a() {}\n"
    );
    // A test-only helper or import does not hide what follows it.
    let with_helper = "#[cfg(test)]\nfn helper() {}\nfn production() { p.stage_wage(1); }\n";
    assert!(production_source(with_helper).contains("stage_wage"));
    // A file-backed test module is declared elsewhere: what follows it is production code.
    let file_backed = "fn a() {}\n#[cfg(test)]\nmod tests;\nfn production() { p.stage_wage(1); }\n";
    assert!(
        production_source(file_backed).contains("stage_wage"),
        "a `mod tests;` declaration hid the production code after it"
    );
}
