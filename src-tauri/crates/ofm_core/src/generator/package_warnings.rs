//! Things a package author should hear about that do not stop the package loading.
//!
//! Errors (`validate_package`) say the package cannot be used. A warning says it can, but the
//! game will not do what the author probably meant. For example: a country the package
//! defines itself, outside the nation catalog, with no `confederation`. Nothing places it, so
//! `nations::region_for_code` quietly files it, and its clubs, under Europe.
//!
//! (A club or player naming a country that neither the catalog nor the package defines is
//! already an error, `unknownCountry`.)

use super::package::{PackageError, RepeatedNamePool, WorldPackage};
use crate::nations::nation_by_code;

pub(super) const COUNTRY_WITHOUT_REGION: &str = "be.warning.package.countryWithoutRegion";
pub(super) const FREE_AGENT_FIELD_IGNORED: &str = "be.warning.package.freeAgentFieldIgnored";
pub(super) const REPEATED_NAME_POOL: &str = "be.warning.package.repeatedNamePool";
pub(super) const PATCH_CLUB_REFS_UNVERIFIED: &str = "be.warning.package.patchClubRefsUnverified";
// One code for either half missing, rather than one carrying which half as a parameter: the
// message is rendered with `t(code, params)`, so a parameter would put an English word into a
// translated sentence.
pub(super) const NAME_POOL_INCOMPLETE: &str = "be.warning.package.namePoolIncomplete";

/// Problems that leave the package usable. Empty for a package whose own countries are all in
/// the catalog or placed in a confederation, whose club-less players carry no contract, and whose
/// name pools each have both first names and surnames.
pub fn package_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let mut warnings = country_warnings(package);
    warnings.extend(free_agent_warnings(package));
    warnings.extend(repeated_name_pool_warnings(package));
    warnings.extend(incomplete_name_pool_warnings(package));
    warnings.extend(patch_club_warnings(package));
    warnings
}

/// A patch naming clubs it does not define. Alone it cannot tell a club its base database defines
/// from one nothing defines, so this is not an error (#1271); the game checks it when the patch is
/// stacked, and `ofm-cli validate --base` checks it the same way. One warning, however many
/// references, naming how many distinct clubs went unchecked.
fn patch_club_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let is_patch = package
        .meta
        .as_ref()
        .is_some_and(|meta| meta.package_type == "patch");
    if !is_patch {
        return Vec::new();
    }
    let defined: std::collections::HashSet<&str> =
        package.teams.iter().map(|team| team.id.as_str()).collect();
    let mut unchecked: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut first_file: Option<String> = None;
    let player_clubs = package
        .players
        .iter()
        .enumerate()
        .map(|(index, player)| (player.club.as_str(), package.source_at("player", index)));
    let staff_clubs = package
        .staff
        .iter()
        .enumerate()
        .map(|(index, staff)| (staff.club.as_str(), package.source_at("staff", index)));
    for (club, file) in player_clubs.chain(staff_clubs) {
        if !club.is_empty() && !defined.contains(club) {
            unchecked.insert(club);
            first_file.get_or_insert(file);
        }
    }
    match first_file {
        Some(file) => vec![
            PackageError::new(PATCH_CLUB_REFS_UNVERIFIED, &file)
                .with("clubs", unchecked.len().to_string()),
        ],
        None => Vec::new(),
    }
}

/// A name pool missing its first names or its surnames. Building a world uses a package's pool
/// only when it has both, and drops one that does not without a word, so the players of that
/// country get the built-in pool's names, or generic ones (#1200).
fn incomplete_name_pool_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let Some(names) = &package.names else {
        return Vec::new();
    };
    let mut countries: Vec<&String> = names
        .pools
        .iter()
        .filter(|(_, pool)| !pool.is_usable())
        .map(|(country, _)| country)
        .collect();
    countries.sort();
    countries
        .into_iter()
        .map(|country| {
            PackageError::new(NAME_POOL_INCOMPLETE, package.name_pool_file(country))
                .with("country", country.clone())
        })
        .collect()
}

