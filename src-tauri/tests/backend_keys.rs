//! Every translation key the shipped backend can hand the UI exists in `en.json`.
//!
//! The frontend's gates (`localeCoverage.test.ts`, `frontendKeyCoverage.test.ts`) start from the
//! keys `en.json` already has, so a key that Rust emits and nobody added is invisible to them.
//! The player then sees the raw key, for example `be.msg.delegatedRenewals.notes.markedLetExpire`
//! in an assistant report. This test reads the Rust source instead.
//!
//! # What is scanned
//!
//! The module tree of each shipped crate, from its root (`lib.rs`, `main.rs`), following
//! `mod x;` and `#[path]` the way rustc does. An item that only exists under test — `#[test]`,
//! `#[cfg(test)]`, `#[cfg(all(test, ..))]` — is skipped, and so is every file only such a
//! module declares. Attributes are skipped too, so a key named in a doc comment does not count.
//!
//! Not scanned, on purpose. CLAUDE.md's i18n rule lists these surfaces as read by an agent or a
//! modder, not a player, so English is correct there:
//! - `src/mcp_server/`: MCP tool output (`be.error.mcp.*` keys are rendered to English for the
//!   agent by `formatting.rs`);
//! - the `ofm-cli`, `mcp-results`, `mcp-e2e`, `season-harness` and `sim-bench` crates.
//!
//! An MCP tool that propagates a shared `application::` error still gets that key checked: the
//! key is written in `application/`, which is scanned.
//!
//! # What counts as a key, and when it passes
//!
//! Any string literal, inside a macro or not, that starts with `be.` or `common.` and contains
//! only key characters.
//! - A plain key passes when `en.json` has it, as a leaf or as the base of a plural (`_one`,
//!   `_other`, ...), or as an object the code appends to (`shortage_message_parts` adds
//!   `.subject` and `.body` to `be.msg.squadBelowFloor`).
//! - A literal ending in `.` is a prefix, and passes when some key starts with it.
//! - A format string such as `be.news.roundup.headline{}` passes when at least one key matches
//!   it, with each placeholder standing for one or more characters. That proves the family
//!   exists; it does not prove every variant the code can pick exists.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use syn::visit::{self, Visit};

const EN_JSON: &str = include_str!("../../src/i18n/locales/en.json");

/// The roots of the crates whose text reaches a player, relative to `src-tauri/`.
const SHIPPED_CRATE_ROOTS: &[&str] = &[
    "src/lib.rs",
    "src/main.rs",
    "crates/db/src/lib.rs",
    "crates/domain/src/lib.rs",
    "crates/engine/src/lib.rs",
    "crates/ofm_core/src/lib.rs",
];

/// Module files inside a shipped crate whose output is not a player's, relative to `src-tauri/`.
const NOT_PLAYER_FACING: &[&str] = &["src/mcp_server.rs", "src/mcp_server/mod.rs"];

const PLURAL_SUFFIXES: &[&str] = &["_zero", "_one", "_two", "_few", "_many", "_other"];

/// A key-shaped string literal and the file it was written in.
#[derive(Debug, PartialEq, Eq)]
struct Emitted {
    file: PathBuf,
    key: String,
}

/// Collects the key literals of one file, and the files its `mod x;` declarations load.
struct FileScan {
    /// The directory a `mod x;` here resolves against: rustc's rule for this file and the
    /// inline modules entered so far.
    module_dir: PathBuf,
    /// The directory of the file itself, which `#[path]` resolves against.
    file_dir: PathBuf,
    keys: Vec<String>,
    children: Vec<PathBuf>,
}

impl FileScan {
    fn new(file: &Path) -> Self {
        let file_dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
        let owns_its_directory = matches!(
            file.file_name().and_then(|name| name.to_str()),
            Some("lib.rs" | "main.rs" | "mod.rs")
        );
        let module_dir = if owns_its_directory {
            file_dir.clone()
        } else {
            file_dir.join(file.file_stem().unwrap_or_default())
        };
        Self {
            module_dir,
            file_dir,
            keys: Vec::new(),
            children: Vec::new(),
        }
    }

    fn record(&mut self, text: String) {
        if looks_like_a_key(&text) {
            self.keys.push(text);
        }
    }

    fn record_tokens(&mut self, tokens: TokenStream) {
        for token in tokens {
            match token {
                TokenTree::Literal(literal) => {
                    if let syn::Lit::Str(text) = syn::Lit::new(literal) {
                        self.record(text.value());
                    }
                }
                TokenTree::Group(group) => self.record_tokens(group.stream()),
                TokenTree::Ident(_) | TokenTree::Punct(_) => {}
            }
        }
    }

