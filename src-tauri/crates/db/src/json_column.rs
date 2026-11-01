//! Reading a JSON column without ever losing it.
//!
//! The save stores much of a row as JSON text (a player's career, a team's form, a message's
//! context). A cell that does not parse used to read silently as the field's default, and the
//! next save wrote that default back over it: one bad cell, and a player's whole history was gone
//! for good.
//!
//! Every JSON column is now read through [`read`] (or its siblings). A cell that does not parse
//! still reads as the default, so the career loads and plays on, but it is logged, and its text is
//! copied into `unreadable_records` (migration v063) first. The save rewrites the column; the copy
//! stays. Nothing a save does writes to that table: it is filled only here, on load, one row per
//! cell, keeping the first text seen. The bug-report diagnostics list the cells it holds, never
//! their text ([`unreadable_cells`]).

use rusqlite::{Connection, params};
use serde::de::DeserializeOwned;

/// Which cell a JSON text came from: its table, the row's id and the column.
#[derive(Debug, Clone, Copy)]
pub struct Cell<'a> {
    pub table: &'static str,
    pub row_id: &'a str,
    pub column: &'static str,
}

impl<'a> Cell<'a> {
    pub fn new(table: &'static str, row_id: &'a str, column: &'static str) -> Self {
        Self {
            table,
            row_id,
            column,
        }
    }
}

/// The cell's value, or the type's default when it does not parse (the cell then quarantined).
pub fn read<T: DeserializeOwned + Default>(conn: &Connection, cell: Cell<'_>, text: &str) -> T {
    read_or_else(conn, cell, text, T::default)
}

/// The cell's value, or None when it does not parse (the cell then quarantined): for a caller that
/// must repair what the lost cell held, rather than carry on with a default.
pub fn try_read<T: DeserializeOwned>(conn: &Connection, cell: Cell<'_>, text: &str) -> Option<T> {
    match serde_json::from_str(text) {
        Ok(value) => Some(value),
        Err(error) => {
            quarantine(conn, cell, text, &error.to_string());
            None
        }
    }
}

/// The cell's value, or `fallback()` when it does not parse (the cell then quarantined).
pub fn read_or_else<T: DeserializeOwned>(
    conn: &Connection,
    cell: Cell<'_>,
    text: &str,
    fallback: impl FnOnce() -> T,
) -> T {
    try_read(conn, cell, text).unwrap_or_else(fallback)
}

/// A nullable cell's value: None when it is NULL, and None when it does not parse (the cell then
/// quarantined).
pub fn read_optional<T: DeserializeOwned>(
    conn: &Connection,
    cell: Cell<'_>,
    text: Option<&str>,
) -> Option<T> {
    let text = text?;
    match serde_json::from_str(text) {
        Ok(value) => Some(value),
        Err(error) => {
            quarantine(conn, cell, text, &error.to_string());
            None
        }
    }
}

/// One member of a cell already read as a JSON object: its default when the object does not hold
/// it, and its default when it is there but does not read, the whole cell (`text`) then
/// quarantined.
pub fn read_member<T: DeserializeOwned + Default>(
    conn: &Connection,
    cell: Cell<'_>,
    text: &str,
    object: &serde_json::Value,
    member: &str,
) -> T {
    let Some(value) = object.get(member) else {
        return T::default();
    };
    match serde_json::from_value(value.clone()) {
        Ok(read) => read,
        Err(error) => {
            quarantine(conn, cell, text, &format!("{member}: {error}"));
            T::default()
        }
    }
}

/// Keeps a cell's text as it was, and logs it. For a cell that parsed but lost a part to a default
/// (one bad element of a map), so the whole cell can be recovered. Never fails the load: a copy
/// that cannot be written is logged as an error instead.
pub fn quarantine(conn: &Connection, cell: Cell<'_>, raw: &str, error: &str) {
    log::warn!(
        "[json_column] {}.{} of row {} did not read ({error}); its text is kept in unreadable_records",
        cell.table,
        cell.column,
        cell.row_id
    );
    let stored = conn.execute(
        "INSERT OR IGNORE INTO unreadable_records (table_name, row_id, column_name, raw, error)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![cell.table, cell.row_id, cell.column, raw, error],
    );
    if let Err(store_error) = stored {
        log::error!(
            "[json_column] could not keep {}.{} of row {}: {store_error}",
            cell.table,
            cell.column,
            cell.row_id
        );
    }
}

