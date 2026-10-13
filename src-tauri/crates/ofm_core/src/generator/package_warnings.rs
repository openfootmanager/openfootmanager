//! Things a package author should hear about that do not stop the package loading.
//!
//! Errors (`validate_package`) say the package cannot be used. A warning says it can, but the
//! game will not do what the author probably meant. Today there is one: a country code the game
//! has never heard of, which `nations::region_for_code` quietly files under Europe.

use std::collections::HashSet;

use super::package::{PackageError, WorldPackage};
use crate::nations::nation_by_name;

pub(super) const UNKNOWN_COUNTRY_CODE: &str = "be.warning.package.unknownCountryCode";

/// Problems that leave the package usable. Empty for a package whose country codes are all
/// ones the game knows or the package itself defines.
pub fn package_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let own_countries: HashSet<&str> = package.countries.iter().map(|c| c.id.as_str()).collect();
    let is_unknown = |code: &str| {
        !code.trim().is_empty() && nation_by_name(code).is_none() && !own_countries.contains(code)
    };

    let teams = package
        .teams
        .iter()
        .enumerate()
        .filter(|(_, team)| is_unknown(&team.country))
        .map(|(index, team)| warning("team", index, &team.name, &team.country, package));
    let players = package
        .players
        .iter()
        .enumerate()
        .filter(|(_, player)| is_unknown(&player.nationality))
        .map(|(index, player)| {
            let called = if player.name.is_empty() {
                format!("{} {}", player.first_name, player.last_name)
            } else {
                player.name.clone()
            };
            warning("player", index, called.trim(), &player.nationality, package)
        });
    teams.chain(players).collect()
}

fn warning(
    schema: &str,
    index: usize,
    name: &str,
    code: &str,
    package: &WorldPackage,
) -> PackageError {
    PackageError::new(UNKNOWN_COUNTRY_CODE, &package.source_at(schema, index))
        .with("kind", schema)
        .with("name", name)
        .with("code", code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::load_world_package;
    use std::path::{Path, PathBuf};

    fn package_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ofm-warn-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, rel: &str, contents: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn warnings_for(tag: &str, files: &[(&str, &str)]) -> Vec<PackageError> {
        let dir = package_dir(tag);
        for (rel, contents) in files {
            write(&dir, rel, contents);
        }
        let (package, _errors) = load_world_package(&dir);
        std::fs::remove_dir_all(&dir).ok();
        package_warnings(&package)
    }

    const CLUB_IN_QQ: (&str, &str) = (
        "teams/teams.json",
        r##"{"schema":"team","items":[{"id":"qq-fc","name":"QQ FC","city":"Qtown","country":"QQ",
            "colors":{"primary":"#000000","secondary":"#ffffff"}}]}"##,
    );

    /// Given a package with a club whose country code is not in the catalog
    /// When its warnings are listed
    /// Then one names the club, the file and the unknown code.
    #[test]
    fn a_club_in_an_unknown_country_is_warned_about() {
        let warnings = warnings_for("club", &[CLUB_IN_QQ]);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].code, UNKNOWN_COUNTRY_CODE);
        assert_eq!(warnings[0].file, "teams/teams.json");
        let param = |key: &str| {
            warnings[0]
                .params
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(param("name"), Some("QQ FC"));
        assert_eq!(param("code"), Some("QQ"));
    }

    /// Given a package with a player of an unknown nationality
    /// When its warnings are listed
    /// Then one names the player, the file and the code.
    #[test]
    fn a_player_of_an_unknown_nationality_is_warned_about() {
        let warnings = warnings_for(
            "player",
            &[(
                "players/players.json",
                r#"{"schema":"player","id":"p1","name":"Pat Q","nationality":"QQ","club":"x"}"#,
            )],
        );

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].file, "players/players.json");
        assert!(warnings[0].params.contains(&("code".into(), "QQ".into())));
        assert!(
            warnings[0]
                .params
                .contains(&("name".into(), "Pat Q".into()))
        );
    }

    /// Given a package whose club and player codes are all in the catalog, by code or by name
    /// When it is validated
    /// Then there is no country-code warning.
    #[test]
    fn known_country_codes_are_silent() {
        let warnings = warnings_for(
            "known",
            &[
                (
                    "teams/teams.json",
                    r##"{"schema":"team","id":"t","name":"T","city":"C","country":"ENG",
                        "colors":{"primary":"#000000","secondary":"#ffffff"}}"##,
                ),
                (
                    "players/players.json",
                    r#"{"schema":"player","id":"p1","name":"Pat","nationality":"England","club":"t"}"#,
                ),
            ],
        );

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// Given a club in a country the package itself defines, outside the catalog
    /// When it is validated
    /// Then it is silent: the package placed that country, so nothing falls back to Europe.
    #[test]
    fn a_country_the_package_defines_is_silent() {
        let warnings = warnings_for(
            "own",
            &[
                CLUB_IN_QQ,
                (
                    "countries/qq.json",
                    r#"{"schema":"country","id":"QQ","name":"Qland","confederation":"qconf"}"#,
                ),
            ],
        );

        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