    fn declare_module(&mut self, module: &syn::ItemMod) {
        let name = module.ident.to_string();
        if let Some(path) = path_attribute(&module.attrs) {
            self.children.push(self.file_dir.join(path));
            return;
        }
        let flat = self.module_dir.join(format!("{name}.rs"));
        let nested = self.module_dir.join(&name).join("mod.rs");
        self.children.push(if flat.exists() || !nested.exists() {
            flat
        } else {
            nested
        });
    }
}

impl<'ast> Visit<'ast> for FileScan {
    // Doc comments are `#[doc = ".."]` and lint reasons are `reason = ".."`: text, not output.
    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {}

    fn visit_item(&mut self, item: &'ast syn::Item) {
        if only_under_test(item_attributes(item)) {
            return;
        }
        if let syn::Item::Mod(module) = item {
            match &module.content {
                None => self.declare_module(module),
                Some(_) => {
                    let outer = self.module_dir.clone();
                    self.module_dir = outer.join(module.ident.to_string());
                    visit::visit_item_mod(self, module);
                    self.module_dir = outer;
                }
            }
            return;
        }
        visit::visit_item(self, item);
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        let attributes = match item {
            syn::ImplItem::Const(constant) => &constant.attrs,
            syn::ImplItem::Fn(function) => &function.attrs,
            syn::ImplItem::Type(alias) => &alias.attrs,
            syn::ImplItem::Macro(invocation) => &invocation.attrs,
            _ => return visit::visit_impl_item(self, item),
        };
        if !only_under_test(attributes) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
        self.record(literal.value());
    }

    // A macro's body is tokens, not syntax, so `format!("be.x{}", n)` is read token by token.
    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        self.record_tokens(invocation.tokens.clone());
    }
}

fn item_attributes(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::ForeignMod(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

/// `#[test]`, `#[cfg(test)]` or `#[cfg(all(test, ..))]`. `#[cfg(any(test, ..))]` ships.
fn only_under_test(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if attribute.path().is_ident("test") {
            return true;
        }
        if !attribute.path().is_ident("cfg") {
            return false;
        }
        let Ok(list) = attribute.meta.require_list() else {
            return false;
        };
        let predicate: String = list.tokens.to_string().split_whitespace().collect();
        predicate == "test"
            || predicate.strip_prefix("all(").is_some_and(|inner| {
                inner
                    .trim_end_matches(')')
                    .split(',')
                    .any(|term| term == "test")
            })
    })
}

fn path_attribute(attributes: &[syn::Attribute]) -> Option<String> {
    attributes.iter().find_map(|attribute| {
        let syn::Meta::NameValue(pair) = &attribute.meta else {
            return None;
        };
        if !pair.path.is_ident("path") {
            return None;
        }
        match &pair.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(path),
                ..
            }) => Some(path.value()),
            _ => None,
        }
    })
}

fn looks_like_a_key(text: &str) -> bool {
    (text.starts_with("be.") || text.starts_with("common."))
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '{' | '}' | ':'))
}

/// The key literals of one source file, and the module files it declares.
fn scan_source(file: &Path, source: &str) -> (Vec<String>, Vec<PathBuf>) {
    let syntax = syn::parse_file(source)
        .unwrap_or_else(|error| panic!("{} does not parse: {error}", file.display()));
    let mut scan = FileScan::new(file);
    scan.visit_file(&syntax);
    (scan.keys, scan.children)
}

/// Every key literal in the module trees under `roots`, skipping the `excluded` module files.
fn emitted_keys(roots: &[PathBuf], excluded: &[PathBuf]) -> Vec<Emitted> {
    let mut pending: Vec<PathBuf> = roots.to_vec();
    let mut visited = BTreeSet::new();
    let mut emitted = Vec::new();
    while let Some(file) = pending.pop() {
        if excluded.contains(&file) || !visited.insert(file.clone()) {
            continue;
        }
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        let (keys, children) = scan_source(&file, &source);
        emitted.extend(keys.into_iter().map(|key| Emitted {
            file: file.clone(),
            key,
        }));
        pending.extend(children);
    }
    emitted
}

/// The keys of a locale file, as dotted paths to its leaves.
fn locale_keys(json: &str) -> BTreeSet<String> {
    fn walk(prefix: &str, node: &serde_json::Value, keys: &mut BTreeSet<String>) {
        if let serde_json::Value::Object(entries) = node {
            for (name, child) in entries {
                let path = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                walk(&path, child, keys);
            }
        } else {
            keys.insert(prefix.to_string());
        }
    }
    let root: serde_json::Value = serde_json::from_str(json).expect("en.json is not valid JSON");
    let mut keys = BTreeSet::new();
    walk("", &root, &mut keys);
    keys
}

