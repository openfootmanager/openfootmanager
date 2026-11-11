//! What `ofm-cli` prints and exits with, read from the real binary: stdout is the interface an
//! author sees, so it is tested where it is written (#1364).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ofm-cli-output-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
    std::fs::write(path, body).expect("written");
}

/// A database package defining one club, "base-fc", and a second so a world could be built.
fn base_database(name: &str) -> PathBuf {
    let dir = scratch(name);
    write(
        &dir,
        "package.json",
        r#"{"schema":"world","id":"base","name":"Base","version":"1.0.0","license":"CC0-1.0"}"#,
    );
    write(
        &dir,
        "country.json",
        r#"{"schema":"country","id":"ENG","name":"England"}"#,
    );
    write(
        &dir,
        "teams/teams.json",
        r##"{"schema":"team","items":[{"id":"base-fc","name":"Base FC","city":"Base","country":"ENG","colors":{"primary":"#000000","secondary":"#ffffff"}}]}"##,
    );
    dir
}

/// A patch signing one player for `club`.
fn patch_signing_for(name: &str, club: &str) -> PathBuf {
    let dir = scratch(name);
    write(
        &dir,
        "package.json",
        r#"{"schema":"world","id":"patchy","name":"Patchy","version":"1.0.0","license":"CC0-1.0","packageType":"patch"}"#,
    );
    write(
        &dir,
        "players/players.json",
        &format!(
            r#"{{"schema":"player","items":[{{"id":"p1","name":"P One","club":"{club}","nationality":"ENG","position":"Striker","overall":70}}]}}"#
        ),
    );
    dir
}

fn ofm_cli(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ofm-cli"))
        .args(args)
        // The words are what is read, not the colour codes round them.
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .output()
        .expect("the CLI runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn pack(dir: &Path) -> Output {
    let archive = dir.with_extension("ofm");
    ofm_cli(&[
        "pack".as_ref(),
        dir.as_os_str(),
        "--output".as_ref(),
        archive.as_os_str(),
    ])
}

fn validate(path: &Path, bases: &[&Path]) -> Output {
    let mut args: Vec<&std::ffi::OsStr> = vec!["validate".as_ref(), path.as_os_str()];
    for base in bases {
        args.push("--base".as_ref());
        args.push(base.as_os_str());
    }
    ofm_cli(&args)
}

fn archive_of(dir: &Path) -> PathBuf {
    let archive = dir.with_extension("ofm");
    assert_eq!(pack(dir).status.code(), Some(0), "the package packs");
    archive
}

/// Given a patch package, and a database package
/// When each is packed
/// Then the patch's output carries the guidance (references are not checked by pack; use
///      `validate --base`) and the database package's does not.
#[test]
fn packing_a_patch_prints_the_guidance_and_a_database_does_not() {
    let patch = patch_signing_for("pack-patch", "base-fc");
    let database = base_database("pack-database");

    let patch_out = stdout(&pack(&patch));
    let database_out = stdout(&pack(&database));

    assert!(patch_out.contains("patch package"), "{patch_out}");
    assert!(patch_out.contains("--base"), "{patch_out}");
    assert!(!database_out.contains("patch package"), "{database_out}");
}

/// Given a patch naming a club only its base defines, validated with and without that base
/// When `validate` prints
/// Then alone it warns that the club references were not checked, and with the base that warning is
///      gone (they were), while the rest of the output stands.
#[test]
fn a_successful_base_check_does_not_say_the_clubs_were_not_checked() {
    let base = base_database("warn-base");
    let patch = patch_signing_for("warn-patch", "base-fc");

    let alone = validate(&patch, &[]);
    let on_base = validate(&patch, &[&base]);

    assert_eq!(alone.status.code(), Some(0), "{}", stdout(&alone));
    assert!(
        stdout(&alone).contains("patchClubRefsUnverified"),
        "{}",
        stdout(&alone)
    );
    assert_eq!(on_base.status.code(), Some(0), "{}", stdout(&on_base));
    assert!(
        !stdout(&on_base).contains("patchClubRefsUnverified"),
        "{}",
        stdout(&on_base)
    );
    assert!(stdout(&on_base).contains("Valid"), "{}", stdout(&on_base));
}

/// Given a legit patch and a ghost-club patch, each packed as a `.ofm`, and the base packed too
/// When `validate <archive> --base <base archive>` runs on each
/// Then the legit archive passes and the ghost archive fails: the flag is not ignored for an
///      archive.
#[test]
fn validate_checks_a_patch_archive_on_its_base_too() {
    let base = archive_of(&base_database("archive-base"));
    let legit = archive_of(&patch_signing_for("archive-legit", "base-fc"));
    let ghost = archive_of(&patch_signing_for("archive-ghost", "ghost-fc"));

    let legit_out = validate(&legit, &[&base]);
    let ghost_out = validate(&ghost, &[&base]);

    assert_eq!(legit_out.status.code(), Some(0), "{}", stdout(&legit_out));
    assert_eq!(ghost_out.status.code(), Some(1), "{}", stdout(&ghost_out));
}