/// A country given a name pool by two files of one package: the later file replaces the earlier
/// pool. (Stacked packages overriding each other is the point of stacking and is not reported.)
fn repeated_name_pool_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let mut repeats: Vec<&RepeatedNamePool> = package.repeated_name_pools.iter().collect();
    repeats.sort_by(|a, b| (&a.later_file, &a.country).cmp(&(&b.later_file, &b.country)));
    repeats
        .into_iter()
        .map(|repeat| {
            PackageError::new(REPEATED_NAME_POOL, &repeat.later_file)
                .with("country", repeat.country.clone())
                .with("first", repeat.first_file.clone())
        })
        .collect()
}

fn country_warnings(package: &WorldPackage) -> Vec<PackageError> {
    package
        .countries
        .iter()
        .enumerate()
        .filter(|(_, country)| {
            !country.id.trim().is_empty()
                && country.confederation.trim().is_empty()
                && nation_by_code(&country.id).is_none()
        })
        .map(|(index, country)| {
            PackageError::new(COUNTRY_WITHOUT_REGION, &package.source_at("country", index))
                .with("name", country.name.clone())
                .with("code", country.id.clone())
        })
        .collect()
}

/// A player with no club is a free agent: no contract, no wage, not in any squad. A field that
/// only a club can give is reported rather than quietly dropped.
fn free_agent_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let mut warnings = Vec::new();
    for (index, player) in package.players.iter().enumerate() {
        if !player.club.is_empty() {
            continue;
        }
        let given = [
            ("wage", player.wage.is_some()),
            ("contractStart", player.contract_start.is_some()),
            ("contractEnd", player.contract_end.is_some()),
            ("contractLength", player.contract_length.is_some()),
            ("youth", player.youth),
        ];
        for (field, present) in given {
            if present {
                warnings.push(
                    PackageError::new(
                        FREE_AGENT_FIELD_IGNORED,
                        &package.source_at("player", index),
                    )
                    .with("id", player.id.clone())
                    .with("field", field),
                );
            }
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a patch package whose player is at a club the patch does not define (its base would)
    /// When its warnings are listed
    /// Then there is one saying the club references were not checked, naming how many: the package
    ///      alone cannot tell a club its base defines from a ghost, so it is not an error (#1364).
    #[test]
    fn a_patch_naming_clubs_it_does_not_define_gets_an_unverified_warning() {
        let dir = std::env::temp_dir().join(format!("ofm-warn-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("players")).unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"schema":"world","id":"patchy","name":"Patchy","version":"1.0.0","license":"CC0-1.0","packageType":"patch"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("players/players.json"),
            r#"{"schema":"player","items":[{"id":"p1","name":"P One","club":"base-fc","nationality":"ENG","position":"Striker","overall":70},{"id":"p2","name":"P Two","club":"base-fc","nationality":"ENG","position":"Striker","overall":70}]}"#,
        )
        .unwrap();
        let (package, errors) = crate::generator::load_world_package(&dir);
        std::fs::remove_dir_all(&dir).ok();

        let warnings = package_warnings(&package);

        assert!(
            errors.is_empty(),
            "alone, the patch has no error: {errors:?}"
        );
        let unverified: Vec<_> = warnings
            .iter()
            .filter(|w| w.code == PATCH_CLUB_REFS_UNVERIFIED)
            .collect();
        assert_eq!(unverified.len(), 1, "{warnings:?}");
        assert!(unverified[0].params.contains(&("clubs".into(), "1".into())));
    }

    /// Given a database package whose players name only clubs it defines
    /// When its warnings are listed
    /// Then there is no unverified-references warning: only a patch defers its references.
    #[test]
    fn a_database_package_gets_no_unverified_warning() {
        let warnings = package_warnings(&WorldPackage::default());

        assert!(
            warnings
                .iter()
                .all(|w| w.code != PATCH_CLUB_REFS_UNVERIFIED)
        );
    }
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

    fn incomplete_pool_warnings(first: &[&str], last: &[&str]) -> Vec<PackageError> {
        let list = |names: &[&str]| {
            names
                .iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(",")
        };
        let contents = format!(
            r#"{{"schema":"names","pools":{{"BR":{{"first_names":[{}],"last_names":[{}]}}}}}}"#,
            list(first),
            list(last)
        );
        warnings_for("pool", &[("names/pools.json", contents.as_str())])
            .into_iter()
            .filter(|warning| warning.code == NAME_POOL_INCOMPLETE)
            .collect()
    }

    /// Given a package whose names file gives Brazil surnames but no first names
    /// When its warnings are listed
    /// Then one names the country and the file: a pool is used only with both lists, so this one
    ///      would be dropped without a word when the world is built (#1200)
    #[test]
    fn a_name_pool_with_only_surnames_is_warned_about() {
        let warnings = incomplete_pool_warnings(&[], &["Silva", "Souza"]);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].file, "names/pools.json");
        assert_eq!(
            warnings[0].params,
            vec![("country".to_string(), "BR".to_string())]
        );
    }

    /// Given a package whose names file gives Brazil first names but no surnames
    /// When its warnings are listed
    /// Then one names the country, as for the other half missing
    #[test]
    fn a_name_pool_with_only_first_names_is_warned_about() {
        let warnings = incomplete_pool_warnings(&["Joao", "Pedro"], &[]);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(
            warnings[0].params,
            vec![("country".to_string(), "BR".to_string())]
        );
    }

    /// Given a package whose names file gives Brazil both first names and surnames
    /// When its warnings are listed
    /// Then none is about the pool
    #[test]
    fn a_complete_name_pool_is_not_warned_about() {
        assert!(incomplete_pool_warnings(&["Joao"], &["Silva"]).is_empty());
    }

    const COUNTRY_QQ: (&str, &str) = (
        "countries/qq.json",
        r#"{"schema":"country","id":"QQ","name":"Qland"}"#,
    );

    /// Given a package that defines a country outside the catalog and puts it in no confederation
    /// When its warnings are listed
    /// Then one names the country, the file and the code.
    #[test]
    fn a_country_with_no_region_is_warned_about() {
        let warnings = warnings_for("qq", &[COUNTRY_QQ]);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].code, COUNTRY_WITHOUT_REGION);
        assert_eq!(warnings[0].file, "countries/qq.json");
        assert!(
            warnings[0]
                .params
                .contains(&("name".into(), "Qland".into()))
        );
        assert!(warnings[0].params.contains(&("code".into(), "QQ".into())));
    }

    /// Given the same country placed in a confederation the package defines
    /// When its warnings are listed
    /// Then there is none: the package placed it.
    #[test]
    fn a_country_placed_in_a_confederation_is_silent() {
        let warnings = warnings_for(
            "placed",
            &[
                (
                    "countries/qq.json",
                    r#"{"schema":"country","id":"QQ","name":"Qland","confederation":"qconf"}"#,
                ),
                (
                    "confederations/qconf.json",
                    r#"{"schema":"confederation","id":"qconf","name":"Q Confed"}"#,
                ),
            ],
        );

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// Given a package whose countries are all in the catalog, placed or not
    /// When it is validated
    /// Then there is no warning.
    #[test]
    fn a_catalog_country_is_silent() {
        let warnings = warnings_for(
            "catalog",
            &[(
                "countries/eng.json",
                r#"{"schema":"country","id":"ENG","name":"England"}"#,
            )],
        );

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    fn free_agent_json(extra: &str) -> String {
        format!(
            r#"{{"schema":"player","id":"agent-1","firstName":"Free","lastName":"Agent",
                "nationality":"ENG","position":"Striker","dateOfBirth":"1990-01-01",
                "overall":70{extra}}}"#
        )
    }

    /// Given a player with no club who is given one contract field
    /// When the package's warnings are listed
    /// Then one names the player, the file and the field: a free agent has no contract, so the
    ///      field would otherwise be dropped without a word.
    fn assert_clubless_field_is_warned(field: &str, extra: &str) {
        let json = free_agent_json(extra);
        let warnings = warnings_for("agent-field", &[("players/agent-1.json", &json)]);

        assert_eq!(warnings.len(), 1, "{field}: {warnings:?}");
        assert_eq!(warnings[0].code, FREE_AGENT_FIELD_IGNORED, "{field}");
        assert_eq!(warnings[0].file, "players/agent-1.json", "{field}");
        assert!(
            warnings[0]
                .params
                .contains(&("id".into(), "agent-1".into())),
            "{field}: {warnings:?}"
        );
        assert!(
            warnings[0].params.contains(&("field".into(), field.into())),
            "{field}: {warnings:?}"
        );
    }

    #[test]
    fn a_clubless_players_wage_is_warned_about() {
        assert_clubless_field_is_warned("wage", r#","wage":5000"#);
    }

    #[test]
    fn a_clubless_players_contract_start_is_warned_about() {
        assert_clubless_field_is_warned("contractStart", r#","contractStart":"2026-07-01""#);
    }

    #[test]
    fn a_clubless_players_contract_end_is_warned_about() {
        assert_clubless_field_is_warned("contractEnd", r#","contractEnd":"2028-06-30""#);
    }

    #[test]
    fn a_clubless_players_contract_length_is_warned_about() {
        assert_clubless_field_is_warned("contractLength", r#","contractLength":2"#);
    }

    #[test]
    fn a_clubless_players_youth_flag_is_warned_about() {
        assert_clubless_field_is_warned("youth", r#","youth":true"#);
    }

    /// Given a club-less player with no contract fields, and a clubbed player with all of them
    /// When the warnings are listed
    /// Then there are none: only a field that cannot apply is reported.
    #[test]
    fn a_bare_free_agent_and_a_clubbed_contract_are_silent() {
        let bare = free_agent_json("");
        let clubbed = r#"{"schema":"player","id":"p2","club":"club-x","firstName":"A","lastName":"B",
            "nationality":"ENG","position":"Striker","dateOfBirth":"1990-01-01","overall":70,
            "wage":5000,"contractStart":"2026-07-01","contractEnd":"2028-06-30","youth":true}"#;

        let warnings = warnings_for(
            "silent",
            &[("players/a.json", &bare), ("players/b.json", clubbed)],
        );

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// Given a package that defines no country
    /// When it is validated
    /// Then there is no warning.
    #[test]
    fn a_package_without_countries_is_silent() {
        assert!(warnings_for("none", &[]).is_empty());
    }

    const ENG_POOL: &str = r#""ENG": { "first_names": ["Tom"], "last_names": ["Smith"] }"#;
    const BRA_POOL: &str = r#""BRA": { "first_names": ["Joao"], "last_names": ["Silva"] }"#;

    fn names_file(pools: &str) -> String {
        format!(r#"{{ "schema": "names", "pools": {{ {pools} }} }}"#)
    }

    /// Given two files of one package that each give the pool for England
    /// When the warnings are listed
    /// Then one names the country, the later file and the earlier one: the later replaces it.
    #[test]
    fn a_country_given_a_name_pool_twice_is_warned_about() {
        let (a, b) = (names_file(ENG_POOL), names_file(ENG_POOL));

        let warnings = warnings_for("pool-twice", &[("names/a.json", &a), ("names/b.json", &b)]);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].code, REPEATED_NAME_POOL);
        assert_eq!(warnings[0].file, "names/b.json");
        assert!(
            warnings[0]
                .params
                .contains(&("country".into(), "ENG".into()))
        );
        assert!(
            warnings[0]
                .params
                .contains(&("first".into(), "names/a.json".into()))
        );
    }

    /// Given files that give different countries, and one file whose `pools` and `items` both name
    ///       England (the documented order inside a file)
    /// When the warnings are listed
    /// Then there is none: only a country given by two files is reported.
    #[test]
    fn different_countries_and_a_pools_items_overlap_in_one_file_are_silent() {
        let (a, b) = (names_file(ENG_POOL), names_file(BRA_POOL));
        let overlap = format!(
            r#"{{ "schema": "names", "pools": {{ {ENG_POOL} }}, "items": [ {{ "pools": {{ {ENG_POOL} }} }} ] }}"#
        );

        let apart = warnings_for("pool-apart", &[("names/a.json", &a), ("names/b.json", &b)]);
        let within = warnings_for("pool-within", &[("names/c.json", &overlap)]);

        assert!(
            apart.is_empty() && within.is_empty(),
            "{apart:?} {within:?}"
        );
    }
}