fn has_plain_key(keys: &BTreeSet<String>, key: &str) -> bool {
    keys.contains(key)
        || PLURAL_SUFFIXES
            .iter()
            .any(|suffix| keys.contains(&format!("{key}{suffix}")))
        || keys.iter().any(|known| {
            known
                .strip_prefix(key)
                .is_some_and(|rest| rest.starts_with('.'))
        })
}

/// `be.x{}.y{n}` as the literal pieces between its placeholders: `["be.x", ".y", ""]`.
fn format_pieces(format: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut rest = format;
    while let Some(open) = rest.find('{') {
        pieces.push(&rest[..open]);
        rest = rest[open..]
            .find('}')
            .map_or("", |close| &rest[open + close + 1..]);
    }
    pieces.push(rest);
    pieces
}

/// Whether `key` is the format string with each placeholder replaced by one or more characters.
fn matches_format(pieces: &[&str], key: &str) -> bool {
    let Some((first, rest)) = pieces.split_first() else {
        return false;
    };
    let Some(mut remainder) = key.strip_prefix(first) else {
        return false;
    };
    for (index, piece) in rest.iter().enumerate() {
        let is_last = index + 1 == rest.len();
        if is_last {
            return remainder.len() > piece.len() && remainder.ends_with(piece);
        }
        // At least one character for the placeholder before this piece.
        let Some(found) = remainder.get(1..).and_then(|tail| tail.find(piece)) else {
            return false;
        };
        remainder = &remainder[1 + found + piece.len()..];
    }
    true
}

fn is_known(keys: &BTreeSet<String>, emitted: &str) -> bool {
    if emitted.contains('{') {
        let pieces = format_pieces(emitted);
        return keys.iter().any(|key| matches_format(&pieces, key));
    }
    if emitted.ends_with('.') {
        return keys.iter().any(|key| key.starts_with(emitted));
    }
    has_plain_key(keys, emitted)
}

fn missing_keys(emitted: &[Emitted], keys: &BTreeSet<String>) -> Vec<String> {
    let missing: BTreeSet<String> = emitted
        .iter()
        .filter(|found| !is_known(keys, &found.key))
        .map(|found| format!("{}: {}", found.file.display(), found.key))
        .collect();
    missing.into_iter().collect()
}

fn src_tauri() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Given the shipped crates' source and `en.json`,
/// when every key literal outside test code is collected,
/// then each one exists in `en.json`, so no player ever sees a raw key from the backend.
#[test]
fn every_backend_key_a_player_can_see_exists_in_en_json() {
    let root = src_tauri();
    let roots: Vec<PathBuf> = SHIPPED_CRATE_ROOTS
        .iter()
        .map(|path| root.join(path))
        .collect();
    let excluded: Vec<PathBuf> = NOT_PLAYER_FACING
        .iter()
        .map(|path| root.join(path))
        .collect();

    let emitted = emitted_keys(&roots, &excluded);
    let missing = missing_keys(&emitted, &locale_keys(EN_JSON));

    // A walk that found nothing would pass vacuously.
    assert!(
        emitted.len() > 500,
        "only {} key literals found: is the module walk broken?",
        emitted.len()
    );
    assert!(
        missing.is_empty(),
        "{} backend key(s) are not in src/i18n/locales/en.json. Add each to every locale \
         (/add-ui-string):\n{}",
        missing.len(),
        missing.join("\n")
    );
}

// The scanner's own scenarios: each would let a raw key through, or flag a key no player sees.

fn keys_of(source: &str) -> Vec<String> {
    scan_source(Path::new("src/lib.rs"), source).0
}

fn fixture_locale() -> BTreeSet<String> {
    locale_keys(
        r#"{"be": {"msg": {"known": "x", "item_one": "a", "item_other": "b",
            "headline0": "h", "headline1": "h", "parent": {"subject": "s"}}}}"#,
    )
}

fn unknown(source: &str) -> Vec<String> {
    let emitted: Vec<Emitted> = keys_of(source)
        .into_iter()
        .map(|key| Emitted {
            file: PathBuf::from("src/lib.rs"),
            key,
        })
        .collect();
    missing_keys(&emitted, &fixture_locale())
}

