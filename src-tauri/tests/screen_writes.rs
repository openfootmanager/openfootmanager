//! The career check for screen writes is only as good as its coverage, so the way round it is
//! closed mechanically.
//!
//! A screen action that changes the active game carries the career token, and
//! `StateManager::update_game_for` refuses a token that is not the active career's, inside the
//! write's lock. That holds only while nothing else writes the active game: `StateManager::update_game`,
//! `set_game` and `clear_game` are `pub` (the `ofm_core` crate and the other workspace crates need
//! them), so visibility cannot hide them from this crate. These tests read the root crate's
//! production source and fail on a direct call outside the places that install or close a career.

use std::fs;
use std::path::{Path, PathBuf};

/// Production code that may write the active game without naming a career, and why.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "set_game",
        "src/application/career.rs",
        "install_career is where a game becomes the active career, under the career lock",
    ),
    (
        "clear_game",
        "src/application/saving.rs",
        "exit_to_menu closes the active career after saving it",
    ),
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The source with comments, string and char literals blanked and `#[cfg(test)]` modules removed,
/// so what is left is production code a call can be found in.
fn production_code(source: &str) -> String {
    let blanked = blank_comments_and_literals(source);
    remove_test_modules(&blanked)
}

fn blank_comments_and_literals(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            out.push_str("\"\"");
        } else if c == 'r' && matches!(next, Some('"') | Some('#')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                j += 1;
                loop {
                    if j >= chars.len() {
                        break;
                    }
                    if chars[j] == '"' && (0..hashes).all(|h| chars.get(j + 1 + h) == Some(&'#')) {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                i = j;
                out.push_str("\"\"");
            } else {
                out.push(c);
                i += 1;
            }
        } else if c == '\'' && is_char_literal(&chars, i) {
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            out.push_str("' '");
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `'x'` or `'\n'` is a char literal; `'a` (a lifetime) is not.
fn is_char_literal(chars: &[char], at: usize) -> bool {
    match (chars.get(at + 1), chars.get(at + 2)) {
        (Some('\\'), _) => true,
        (Some(_), Some('\'')) => true,
        _ => false,
    }
}

fn remove_test_modules(source: &str) -> String {
    let mut out = String::new();
    let mut rest = source;
    while let Some(at) = rest.find("#[cfg(test)]") {
        out.push_str(&rest[..at]);
        let after = &rest[at + "#[cfg(test)]".len()..];
        match after.find('{') {
            Some(open) if after[..open].contains("mod ") => {
                let mut depth = 0usize;
                let mut end = after.len();
                for (offset, c) in after[open..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = open + offset + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                rest = &after[end..];
            }
            // A `#[cfg(test)]` on a single item (a `use`, a fn): drop through its end.
            _ => {
                let end = after.find(['\n']).map_or(after.len(), |n| n + 1);
                rest = &after[end..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn production_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .expect("under the manifest dir")
                .to_string_lossy()
                .replace('\\', "/");
            let source = fs::read_to_string(&path).expect("read source");
            (relative, production_code(&source))
        })
        // Files that exist only as `#[cfg(test)] mod tests;` / `mod testkit;`.
        .filter(|(file, _)| !file.ends_with("/tests.rs") && !file.ends_with("/testkit.rs"))
        .collect()
}

/// Given the root crate's production code
/// When it is searched for direct writes of the active game
/// Then `update_game`, `set_game` and `clear_game` appear only where a career is installed or
///      closed: everything else goes through `update_game_for`, which names a career and checks it.
#[test]
fn no_production_code_writes_the_active_game_without_naming_a_career() {
    let mut offenders = Vec::new();
    for (file, code) in production_sources() {
        for call in ["update_game", "set_game", "clear_game"] {
            let needle = format!(".{call}(");
            let allowed = ALLOWED.iter().any(|(allowed_call, allowed_file, _)| {
                *allowed_call == call && *allowed_file == file
            });
            if code.contains(&needle) && !allowed {
                offenders.push(format!("{file}: .{call}("));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "direct writes of the active game, bypassing the career check: {offenders:#?}"
    );
}

/// Given the commands a screen calls
/// When their bodies are searched for `Career::Active`
/// Then none uses it: a screen always writes for the career it was shown, and `Active` is for
///      agents and in-process callers that act on whichever career is open.
#[test]
fn no_tauri_command_writes_for_whichever_career_is_active() {
    let mut offenders = Vec::new();
    for (file, code) in production_sources() {
        if !file.starts_with("src/commands/") {
            continue;
        }
        let mut rest = code.as_str();
        while let Some(at) = rest.find("#[tauri::command]") {
            let after = &rest[at..];
            let Some(open) = after.find('{') else { break };
            let mut depth = 0usize;
            let mut end = after.len();
            for (offset, c) in after[open..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = open + offset + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if after[..end].contains("Career::Active") {
                let name = after[..open]
                    .split("fn ")
                    .nth(1)
                    .and_then(|tail| tail.split('(').next())
                    .unwrap_or("?")
                    .trim()
                    .to_string();
                offenders.push(format!("{file}: {name}"));
            }
            rest = &after[end..];
        }
    }
    assert!(
        offenders.is_empty(),
        "screen commands that write for whichever career is active: {offenders:#?}"
    );
}

/// The scanner itself: the two helpers must find a call in production code and ignore the same
/// text in a comment, a string and a test module, or the tests above prove nothing.
#[test]
fn the_scanner_sees_production_calls_and_ignores_comments_strings_and_test_modules() {
    let source = r#"
        fn real(state: &S) { state.update_game(|g| g.x = 1); }
        // state.set_game(g)
        fn quoted() -> &'static str { "state.clear_game()" }
        #[cfg(test)]
        mod tests { fn t(state: &S) { state.set_game(game()); let c = '{'; } }
        fn after_tests(state: &S) { state.clear_game(); }
    "#;

    let code = production_code(source);

    assert!(code.contains(".update_game("));
    assert!(
        code.contains(".clear_game("),
        "code after a test module is production code"
    );
    assert!(
        !code.contains(".set_game("),
        "comments and test modules are ignored: {code}"
    );
    assert_eq!(
        code.matches(".clear_game(").count(),
        1,
        "a string is ignored"
    );
}