/// A cell kept in `unreadable_records`, as the diagnostics name it: where it is, never its text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UnreadableCell {
    pub table: String,
    pub row_id: String,
    pub column: String,
}

/// Every cell the save holds in `unreadable_records`, in the order they were found.
pub fn unreadable_cells(conn: &Connection) -> rusqlite::Result<Vec<UnreadableCell>> {
    let mut statement = conn.prepare(
        "SELECT table_name, row_id, column_name FROM unreadable_records
         ORDER BY found_at, table_name, row_id, column_name",
    )?;
    statement
        .query_map([], |row| {
            Ok(UnreadableCell {
                table: row.get(0)?,
                row_id: row.get(1)?,
                column: row.get(2)?,
            })
        })?
        .collect()
}

/// The cells a save file holds in `unreadable_records`, for the bug-report diagnostics. Read-only;
/// a save that cannot be read here (or predates the table) lists none, since a missing list is not
/// worth failing a report over, and the failure is logged.
pub fn unreadable_cells_in_save(path: &std::path::Path) -> Vec<UnreadableCell> {
    let read = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|conn| unreadable_cells(&conn));
    match read {
        Ok(cells) => cells,
        Err(error) => {
            log::warn!("[json_column] could not list the save's unreadable cells: {error}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;

    fn kept(conn: &Connection) -> Vec<(String, String, String, String)> {
        let mut statement = conn
            .prepare("SELECT table_name, row_id, column_name, raw FROM unreadable_records")
            .unwrap();
        statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    /// The text with every `#[cfg(test)]` item taken out: a module or function up to its matching
    /// brace, or a one-line item up to its `;`. Code after a test module is still production code,
    /// so it is kept and scanned.
    fn production_code(text: &str) -> String {
        const MARK: &str = "#[cfg(test)]";
        let mut kept = String::new();
        let mut rest = text;
        while let Some(at) = rest.find(MARK) {
            kept.push_str(&rest[..at]);
            let item = &rest[at + MARK.len()..];
            let end = match (item.find('{'), item.find(';')) {
                (Some(open), Some(stop)) if stop < open => stop + 1,
                (Some(open), _) => matching_close(item, open).map_or(item.len(), |close| close + 1),
                (None, Some(stop)) => stop + 1,
                (None, None) => item.len(),
            };
            rest = &item[end..];
        }
        kept.push_str(rest);
        kept
    }

    /// Where the bracket at `open` (`(` or `{`) is closed, counting the nesting in between.
    fn matching_close(text: &str, open: usize) -> Option<usize> {
        let (opening, closing) = match text[open..].chars().next()? {
            '(' => ('(', ')'),
            _ => ('{', '}'),
        };
        let mut depth = 0usize;
        for (offset, ch) in text[open..].char_indices() {
            if ch == opening {
                depth += 1;
            } else if ch == closing {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
        }
        None
    }

    /// The methods chained onto a call, read from the text just after its closing parenthesis:
    /// `.map_err(..)?.unwrap_or_default()` is `["map_err", "unwrap_or_default"]`.
    fn chained_methods(after: &str) -> Vec<String> {
        let mut methods = Vec::new();
        let mut rest = after;
        loop {
            rest = rest.trim_start().trim_start_matches('?').trim_start();
            let Some(chain) = rest.strip_prefix('.') else {
                break;
            };
            let name: String = chain
                .chars()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect();
            if name.is_empty() {
                break;
            }
            let tail = &chain[name.len()..];
            methods.push(name);
            let args = tail.len() - tail.trim_start().len();
            if !tail.trim_start().starts_with('(') {
                rest = tail;
                continue;
            }
            let Some(close) = matching_close(tail, args) else {
                break;
            };
            rest = &tail[close + 1..];
        }
        methods
    }

    /// Each production source file of the db crate but this one, its tests taken out.
    fn production_sources() -> Vec<(String, String)> {
        fn walk(dir: &std::path::Path, found: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, found);
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                    && !path.ends_with("json_column.rs")
                {
                    let text = std::fs::read_to_string(&path).unwrap();
                    found.push((path.display().to_string(), production_code(&text)));
                }
            }
        }
        let mut found = Vec::new();
        walk(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut found,
        );
        found
    }

    /// Every `serde_json::from_*` call in the given code, with the methods chained onto it.
    fn parse_calls_in(code: &str) -> Vec<Vec<String>> {
        let mut calls = Vec::new();
        let mut rest = code;
        while let Some(at) = rest.find("serde_json::from_") {
            let call = &rest[at..];
            let Some(open) = call.find('(') else { break };
            let Some(close) = matching_close(call, open) else {
                break;
            };
            calls.push(chained_methods(&call[close + 1..]));
            rest = &call[close + 1..];
        }
        calls
    }

    /// Whether a chain falls back to a value of its own on a failed parse.
    fn defaults_silently(methods: &[String]) -> bool {
        methods
            .iter()
            .any(|method| method.starts_with("unwrap_or") || method == "ok")
    }

    /// Given a fallback chained after another method, and code after a test module
    /// When the scan reads them
    /// Then it sees both: the guard below would catch either
    #[test]
    fn the_scan_reads_the_whole_chain_and_past_a_test_module() {
        let chained = "let x: Vec<u8> = serde_json::from_str(&raw)\n    .map_err(|e| e)\n    .unwrap_or_default();";
        assert!(defaults_silently(&parse_calls_in(chained)[0]));
        let loud = "serde_json::from_str(&raw).map_err(|_| KEY.to_string())?";
        assert!(!defaults_silently(&parse_calls_in(loud)[0]));

        let after_tests = "fn a() {}\n#[cfg(test)]\nmod tests { fn t() { let _ = 1; } }\nfn b() { serde_json::from_str(&raw).ok(); }";
        let code = production_code(after_tests);
        assert!(!code.contains("mod tests"), "{code}");
        assert!(code.contains("fn b()"), "{code}");
        assert!(defaults_silently(&parse_calls_in(&code)[0]));
    }

    /// Given every JSON parse in the db crate's production code
    /// When the methods chained onto each are read
    /// Then none falls back to a default on its own (`.unwrap_or…`, `.ok()`, anywhere in the
    ///      chain): a cell that does not parse either goes through this module, which keeps it, or
    ///      fails the load. A `match` on a parse is left to review: the two in team_repo quarantine
    ///      in their error arms
    #[test]
    fn no_json_column_falls_back_to_a_default_by_itself() {
        let calls: Vec<(String, Vec<String>)> = production_sources()
            .into_iter()
            .flat_map(|(path, code)| {
                parse_calls_in(&code)
                    .into_iter()
                    .map(move |methods| (path.clone(), methods))
            })
            .collect();
        assert!(
            calls.len() > 10,
            "the scan found the parses: {}",
            calls.len()
        );

        let silent: Vec<&(String, Vec<String>)> = calls
            .iter()
            .filter(|(_, methods)| defaults_silently(methods))
            .collect();

        assert!(
            silent.is_empty(),
            "a JSON column that defaults silently: {silent:?}"
        );
    }

    /// Given the db crate's production code
    /// When it is searched for the quarantine table
    /// Then only this module has SQL that touches it (the migration that creates it is a .sql
    ///      file): no save, delete or rewrite can touch a kept cell
    #[test]
    fn only_this_module_writes_the_quarantine_table() {
        // SQL that touches the table, not a comment that names it.
        let statements = [
            "INTO unreadable_records",
            "FROM unreadable_records",
            "UPDATE unreadable_records",
            "TABLE unreadable_records",
        ];
        let found: Vec<String> = production_sources()
            .into_iter()
            .filter(|(_, code)| statements.iter().any(|statement| code.contains(statement)))
            .map(|(path, _)| path)
            .collect();

        assert!(found.is_empty(), "outside json_column: {found:?}");
    }

    /// Given one cell that parses and one that does not
    /// When each is read for a caller that repairs a lost cell
    /// Then the first is its value, the second None and kept: the caller knows to repair it
    #[test]
    fn a_cell_read_to_be_repaired_says_whether_it_was_lost() {
        let db = GameDatabase::open_in_memory().unwrap();

        let fine: Option<Vec<u32>> = try_read(db.conn(), Cell::new("t", "r", "fine"), "[1]");
        let lost: Option<Vec<u32>> = try_read(db.conn(), Cell::new("t", "r", "lost"), "[1,");

        assert_eq!(fine, Some(vec![1]));
        assert_eq!(lost, None);
        assert_eq!(
            unreadable_cells(db.conn()).unwrap(),
            vec![UnreadableCell {
                table: "t".into(),
                row_id: "r".into(),
                column: "lost".into(),
            }]
        );
    }

    /// Given a cell that parses
    /// When it is read
    /// Then it reads as written, and nothing is kept
    #[test]
    fn a_cell_that_parses_reads_and_keeps_nothing() {
        let db = GameDatabase::open_in_memory().unwrap();

        let value: Vec<u32> = read(db.conn(), Cell::new("players", "p1", "career"), "[1,2]");

        assert_eq!(value, vec![1, 2]);
        assert!(kept(db.conn()).is_empty());
    }

    /// Given a cell that does not parse
    /// When it is read twice (two loads)
    /// Then it reads as the default both times, and its text is kept once, with where it came from
    #[test]
    fn a_cell_that_does_not_parse_is_kept_once() {
        let db = GameDatabase::open_in_memory().unwrap();
        let cell = Cell::new("teams", "t1", "form");

        let first: Vec<String> = read(db.conn(), cell, "[\"W\", oops");
        let second: Vec<String> = read(db.conn(), cell, "[\"W\", oops");

        assert!(first.is_empty() && second.is_empty());
        assert_eq!(
            kept(db.conn()),
            vec![(
                "teams".to_string(),
                "t1".to_string(),
                "form".to_string(),
                "[\"W\", oops".to_string()
            )]
        );
    }

    /// Given a NULL cell and an unreadable one, both nullable
    /// When each is read
    /// Then both read as None, and only the unreadable one is kept
    #[test]
    fn a_null_cell_is_none_and_not_kept() {
        let db = GameDatabase::open_in_memory().unwrap();

        let null: Option<u32> =
            read_optional(db.conn(), Cell::new("players", "p1", "injury"), None);
        let bad: Option<u32> =
            read_optional(db.conn(), Cell::new("players", "p2", "injury"), Some("{"));

        assert_eq!((null, bad), (None, None));
        assert_eq!(kept(db.conn()).len(), 1);
        assert_eq!(kept(db.conn())[0].1, "p2");
    }

    /// Given a save file with a kept cell
    /// When the diagnostics read the file
    /// Then they list it
    #[test]
    fn the_diagnostics_read_the_kept_cells_from_the_save_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.db");
        let db = GameDatabase::open(&path).unwrap();
        quarantine(
            db.conn(),
            Cell::new("players", "p1", "career"),
            "[oops",
            "eof",
        );
        drop(db);

        let cells = unreadable_cells_in_save(&path);

        assert_eq!(
            cells,
            vec![UnreadableCell {
                table: "players".into(),
                row_id: "p1".into(),
                column: "career".into(),
            }]
        );
    }

    /// Given two kept cells
    /// When the diagnostics list them
    /// Then each is named by table, row and column, and no text is listed
    #[test]
    fn the_diagnostics_name_each_kept_cell_without_its_text() {
        let db = GameDatabase::open_in_memory().unwrap();
        quarantine(
            db.conn(),
            Cell::new("players", "p1", "career"),
            "secret",
            "eof",
        );
        quarantine(db.conn(), Cell::new("teams", "t1", "form"), "secret", "eof");

        let cells = unreadable_cells(db.conn()).unwrap();

        assert_eq!(cells.len(), 2);
        assert!(cells.contains(&UnreadableCell {
            table: "players".into(),
            row_id: "p1".into(),
            column: "career".into(),
        }));
        assert!(!serde_json::to_string(&cells).unwrap().contains("secret"));
    }
}
