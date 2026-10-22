//! The TypeScript types in `src/store/generated/` are generated from the Rust types the backend
//! serialises to the frontend, so a field can no longer reach the wire without reaching the
//! screen's types (#959 had to add 32 of them by hand). These tests keep the checked-in files
//! equal to what the Rust types generate today.
//!
//! When a test here fails because a wire type changed, regenerate the files and commit them:
//!
//! ```text
//! OFM_UPDATE_TS_TYPES=1 cargo test --manifest-path src-tauri/Cargo.toml --test wire_types
//! ```
//!
//! The files are written by `ts-rs`, never by `#[ts(export)]`: that attribute writes into the
//! source tree on every `cargo test`, which would hide drift instead of failing on it.

use std::any::TypeId;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use ts_rs::{Config, TypeVisitor, TS};

const GENERATED_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/store/generated");
const REGENERATE_VAR: &str = "OFM_UPDATE_TS_TYPES";

/// The types a Tauri command hands the frontend whole. Everything they reach is generated with
/// them; a type is added here only when a command returns it and nothing listed already reaches it.
fn visit_wire_roots(visitor: &mut impl TypeVisitor) {
    visitor.visit::<ofm_core::game::Game>();
    visitor.visit::<ofm_core::slices::squad::SquadPlayer>();
}

/// `u64` would otherwise be `bigint`, but `JSON.parse` hands the screen a `number`.
fn config(out_dir: &Path) -> Config {
    Config::new().with_large_int("number").with_out_dir(out_dir)
}

struct Exporter<'a> {
    config: &'a Config,
    failure: Option<String>,
}

impl TypeVisitor for Exporter<'_> {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        if self.failure.is_some() {
            return;
        }
        if let Err(error) = T::export_all(self.config) {
            self.failure = Some(format!("{}: {error}", std::any::type_name::<T>()));
        }
    }
}

fn export_wire_types(out_dir: &Path) -> Result<(), String> {
    let config = config(out_dir);
    let mut exporter = Exporter {
        config: &config,
        failure: None,
    };
    visit_wire_roots(&mut exporter);
    exporter.failure.map_or(Ok(()), Err)
}

/// Walks the same graph `export_all` does, recording for each file the declarations that would be
/// written to it and the Rust types behind each.
struct FileClaims {
    config: Config,
    seen: HashSet<TypeId>,
    rust_types_by_declaration_by_file: BTreeMap<PathBuf, BTreeMap<String, BTreeSet<&'static str>>>,
}

impl FileClaims {
    fn new() -> Self {
        Self {
            config: config(Path::new(".")),
            seen: HashSet::new(),
            rust_types_by_declaration_by_file: BTreeMap::new(),
        }
    }
}

impl TypeVisitor for FileClaims {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        let Some(file) = T::output_path() else {
            return;
        };
        if !self.seen.insert(TypeId::of::<T>()) {
            return;
        }
        self.rust_types_by_declaration_by_file
            .entry(file)
            .or_default()
            .entry(T::decl(&self.config))
            .or_default()
            .insert(std::any::type_name::<T>());
        T::visit_dependencies(self);
    }
}

/// Files that two different declarations would be written to. `ts-rs` writes the first and
/// silently drops the rest, so the screen would read one type under the other's name. Two Rust
/// types with one declaration are not a clash: `ts-rs` itself declares `serde_json::Value` as its
/// own `TsJsonValue`.
fn contested_files(claims: &FileClaims) -> Vec<String> {
    claims
        .rust_types_by_declaration_by_file
        .iter()
        .filter(|(_, declarations)| declarations.len() > 1)
        .map(|(file, declarations)| {
            let rust_types: Vec<_> = declarations.values().flatten().copied().collect();
            format!("{} <- {}", file.display(), rust_types.join(", "))
        })
        .collect()
}

fn relative_files(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeSet<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, files);
            } else if let Ok(relative) = path.strip_prefix(root) {
                files.insert(relative.to_path_buf());
            }
        }
    }
    let mut files = BTreeSet::new();
    walk(root, root, &mut files);
    files
}

/// How the checked-in files differ from freshly generated ones, one line per file.
fn differences(checked_in: &Path, generated: &Path) -> Vec<String> {
    let checked_in_files = relative_files(checked_in);
    let generated_files = relative_files(generated);
    let mut found = Vec::new();
    for file in generated_files.difference(&checked_in_files) {
        found.push(format!("missing: {}", file.display()));
    }
    for file in checked_in_files.difference(&generated_files) {
        found.push(format!("no Rust type generates it: {}", file.display()));
    }
    for file in checked_in_files.intersection(&generated_files) {
        if std::fs::read(checked_in.join(file)).ok() != std::fs::read(generated.join(file)).ok() {
            found.push(format!("out of date: {}", file.display()));
        }
    }
    found
}

/// Never a fixed path: `ts-rs` remembers every file it has written in this process and merges
/// later exports into it, so two tests sharing a directory would see each other's types.
fn scratch_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("ofm-wire-types-")
        .tempdir()
        .expect("could not create a scratch directory")
}

/// Clears `target` first, so a file whose Rust type left the wire does not survive.
fn regenerate(target: &Path) {
    if target.exists() {
        std::fs::remove_dir_all(target).expect("could not clear the generated types");
    }
    export_wire_types(target).expect("could not write the generated types");
}

