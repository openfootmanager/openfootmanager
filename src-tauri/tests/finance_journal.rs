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

fn brace_delta(line: &str) -> i32 {
    line.chars()
        .map(|ch| match ch {
            '{' => 1,
            '}' => -1,
            _ => 0,
        })
        .sum()
}

fn production_lines(source: &str) -> Vec<(usize, &str)> {
    if source.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("#![cfg(test)]")
    }) {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut skip_depth: Option<i32> = None;
    let mut pending_cfg = false;
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(depth) = skip_depth.as_mut() {
            *depth += brace_delta(line);
            if *depth <= 0 {
                skip_depth = None;
            }
            continue;
        }
        if pending_cfg {
            if trimmed.starts_with("#[") {
                continue;
            }
            pending_cfg = false;
            if trimmed.contains('{') {
                let depth = brace_delta(line);
                if depth > 0 {
                    skip_depth = Some(depth);
                }
                continue;
            }
            continue;
        }
        if trimmed.contains("#[cfg(test)]") && !trimmed.starts_with("//") {
            if trimmed.contains('{') {
                let depth = brace_delta(line);
                if depth > 0 {
                    skip_depth = Some(depth);
                }
            } else if !trimmed.ends_with(';') {
                pending_cfg = true;
            }
            continue;
        }
        out.push((index, line));
    }
    out
}

fn line_writes_finance(line: &str) -> bool {
    let line = line.split("//").next().unwrap_or(line);
    if line.contains(".finance +=") || line.contains(".finance -=") {
        return true;
    }
    if line.contains("&mut") && line.contains(".finance") {
        return true;
    }
    line.split_once(".finance").is_some_and(|(_, suffix)| {
        let suffix = suffix.trim_start();
        suffix.starts_with('=') && !suffix.starts_with("==")
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
fn production_lines_resume_after_a_test_module() {
    let src = r#"
fn keep() { team.finance += 1; }
#[cfg(test)]
mod tests {
    fn hidden() { team.finance -= 1; }
}
fn also_keep() { team.finance = 0; }
#[cfg(test)]
use super::fixture;
fn after_use() {
    let cash = &mut team.finance;
    *cash += 1;
}
"#;
    let production: Vec<&str> = production_lines(src)
        .into_iter()
        .map(|(_, line)| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    assert!(production.iter().any(|line| line.contains("fn keep()")));
    assert!(production
        .iter()
        .any(|line| line.contains("fn also_keep()")));
    assert!(production
        .iter()
        .any(|line| line.contains("fn after_use()")));
    assert!(!production.iter().any(|line| line.contains("hidden()")));
    assert!(!production
        .iter()
        .any(|line| line.contains("use super::fixture")));

    let writes: Vec<&str> = production_lines(src)
        .into_iter()
        .filter(|(_, line)| line_writes_finance(line))
        .map(|(_, line)| line.trim())
        .collect();
    assert_eq!(
        writes,
        vec![
            "fn keep() { team.finance += 1; }",
            "fn also_keep() { team.finance = 0; }",
            "let cash = &mut team.finance;",
        ]
    );
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
        for (index, line) in production_lines(&source) {
            if line_writes_finance(line) {
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
