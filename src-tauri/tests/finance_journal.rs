//! Cash may only move through `post` / `post_all`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("could not read {}: {err}", dir.display()));
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if matches!(name, "target" | "tests") {
                continue;
            }
            rust_sources(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if name == "tests.rs" {
                continue;
            }
            out.push(path);
        }
    }
}

fn is_allowed_cash_writer(path: &Path) -> bool {
    path.ends_with(Path::new("finances").join("journal.rs"))
        || path
            .components()
            .any(|component| component == Component::Normal(OsStr::new("generator")))
        || path.ends_with("history_generation.rs")
}

fn production_lines(source: &str) -> impl Iterator<Item = &str> {
    let mut in_tests = false;
    source.lines().filter(move |line| {
        if line.contains("#[cfg(test)]") || line.contains("#![cfg(test)]") {
            in_tests = true;
        }
        !in_tests
    })
}

#[test]
fn allowed_writer_paths_are_component_aware() {
    assert!(is_allowed_cash_writer(
        &Path::new("crates")
            .join("ofm_core")
            .join("src")
            .join("finances")
            .join("journal.rs")
    ));
    assert!(is_allowed_cash_writer(
        &Path::new("crates")
            .join("ofm_core")
            .join("src")
            .join("generator")
            .join("clubs.rs")
    ));
    assert!(is_allowed_cash_writer(Path::new("history_generation.rs")));
    assert!(!is_allowed_cash_writer(
        &Path::new("crates")
            .join("ofm_core")
            .join("src")
            .join("finances")
            .join("mod.rs")
    ));
}

#[test]
fn production_code_does_not_assign_finance_with_plus_equals() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_sources(&root.join("crates/ofm_core/src"), &mut files);
    rust_sources(&root.join("src"), &mut files);

    let mut violations = Vec::new();
    for path in files {
        if is_allowed_cash_writer(&path) {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        for (index, line) in production_lines(&source).enumerate() {
            let assigns_finance = line.split_once(".finance").is_some_and(|(_, suffix)| {
                let suffix = suffix.trim_start();
                suffix.starts_with('=') && !suffix.starts_with("==")
            });
            if line.contains(".finance +=") || line.contains(".finance -=") || assigns_finance {
                violations.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "club cash must move through ofm_core::finances::post, found:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn finance_snapshot_does_not_clone_the_game() {
    let src = include_str!("../src/commands/finances.rs");
    let start = src
        .find("pub fn get_finance_snapshot_internal")
        .expect("snapshot command");
    let body = src[start..]
        .split("\n#[tauri::command]")
        .next()
        .expect("function body");
    assert!(
        !body.contains("g.clone()") && !body.contains("game.clone()"),
        "get_finance_snapshot_internal must extract via get_game, not clone Game:\n{body}"
    );
}