/// Given production code that emits a key `en.json` lacks,
/// when the file is scanned,
/// then that key is reported.
#[test]
fn a_production_key_missing_from_the_locale_is_reported() {
    assert_eq!(
        unknown(r#"fn note() -> String { "be.msg.missing".to_string() }"#),
        ["src/lib.rs: be.msg.missing"]
    );
}

/// Given a missing key written inside `format!`, `vec!` and `Some(..)`,
/// when the file is scanned,
/// then each is found: a macro body is read token by token.
#[test]
fn keys_inside_macros_are_found() {
    let source = r#"fn f() { let _ = vec![format!("be.msg.inFormat"), "be.msg.inVec".into()]; }"#;
    assert_eq!(keys_of(source), ["be.msg.inFormat", "be.msg.inVec"]);
}

/// Given a missing key used only in a `#[cfg(test)]` function,
/// when the file is scanned,
/// then nothing is reported: the shipped game never emits it.
#[test]
fn a_key_in_a_cfg_test_function_is_not_reported() {
    let source = r#"
        #[cfg(test)]
        fn helper() -> String { "be.msg.missing".into() }
        fn shipped() -> &'static str { "be.msg.known" }
    "#;
    assert_eq!(keys_of(source), ["be.msg.known"]);
}

/// Given a missing key inside a `#[cfg(test)] mod`, a `#[test]` function, an
/// `#[cfg(all(test, ..))]` item and a test-only impl method,
/// when the file is scanned,
/// then none of them is reported.
#[test]
fn keys_in_any_test_only_item_are_not_reported() {
    let source = r#"
        #[cfg(test)] mod tests { fn a() { let _ = "be.msg.inTestMod"; } }
        #[test] fn b() { let _ = "be.msg.inTestFn"; }
        #[cfg(all(test, feature = "mcp"))] fn c() { let _ = "be.msg.inAllTest"; }
        struct S;
        impl S { #[cfg(test)] fn d() { let _ = "be.msg.inTestMethod"; } }
    "#;
    assert!(keys_of(source).is_empty(), "{:?}", keys_of(source));
}

/// Given a key in a `#[cfg(any(test, feature = "mcp"))]` item,
/// when the file is scanned,
/// then it is reported: that code ships in the MCP build.
#[test]
fn a_key_in_cfg_any_test_code_still_counts() {
    let source = r#"#[cfg(any(test, feature = "mcp"))] fn f() { let _ = "be.msg.shipped"; }"#;
    assert_eq!(keys_of(source), ["be.msg.shipped"]);
}

/// Given a key named only in a doc comment and in an `#[expect(.., reason)]`,
/// when the file is scanned,
/// then nothing is collected: attributes are not output.
#[test]
fn keys_in_doc_comments_and_attributes_are_not_collected() {
    let source = r#"
        /// Emits `be.msg.inDocs`.
        #[expect(dead_code, reason = "be.msg.inReason")]
        fn f() {}
    "#;
    assert!(keys_of(source).is_empty(), "{:?}", keys_of(source));
}

/// Given a plural key, an object the code appends to and a prefix,
/// when they are checked,
/// then all pass, and a prefix nothing starts with fails.
#[test]
fn plurals_parents_and_prefixes_resolve_against_the_locale() {
    let keys = fixture_locale();
    assert!(is_known(&keys, "be.msg.item"));
    assert!(is_known(&keys, "be.msg.parent"));
    assert!(is_known(&keys, "be.msg."));
    assert!(!is_known(&keys, "be.nothing."));
}

/// Given format strings for a family `en.json` has and one it lacks,
/// when they are checked,
/// then the first passes and the second fails; a placeholder needs at least one character.
#[test]
fn a_format_family_passes_only_when_some_key_matches_it() {
    let keys = fixture_locale();
    assert!(is_known(&keys, "be.msg.headline{}"));
    assert!(is_known(&keys, "be.msg.{kind}.subject"));
    assert!(!is_known(&keys, "be.msg.body{}"));
    assert!(!is_known(&keys, "be.msg.known{}"));
}

/// Given a crate whose root declares `mod shipped;`, `#[cfg(test)] mod tests;` and a
/// `#[path]` module,
/// when the module tree is walked,
/// then the shipped and `#[path]` files are read and the test module's file is not.
#[test]
fn the_module_walk_follows_shipped_modules_only() {
    let crate_dir = tempfile::tempdir().expect("temp dir");
    let src = crate_dir.path().join("src");
    fs::create_dir_all(src.join("shipped")).expect("module dir");
    let files = [
        (
            "lib.rs",
            "mod shipped;\n#[cfg(test)]\nmod tests;\n#[path = \"elsewhere.rs\"]\nmod moved;\n",
        ),
        (
            "shipped/mod.rs",
            "mod inner;\nfn a() { let _ = \"be.msg.shipped\"; }\n",
        ),
        ("shipped/inner.rs", "fn b() { let _ = \"be.msg.inner\"; }\n"),
        ("elsewhere.rs", "fn c() { let _ = \"be.msg.moved\"; }\n"),
        ("tests.rs", "fn d() { let _ = \"be.msg.inTests\"; }\n"),
    ];
    for (path, source) in files {
        fs::write(src.join(path), source).expect("fixture file");
    }

    let mut found: Vec<String> = emitted_keys(&[src.join("lib.rs")], &[])
        .into_iter()
        .map(|emitted| emitted.key)
        .collect();
    found.sort();

    assert_eq!(found, ["be.msg.inner", "be.msg.moved", "be.msg.shipped"]);
}
