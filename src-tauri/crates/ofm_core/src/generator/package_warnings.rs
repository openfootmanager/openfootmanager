//! Things a package author should hear about that do not stop the package loading.
//!
//! Errors (`validate_package`) say the package cannot be used. A warning says it can, but the
//! game will not do what the author probably meant. For example: a country the package
//! defines itself, outside the nation catalog, with no `confederation`. Nothing places it, so
//! `nations::region_for_code` quietly files it, and its clubs, under Europe.
//!
//! (A club or player naming a country that neither the catalog nor the package defines is
//! already an error, `unknownCountry`.)

use super::package::{PackageError, WorldPackage};
use crate::nations::nation_by_code;

pub(super) const COUNTRY_WITHOUT_REGION: &str = "be.warning.package.countryWithoutRegion";
pub(super) const FREE_AGENT_FIELD_IGNORED: &str = "be.warning.package.freeAgentFieldIgnored";
pub(super) const REPEATED_NAME_POOL: &str = "be.warning.package.repeatedNamePool";

/// Problems that leave the package usable. Empty for a package whose own countries are all in
/// the catalog or placed in a confederation, and whose club-less players carry no contract.
pub fn package_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let mut warnings = country_warnings(package);
    warnings.extend(free_agent_warnings(package));
    warnings.extend(repeated_name_pool_warnings(package));
    warnings
}

/// A country given a name pool by two files of one package: the later file replaces the earlier
/// pool. (Stacked packages overriding each other is the point of stacking and is not reported.)
fn repeated_name_pool_warnings(package: &WorldPackage) -> Vec<PackageError> {
    let mut repeats: Vec<&(String, String, String)> = package.repeated_name_pools.iter().collect();
    repeats.sort_by(|a, b| (&a.2, &a.0).cmp(&(&b.2, &b.0)));
    repeats
        .into_iter()
        .map(|(country, first, later)| {
            PackageError::new(REPEATED_NAME_POOL, later)
                .with("country", country.clone())
                .with("first", first.clone())
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