/// Given the Rust wire types as they stand, when they are generated afresh, then the result is
/// byte-for-byte the files checked in: none missing, none left over, none out of date.
#[test]
fn the_checked_in_typescript_matches_the_rust_wire_types() {
    if std::env::var_os(REGENERATE_VAR).is_some() {
        regenerate(Path::new(GENERATED_DIR));
    }
    let fresh = scratch_dir();
    export_wire_types(fresh.path()).expect("the wire types could not be generated");

    let found = differences(Path::new(GENERATED_DIR), fresh.path());

    assert!(
        found.is_empty(),
        "src/store/generated no longer matches the Rust wire types:\n  {}\n\nRegenerate with\n  \
         {REGENERATE_VAR}=1 cargo test --manifest-path src-tauri/Cargo.toml --test wire_types\n\
         and commit the result.",
        found.join("\n  ")
    );
}

/// Given every type the wire roots reach, when each is assigned its file, then no file is wanted
/// by two different declarations.
#[test]
fn no_two_wire_types_share_a_typescript_name() {
    let mut claims = FileClaims::new();
    visit_wire_roots(&mut claims);

    let contested = contested_files(&claims);

    assert!(
        contested.is_empty(),
        "two Rust types generate different TypeScript under one name; give one a \
         `#[ts(rename = \"...\")]`:\n  {}",
        contested.join("\n  ")
    );
}

#[expect(
    dead_code,
    reason = "these types exist only for the TypeScript they generate; owner=#589"
)]
mod clashing {
    pub mod calendar {
        #[derive(ts_rs::TS)]
        pub enum Phase {
            Opening,
        }

        #[derive(ts_rs::TS)]
        pub enum Side {
            Home,
        }
    }

    pub mod season {
        #[derive(ts_rs::TS)]
        pub enum Phase {
            Preseason,
        }

        #[derive(ts_rs::TS)]
        pub enum Side {
            Home,
        }
    }

    #[derive(ts_rs::TS)]
    pub struct DifferentPhases {
        pub calendar: calendar::Phase,
        pub season: season::Phase,
    }

    #[derive(ts_rs::TS)]
    pub struct SameSides {
        pub calendar: calendar::Side,
        pub season: season::Side,
    }
}

/// Given two Rust types with one name and different variants, reached from one root, when files
/// are assigned, then the shared file is reported with both Rust types named.
#[test]
fn two_rust_types_with_one_name_are_reported() {
    let mut claims = FileClaims::new();
    claims.visit::<clashing::DifferentPhases>();

    let contested = contested_files(&claims);

    assert_eq!(contested.len(), 1, "{contested:?}");
    assert!(contested[0].contains("calendar::Phase"), "{}", contested[0]);
    assert!(contested[0].contains("season::Phase"), "{}", contested[0]);
}

/// Given two Rust types with one name that declare the same TypeScript, when files are assigned,
/// then nothing is reported: whichever is written, the screen reads the same type.
#[test]
fn two_rust_types_with_one_declaration_are_not_reported() {
    let mut claims = FileClaims::new();
    claims.visit::<clashing::SameSides>();

    assert_eq!(contested_files(&claims), Vec::<String>::new());
}

fn write(dir: &Path, file: &str, contents: &str) {
    std::fs::write(dir.join(file), contents).expect("could not write a fixture file");
}

/// Given a checked-in file whose Rust type has since gained a field, when compared, then that
/// file is reported out of date.
#[test]
fn a_file_older_than_its_rust_type_is_reported() {
    let checked_in = scratch_dir();
    let generated = scratch_dir();
    write(
        checked_in.path(),
        "Team.ts",
        "export type Team = { id: string, };\n",
    );
    write(
        generated.path(),
        "Team.ts",
        "export type Team = { id: string, name: string, };\n",
    );

    assert_eq!(
        differences(checked_in.path(), generated.path()),
        ["out of date: Team.ts"]
    );
}

/// Given a Rust type that nothing has generated a file for, when compared, then the file is
/// reported missing.
#[test]
fn a_rust_type_without_a_file_is_reported() {
    let checked_in = scratch_dir();
    let generated = scratch_dir();
    write(
        generated.path(),
        "Injury.ts",
        "export type Injury = { days: number, };\n",
    );

    assert_eq!(
        differences(checked_in.path(), generated.path()),
        ["missing: Injury.ts"]
    );
}

/// Given a checked-in file whose Rust type left the wire, when compared, then the file is
/// reported as generated by nothing.
#[test]
fn a_file_without_a_rust_type_is_reported() {
    let checked_in = scratch_dir();
    let generated = scratch_dir();
    write(
        checked_in.path(),
        "LegacyLeague.ts",
        "export type LegacyLeague = {};\n",
    );

    assert_eq!(
        differences(checked_in.path(), generated.path()),
        ["no Rust type generates it: LegacyLeague.ts"]
    );
}

/// Given a directory holding a stale and an orphaned file, when it is regenerated, then it
/// matches a fresh export exactly.
#[test]
fn regenerating_replaces_stale_and_orphaned_files() {
    let target = scratch_dir();
    write(target.path(), "Game.ts", "export type Game = {};\n");
    write(
        target.path(),
        "LegacyLeague.ts",
        "export type LegacyLeague = {};\n",
    );
    let fresh = scratch_dir();
    export_wire_types(fresh.path()).expect("the wire types could not be generated");

    regenerate(target.path());

    assert_eq!(
        differences(target.path(), fresh.path()),
        Vec::<String>::new()
    );
}
