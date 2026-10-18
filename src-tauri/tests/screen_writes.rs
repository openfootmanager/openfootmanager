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

/// The only functions that may write the active game without naming a career: `(call, file,
/// enclosing fn, why)`. Scoped to the function, so a second `set_game` added elsewhere in
/// `application/career.rs` is still caught.
const ALLOWED: &[(&str, &str, &str, &str)] = &[
    (
        "set_game",
        "src/application/career.rs",
        "install_career",
        "where a game becomes the active career, under the career lock",
    ),
    (
        "clear_game",
        "src/application/saving.rs",
        "exit_to_menu",
        "closes the active career after saving it",
    ),
];

/// The calls that write the active game. Only the game: the live match session and the stats are
/// separate state (`set_live_match`, `take_live_match`, `append_stats_state`, ...) and are not
/// covered here.
const WRITES: &[&str] = &["update_game", "set_game", "clear_game"];

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
    matches!(
        (chars.get(at + 1), chars.get(at + 2)),
        (Some('\\'), _) | (Some(_), Some('\''))
    )
}

/// Removes `#[cfg(test)] mod name { .. }` blocks. An out-of-line `#[cfg(test)] mod name;` removes
/// only that statement, never the item after it. A `#[cfg(test)]` on anything else drops just the
/// attribute, so the item is scanned: the safe error is a false alarm, not a hidden call.
fn remove_test_modules(source: &str) -> String {
    const ATTRIBUTE: &str = "#[cfg(test)]";
    let mut out = String::new();
    let mut rest = source;
    while let Some(at) = rest.find(ATTRIBUTE) {
        out.push_str(&rest[..at]);
        let after = &rest[at + ATTRIBUTE.len()..];
        let item = after.trim_start();
        let item = strip_visibility(item);
        let Some(named) = item.strip_prefix("mod ") else {
            rest = after;
            continue;
        };
        let name_end = named
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(named.len());
        let tail = named[name_end..].trim_start();
        let consumed = |remaining: &str| after.len() - remaining.len();
        if let Some(body) = tail.strip_prefix('{') {
            let mut depth = 1usize;
            let mut end = body.len();
            for (offset, c) in body.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = offset + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            rest = &after[consumed(&body[end..])..];
        } else if let Some(after_semicolon) = tail.strip_prefix(';') {
            rest = &after[consumed(after_semicolon)..];
        } else {
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn strip_visibility(item: &str) -> &str {
    let Some(rest) = item.strip_prefix("pub") else {
        return item;
    };
    let rest = rest.trim_start();
    match rest.strip_prefix('(') {
        Some(inner) => inner
            .split_once(')')
            .map_or(item, |(_, after)| after.trim_start()),
        None => rest,
    }
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

/// The name of the function whose body contains byte `at` of `code`, innermost first.
fn enclosing_fn(code: &str, at: usize) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for (start, _) in code.match_indices("fn ") {
        let before = code[..start].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let name: String = code[start + 3..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let Some(open) = code[start..].find('{').map(|n| start + n) else {
            continue;
        };
        let mut depth = 0usize;
        let mut end = code.len();
        for (offset, c) in code[open..].char_indices() {
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
        if open < at && at < end && best.as_ref().is_none_or(|(s, _)| start > *s) {
            best = Some((start, name));
        }
    }
    best.map(|(_, name)| name)
}

/// Every direct write of the active game in `code` (a file's production code) that is not one of
/// the scoped exceptions, as `file: call (in fn)`. A call is `x.call(` or `Type::call(`.
fn direct_writes(file: &str, code: &str, allowed: &[(&str, &str, &str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    for call in WRITES {
        let needle = format!("{call}(");
        for (at, _) in code.match_indices(&needle) {
            let before = &code[..at];
            if !(before.ends_with('.') || before.ends_with("::")) {
                continue;
            }
            let function = enclosing_fn(code, at).unwrap_or_default();
            let permitted = allowed
                .iter()
                .any(|(allowed_call, allowed_file, allowed_fn, _)| {
                    allowed_call == call && *allowed_file == file && *allowed_fn == function
                });
            if !permitted {
                found.push(format!("{file}: {call} (in fn {function})"));
            }
        }
    }
    found
}

/// Given the root crate's production code
/// When it is searched for direct writes of the active game
/// Then `update_game`, `set_game` and `clear_game` appear only in the functions that install or
///      close a career: everything else goes through `update_game_for`, which names a career and
///      checks it.
#[test]
fn no_production_code_writes_the_active_game_without_naming_a_career() {
    let offenders: Vec<String> = production_sources()
        .iter()
        .flat_map(|(file, code)| direct_writes(file, code, ALLOWED))
        .collect();
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

/// Given production code that calls the writes in ways a plain `.call(` search misses
/// When the scanner reads it
/// Then each way is found: a qualified call, a call after an out-of-line test module, and a second
///      call in a file that has an allowed one, while comments, strings and test modules are not.
#[test]
fn the_scanner_finds_every_way_to_write_and_ignores_what_is_not_code() {
    let allowed = [("set_game", "a.rs", "install", "the installer")];
    let scan = |source: &str| direct_writes("a.rs", &production_code(source), &allowed);

    // A qualified call.
    assert_eq!(
        scan("fn f(s: &S) { StateManager::update_game(s, g); }").len(),
        1
    );
    // An out-of-line test module must not swallow the next braced item.
    assert_eq!(
        scan("#[cfg(test)]\nmod helpers;\nfn f(s: &S) { s.update_game(g); }").len(),
        1
    );
    assert_eq!(
        scan("#[cfg(test)]\npub(crate) mod helpers;\nfn f(s: &S) { s.clear_game(); }").len(),
        1
    );
    // An inline test module is removed, and code after it is still scanned.
    let after_tests = scan(
        "#[cfg(test)]\nmod tests { fn t(s: &S) { s.set_game(g); let c = '{'; } }\nfn f(s: &S) { s.clear_game(); }",
    );
    assert_eq!(after_tests, ["a.rs: clear_game (in fn f)"]);
    // Comments and strings are not code.
    assert!(scan("// s.set_game(g)\nfn q() -> &'static str { \"s.clear_game()\" }").is_empty());
}

/// Given an exception for `set_game` in `install`
/// When the same file calls it from another function, or calls a write the exception does not name
/// Then only the call in `install` is allowed.
#[test]
fn an_exception_is_scoped_to_its_function() {
    let allowed = [("set_game", "a.rs", "install", "the installer")];
    let code = production_code(
        "fn install(s: &S) { s.set_game(g); }\nfn other(s: &S) { s.set_game(g); s.update_game(f); }",
    );

    let found = direct_writes("a.rs", &code, &allowed);

    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|f| f.contains("in fn other")));
    // And the same call in the same function of another file is not excused.
    assert_eq!(direct_writes("b.rs", &code, &allowed).len(), 3);
}
