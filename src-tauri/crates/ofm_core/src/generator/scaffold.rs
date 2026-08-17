//! Creating packages: the project skeleton and the per-entity starter shapes.
//!
//! Reading a package was always shared — `load_world_package`, `validate_package`
//! and `export_directory_to_ofm` serve the CLI, the editor and the installer
//! alike. *Creating* one was not: `ofm-cli new` hand-wrote its manifest as a JSON
//! literal while the editor serialized [`WorldMetaDef`], so a field added to the
//! struct reached the editor for free and never reached the CLI. `fallbackLeague`
//! and `logo` were both missing from `ofm-cli new` output for exactly that
//! reason, and nothing failed — there was no test that could.
//!
//! Everything here is the one definition both front ends use. The parity tests at
//! the bottom are the mechanism, not a formality: each builds its `*Def` with an
//! **exhaustive** struct literal, so adding a field to a definition stops this
//! module compiling until the template is updated too.

use std::path::Path;

use serde_json::{json, Value};

use super::package::WorldMetaDef;

/// The entity kinds a package can hold, and the only list of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    World,
    Team,
    Player,
    Staff,
    Confederation,
    Country,
    Competition,
    Names,
}

impl EntityKind {
    /// Every kind, for callers that need to iterate (the CLI's `--help`, the
    /// scaffolder, and the parity tests, which must not be able to miss one).
    pub const ALL: &'static [EntityKind] = &[
        EntityKind::World,
        EntityKind::Team,
        EntityKind::Player,
        EntityKind::Staff,
        EntityKind::Confederation,
        EntityKind::Country,
        EntityKind::Competition,
        EntityKind::Names,
    ];

    /// Value of the file's `schema` field.
    pub fn schema_name(self) -> &'static str {
        match self {
            EntityKind::World => "world",
            EntityKind::Team => "team",
            EntityKind::Player => "player",
            EntityKind::Staff => "staff",
            EntityKind::Confederation => "confederation",
            EntityKind::Country => "country",
            EntityKind::Competition => "competition",
            EntityKind::Names => "names",
        }
    }

    /// Subdirectory this kind lives in. `None` for the manifest, which sits at
    /// the package root.
    pub fn dir(self) -> Option<&'static str> {
        match self {
            EntityKind::World => None,
            EntityKind::Team => Some("teams"),
            EntityKind::Player => Some("players"),
            EntityKind::Staff => Some("staff"),
            EntityKind::Confederation => Some("confederations"),
            EntityKind::Country => Some("countries"),
            EntityKind::Competition => Some("competitions"),
            EntityKind::Names => Some("names"),
        }
    }

    /// Default filename for this kind's collection file.
    pub fn file_name(self) -> &'static str {
        match self {
            EntityKind::World => "package.json",
            EntityKind::Team => "teams.json",
            EntityKind::Player => "players.json",
            EntityKind::Staff => "staff.json",
            EntityKind::Confederation => "confederations.json",
            EntityKind::Country => "countries.json",
            EntityKind::Competition => "competitions.json",
            EntityKind::Names => "names.json",
        }
    }
}

/// Turn a display name into an id: lowercase, alphanumerics kept, everything
/// else collapsed to a single hyphen.
///
/// Ids reach the filesystem (`<id>.ofm`, `assets/images/<id>.png`), so this is
/// deliberately narrow. It is *not* the same as validating an id someone typed —
/// that is [`super::package::is_valid_package_id`].
pub fn slugify(name: &str) -> String {
    let raw: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    raw.split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Metadata for a brand-new package.
///
/// The one place that decides what a package starts out as. The editor's New
/// Package form and `ofm-cli new` must agree on this, or two packages created
/// the same day differ by which tool made them.
pub fn new_package_meta(
    name: &str,
    author: &str,
    version: &str,
    package_type: &str,
) -> WorldMetaDef {
    WorldMetaDef {
        id: slugify(name),
        name: name.to_string(),
        version: version.to_string(),
        author: author.to_string(),
        package_type: package_type.to_string(),
        license: "CC-BY-4.0".to_string(),
        format_version: super::package::SUPPORTED_PACKAGE_FORMAT_VERSION,
        ..WorldMetaDef::default()
    }
}

/// The manifest as it is written to disk: the metadata, plus the `schema` tag
/// that marks the file as the package manifest.
///
/// Serialized from [`WorldMetaDef`] rather than restated, so a new field appears
/// here the moment it exists on the struct.
pub fn manifest_json(meta: &WorldMetaDef) -> Result<Value, String> {
    let mut value = serde_json::to_value(meta).map_err(|e| e.to_string())?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "package metadata did not serialize to an object".to_string())?;
    object.insert("schema".to_string(), json!(EntityKind::World.schema_name()));
    Ok(value)
}

/// A names definition as it is written to disk: the pools, plus the `schema` tag.
///
/// Shares [`manifest_json`]'s shape for the same reason — the editor's save path
/// and the scaffolder must not disagree about what a names file looks like.
pub fn names_json(names: &super::definitions::NamesDefinition) -> Result<Value, String> {
    let mut value = serde_json::to_value(names).map_err(|e| e.to_string())?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "names definition did not serialize to an object".to_string())?;
    object.insert("schema".to_string(), json!(EntityKind::Names.schema_name()));
    Ok(value)
}

/// An empty collection file for `kind`, as written into a fresh project.
fn empty_collection(kind: EntityKind) -> Value {
    match kind {
        // Names is not a list of entities but a single keyed document, so its
        // empty shape is its own.
        EntityKind::Names => json!({
            "schema": "names",
            "version": 1,
            "description": "",
            "pools": {},
        }),
        EntityKind::World => json!({}),
        _ => json!({ "schema": kind.schema_name(), "items": [] }),
    }
}

/// A filled-in starter entity, for `ofm-cli add` and `ofm-cli schema`.
///
/// The values are illustrative on purpose — `"Man Utd"` teaches the shape in a
/// way `""` does not, which is why this is not derived from `Default`. What must
/// stay true is the *field set*: every field a definition serializes appears
/// here, enforced by the parity tests below.
pub fn entity_template(kind: EntityKind, name: Option<&str>) -> Value {
    let slug = name
        .map(slugify)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("my-{}", kind.schema_name()));
    let display = name.unwrap_or("My Entity");

    match kind {
        EntityKind::World => {
            let mut meta = new_package_meta(display, "", "1.0.0", "database");
            meta.id = slug;
            // An internal invariant: `WorldMetaDef` is a plain struct of plain
            // fields. Swallowing a failure here would emit a manifest with no
            // `schema` and no keys, which reads as a template rather than a bug.
            manifest_json(&meta).expect("package metadata serializes")
        }
        EntityKind::Team => {
            let short: String = display
                .split_whitespace()
                .filter_map(|w| w.chars().next())
                .take(3)
                .collect::<String>()
                .to_uppercase();
            json!({
                "id": slug,
                "name": display,
                "shortName": short,
                "city": "City",
                "country": "ENG",
                "colors": { "primary": "#cc0000", "secondary": "#ffffff" },
                "playStyle": "Balanced",
                "stadiumName": format!("{display} Arena"),
                "reputationRange": [300, 900],
                "financeRange": [500000, 10000000],
                "foundedYear": 1900,
                "kitPattern": "Solid",
                "logo": null,
            })
        }
        EntityKind::Player => {
            let mut parts = display.splitn(2, ' ');
            let first = parts.next().unwrap_or("First");
            let last = parts.next().unwrap_or("Last");
            json!({
                "id": slug,
                "name": display,
                "firstName": first,
                "lastName": last,
                // Empty, not a placeholder id: a scaffolded player has no team
                // to belong to yet, and validation reads empty as "unattached"
                // where `club-id` is a reference to a team that cannot exist.
                "club": "",
                "nationality": "ENG",
                // A `Position` variant, spelled the way the enum spells it. The
                // template said "CM", which is not a variant, so every player
                // `ofm-cli add player` created failed to deserialize at all.
                "position": "CentralMidfielder",
                "footedness": "Right",
                "dateOfBirth": "1995-01-01",
                "age": null,
                "youth": false,
                "overall": 70,
                // The career ceiling, independent of how ability above was
                // expressed. Null so the template shows the field while leaving
                // the engine's age-based roll in charge, which is what a
                // scaffolded player should default to.
                "potential": null,
                // Either `overall` or `attributes` may drive ability; an authored
                // attribute block wins. Null here so the template shows the field
                // without implying both must be filled in.
                "attributes": null,
                "photo": null,
            })
        }
        EntityKind::Staff => {
            let mut parts = display.splitn(2, ' ');
            let first = parts.next().unwrap_or("First");
            let last = parts.next().unwrap_or("Last");
            json!({
                "id": slug,
                "firstName": first,
                "lastName": last,
                // Empty for the same reason as the player template.
                "club": "",
                "nationality": "ENG",
                "role": "Coach",
                "specialization": null,
                "dateOfBirth": "1975-01-01",
                "age": null,
                "attributes": null,
            })
        }
        EntityKind::Confederation => json!({
            "id": slug,
            "name": display,
        }),
        // `ofm-cli add country "Brazil"` has to produce the country the game
        // already knows, not a second one under a slug of the same name: the id
        // is what teams, players and flags resolve against, so `brazil` and `BR`
        // are two different countries and only one of them works.
        EntityKind::Country => match crate::nations::nation_by_name(display) {
            Some(nation) => json!({
                "id": nation.code,
                "name": nation.name,
                "confederation": nation.region_id,
            }),
            // A name the catalog does not carry is a custom country, which is
            // allowed. Its confederation is left empty rather than guessed:
            // validation reads empty as "not set" and lets the world build
            // default it, where the old `confederation-id` placeholder was an id
            // that resolved to nothing and failed validation outright.
            None => json!({
                "id": slug,
                "name": display,
                "confederation": "",
            }),
        },
        EntityKind::Competition => json!({
            "id": slug,
            "name": display,
            "type": "League",
            "scope": "Domestic",
            "countryId": "ENG",
            "regionId": null,
            "requiredRegionIds": [],
            "priority": 1,
            "format": { "kind": "LeagueTable", "legs": 2 },
            "participants": {
                "selector": {
                    "kind": "topByReputation",
                    "country": "ENG",
                    "count": 20
                }
            },
            "berths": [],
            "seasonStartMonth": 8,
            "seasonStartDay": 1,
            "nameKey": null,
            "logo": null,
        }),
        // `NamePool` carries no `rename_all`, so its fields are snake_case while
        // every other entity is camelCase. The template said `firstNames` and
        // produced a file the loader could not deserialize at all — `ofm-cli add
        // names` failed validation on a package the same CLI had just created.
        EntityKind::Names => json!({
            "version": 1,
            "description": format!("{display} name pools"),
            "pools": {
                "ENG": {
                    "first_names": ["James", "Oliver", "Harry"],
                    "last_names": ["Smith", "Jones", "Williams"]
                }
            },
        }),
    }
}

/// Every field name each entity serializes, keyed by schema name.
///
/// The frontend defines its own `TeamDef`/`PlayerDef`/… in TypeScript and cannot
/// read Rust structs, so this is the handshake between them: it is generated
/// from the same fully-populated instances the parity tests use, checked into
/// the repo, and asserted from both sides. A field added to a definition here
/// fails the Rust test until the file is regenerated, and then fails the
/// frontend test until the editor's types learn about it.
///
/// Without it a field could reach the package format and never reach the editor.
/// Nothing is silently deleted when that happens — the editor sends the objects
/// it loaded straight back, and a JavaScript object keeps properties TypeScript
/// never declared. What is lost is the editor's ability to *see* the field: no
/// form can show it, no code can read it without a cast, and nobody adding a
/// feature knows it exists.
pub const SCHEMA_FIELDS_FIXTURE: &str = "src/components/menu/PackageEditor/schemaFields.generated.json";

/// Write `value` to `path` without leaving a half-written file behind.
///
/// Writes a `.tmp` sibling and renames over the target. The editor always did
/// this; the CLI did not, so an interrupted `ofm-cli new` could leave a truncated
/// manifest that then failed to parse. Sharing the scaffolder gives the CLI the
/// safer behaviour rather than giving the editor the weaker one.
fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &content).map_err(|e| e.to_string())?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Leaving the sibling behind is not harmless: `export_directory_to_ofm`
            // archives every file under the package, so a stray `.json.tmp` would
            // ship inside the `.ofm`.
            std::fs::remove_file(&tmp).ok();
            Err(e.to_string())
        }
    }
}

/// Create the directory skeleton and empty collection files for a new package.
///
/// The caller decides whether overwriting an existing directory is allowed —
/// the CLI refuses any existing directory, the editor refuses a non-empty one —
/// because that is a UI decision, not a format one.
pub fn scaffold_package(dir: &Path, meta: &WorldMetaDef) -> Result<(), String> {
    for kind in EntityKind::ALL {
        if let Some(sub) = kind.dir() {
            std::fs::create_dir_all(dir.join(sub)).map_err(|e| e.to_string())?;
        }
    }

    write_json_atomic(&dir.join(EntityKind::World.file_name()), &manifest_json(meta)?)?;

    for kind in EntityKind::ALL {
        let Some(sub) = kind.dir() else { continue };
        write_json_atomic(
            &dir.join(sub).join(kind.file_name()),
            &empty_collection(*kind),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::competition_def::{CompetitionDefinition, FormatDef, ParticipantSpec};
    use domain::league::{CompetitionFormat, CompetitionScope, CompetitionType};
    use domain::player::PlayerAttributes;
    use domain::staff::{CoachingSpecialization, StaffAttributes, StaffRole};
    use crate::generator::definitions::{NamePool, NamesDefinition, TeamColorsDef, TeamDef};
    use crate::generator::package::{ConfederationDef, CountryDef, PlayerDef, StaffDef};

    /// Top-level keys a value serializes to.
    fn keys(value: &Value) -> Vec<String> {
        value
            .as_object()
            .expect("expected a JSON object")
            .keys()
            .cloned()
            .collect()
    }

    /// Assert the template for `kind` offers every field the definition writes.
    ///
    /// `serialized` must come from a **fully populated** instance: several fields
    /// carry `skip_serializing_if`, so a `Default` instance would quietly omit
    /// exactly the optional fields most likely to be forgotten in a template.
    fn assert_template_covers(kind: EntityKind, serialized: &Value) {
        let template = entity_template(kind, Some("Sample Name"));
        let template_keys = keys(&template);
        let struct_keys = keys(serialized);

        let missing: Vec<String> = struct_keys
            .iter()
            .filter(|k| !template_keys.contains(k))
            .cloned()
            .collect();
        assert!(
            missing.is_empty(),
            "`ofm-cli` scaffolds {} without {missing:?}. A modder starting from this \
             template never learns those fields exist. Add them to `entity_template`.",
            kind.schema_name(),
        );

        // The other direction, which matters just as much: a key the template
        // invents is a key the loader ignores, so the scaffolded file quietly
        // does nothing. This is what a one-way check missed — the names template
        // wrote `firstNames` where `NamePool` (uniquely, having no `rename_all`)
        // expects `first_names`, so `ofm-cli add names` produced a file that
        // failed validation on a package the same CLI had just created.
        let invented: Vec<String> = template_keys
            .iter()
            .filter(|k| !struct_keys.contains(k))
            .cloned()
            .collect();
        assert!(
            invented.is_empty(),
            "the {} template writes {invented:?}, which the definition does not \
             deserialize. The scaffolded file would load as if those were absent.",
            kind.schema_name(),
        );
    }

    /// One fully-populated instance of every entity definition, serialized.
    ///
    /// The single place the exhaustive struct literals live, feeding both the
    /// template parity checks and the frontend fixture. Each literal is written
    /// **without** `..Default::default()` on purpose: adding a field to a
    /// definition stops this function compiling, so the field cannot be added
    /// without deciding what the template and the editor should do with it.
    ///
    /// Every value is populated rather than defaulted because several fields
    /// carry `skip_serializing_if` — a default instance would silently omit
    /// exactly the optional fields most likely to be forgotten downstream.
    fn populated_definitions() -> Vec<(EntityKind, Value)> {
        let team = TeamDef {
            id: "sample".into(),
            name: "Sample".into(),
            short_name: "SAM".into(),
            city: "City".into(),
            country: "ENG".into(),
            colors: TeamColorsDef {
                primary: "#cc0000".into(),
                secondary: "#ffffff".into(),
            },
            play_style: "Balanced".into(),
            stadium_name: "Sample Arena".into(),
            reputation_range: Some([300, 900]),
            finance_range: Some([500_000, 10_000_000]),
            logo: Some("assets/images/sample.png".into()),
            kit_pattern: Some("Solid".into()),
            founded_year: Some(1900),
        };

        let player = PlayerDef {
            id: "sample".into(),
            name: "Sample Player".into(),
            first_name: "Sample".into(),
            last_name: "Player".into(),
            club: "club-id".into(),
            nationality: "ENG".into(),
            position: Default::default(),
            date_of_birth: Some("1995-01-01".into()),
            age: Some(30),
            overall: Some(70),
            potential: Some(90),
            // Eight of the nineteen attributes carry a serde default; the other
            // eleven are required, so name those and let the defaults fill the
            // remainder. Only the presence of the `attributes` key matters here.
            attributes: Some(
                serde_json::from_value::<PlayerAttributes>(json!({
                    "pace": 50, "stamina": 50, "strength": 50,
                    "passing": 50, "shooting": 50, "tackling": 50,
                    "dribbling": 50, "defending": 50,
                    "positioning": 50, "vision": 50, "decisions": 50,
                }))
                .expect("a fully-specified attribute block deserializes"),
            ),
            photo: Some("assets/images/sample.png".into()),
            footedness: Some("Right".into()),
            youth: true,
        };

        let staff = StaffDef {
            id: "sample".into(),
            first_name: "Sample".into(),
            last_name: "Staffer".into(),
            club: "club-id".into(),
            nationality: "ENG".into(),
            role: StaffRole::Coach,
            attributes: Some(StaffAttributes {
                coaching: 10,
                judging_ability: 10,
                judging_potential: 10,
                physiotherapy: 10,
            }),
            specialization: Some(CoachingSpecialization::Fitness),
            date_of_birth: Some("1975-01-01".into()),
            age: Some(50),
        };

        let country = CountryDef {
            id: "ENG".into(),
            name: "England".into(),
            confederation: "europe".into(),
        };

        let confederation = ConfederationDef {
            id: "europe".into(),
            name: "Europe".into(),
        };

        // Names is the odd one out twice over: a single keyed document rather
        // than an `items` list, and the only definition with no `rename_all`, so
        // its fields are snake_case. Leaving it out of this list is how the
        // template came to say `firstNames`.
        let names = NamesDefinition {
            version: 1,
            description: "Sample pools".into(),
            pools: std::collections::HashMap::from([(
                "ENG".to_string(),
                NamePool {
                    first_names: vec!["James".into()],
                    last_names: vec!["Smith".into()],
                },
            )]),
        };

        let competition = CompetitionDefinition {
            id: "sample".into(),
            name: "Sample League".into(),
            r#type: CompetitionType::League,
            scope: CompetitionScope::Domestic,
            region_id: Some("europe".into()),
            country_id: Some("ENG".into()),
            required_region_ids: vec!["europe".into()],
            priority: 1,
            format: FormatDef {
                kind: CompetitionFormat::LeagueTable,
                legs: Some(2),
                group_size: None,
                qualifiers_per_group: None,
                best_third_qualifiers: None,
            },
            participants: ParticipantSpec::default(),
            berths: vec![],
            season_start_month: Some(8),
            season_start_day: Some(1),
            name_key: Some("competition.sample".into()),
            logo: Some("assets/images/sample.png".into()),
        };

        let mut competition_value = serde_json::to_value(&competition).unwrap();
        // `berths` is `skip_serializing_if` empty and is genuinely a list, so
        // rather than construct a berth just to see the key, assert it directly.
        competition_value
            .as_object_mut()
            .unwrap()
            .insert("berths".into(), json!([]));

        vec![
            (EntityKind::Team, serde_json::to_value(&team).unwrap()),
            (EntityKind::Player, serde_json::to_value(&player).unwrap()),
            (EntityKind::Staff, serde_json::to_value(&staff).unwrap()),
            (
                EntityKind::Country,
                serde_json::to_value(&country).unwrap(),
            ),
            (
                EntityKind::Confederation,
                serde_json::to_value(&confederation).unwrap(),
            ),
            (EntityKind::Competition, competition_value),
            (EntityKind::Names, serde_json::to_value(&names).unwrap()),
        ]
    }

    #[test]
    fn every_template_offers_every_field_its_definition_serializes() {
        // `foundedYear`, `kitPattern` and `logo` were missing from the team
        // template; `attributes`, `photo` and `age` from the player one —
        // `attributes` being the field that takes precedence over `overall`, so
        // the template hid the format's main authoring decision.
        //
        // Competitions are checked at the top level only: `format` and
        // `participants` are variant-shaped (a knockout carries different keys
        // from a league), so one template cannot cover every nested field and
        // should not pretend to.
        for (kind, serialized) in populated_definitions() {
            assert_template_covers(kind, &serialized);
        }
    }


    /// Naming every field correctly is not the same as filling them with values
    /// the definition can parse, and the key-parity check above cannot tell the
    /// difference. `ofm-cli add player` scaffolded `"position": "CM"`, which is
    /// not a `Position` variant — so the very next `ofm-cli validate` rejected a
    /// file the same CLI had just written, with an unknown-variant error naming a
    /// vocabulary that appears nowhere else in the format.
    #[test]
    fn every_template_deserializes_into_its_definition() {
        // Driven off `ALL` with an exhaustive match rather than a hand-listed set
        // of calls: a ninth `EntityKind` would silently escape a list, which is
        // the same way `position` escaped the key-parity check.
        //
        // Note what this can and cannot see. It catches a bad value in any field
        // backed by an enum — `Position`, `StaffRole`, `CoachingSpecialization`,
        // the competition type/scope/format. It cannot catch one in a field typed
        // as `String`, so `playStyle`, `kitPattern`, `footedness` and
        // `packageType` still rely on their annotated reference text being right.
        for kind in EntityKind::ALL {
            let value = entity_template(*kind, Some("Sample Name"));
            let parsed = match kind {
                EntityKind::World => serde_json::from_value::<WorldMetaDef>(value).map(drop),
                EntityKind::Team => serde_json::from_value::<TeamDef>(value).map(drop),
                EntityKind::Player => serde_json::from_value::<PlayerDef>(value).map(drop),
                EntityKind::Staff => serde_json::from_value::<StaffDef>(value).map(drop),
                EntityKind::Confederation => {
                    serde_json::from_value::<ConfederationDef>(value).map(drop)
                }
                EntityKind::Country => serde_json::from_value::<CountryDef>(value).map(drop),
                EntityKind::Competition => {
                    serde_json::from_value::<CompetitionDefinition>(value).map(drop)
                }
                EntityKind::Names => serde_json::from_value::<NamesDefinition>(value).map(drop),
            };
            parsed.unwrap_or_else(|error| {
                panic!(
                    "the {} template does not deserialize into its definition: {error}",
                    kind.schema_name()
                )
            });
        }
    }

    #[test]
    fn a_country_named_after_a_real_nation_scaffolds_as_that_nation() {
        // `ofm-cli add country "Brazil"` used to write id `brazil`, which is not
        // the id anything else in the game uses: teams, players and flags all
        // resolve against the catalog code. The package looked fine, and the
        // country it defined was a different one from the country its clubs
        // pointed at.
        let template = entity_template(EntityKind::Country, Some("Brazil"));
        assert_eq!(template["id"], "BR");
        assert_eq!(template["name"], "Brazil");
        assert_eq!(template["confederation"], "south-america");

        // The name is matched the way a person types it, not exactly.
        assert_eq!(entity_template(EntityKind::Country, Some("brazil"))["id"], "BR");
        assert_eq!(entity_template(EntityKind::Country, Some("BR"))["id"], "BR");
    }

    #[test]
    fn a_nation_whose_name_is_not_ascii_is_still_matched_case_insensitively() {
        // `eq_ignore_ascii_case` compares bytes, so `Ü` and `ü` are different
        // characters to it and `TÜRKIYE` would scaffold a *custom* country named
        // after one the catalog already has.
        assert_eq!(entity_template(EntityKind::Country, Some("TÜRKIYE"))["id"], "TR");
        assert_eq!(entity_template(EntityKind::Country, Some("türkiye"))["id"], "TR");
        assert_eq!(entity_template(EntityKind::Country, Some("CURAÇAO"))["id"], "CW");
        assert_eq!(
            entity_template(EntityKind::Country, Some("SÃO TOMÉ AND PRÍNCIPE"))["id"],
            "ST"
        );
    }

    #[test]
    fn a_country_the_catalog_does_not_know_keeps_the_name_it_was_given() {
        // Custom countries are legitimate — a historical state, an invented one.
        // What they must not carry is a confederation placeholder:
        // `confederation-id` resolved to nothing, so every custom country the
        // CLI scaffolded failed validation. Empty means "unset", which is
        // allowed and left to the world build.
        let template = entity_template(EntityKind::Country, Some("Yugoslavia"));
        assert_eq!(template["id"], "yugoslavia");
        assert_eq!(template["name"], "Yugoslavia");
        assert_eq!(template["confederation"], "");
    }

    #[test]
    fn the_names_template_round_trips_into_a_definition() {
        // Whatever `ofm-cli add names` writes has to deserialize back into a
        // definition with its pools intact. The template said `firstNames`, which
        // cannot become `first_names`, so the file failed validation on a package
        // the same CLI had just created.
        let template = entity_template(EntityKind::Names, Some("Brazil"));
        let parsed: NamesDefinition =
            serde_json::from_value(template).expect("the names template deserializes");
        let pool = parsed.pools.get("ENG").expect("the template ships a pool");
        assert!(!pool.first_names.is_empty() && !pool.last_names.is_empty());
    }

    #[test]
    fn the_names_template_survives_the_shape_the_cli_writes_it_in() {
        // `ofm-cli add` wraps a template in `{"schema": …, "items": [ … ]}` for
        // every kind. Names is a single keyed document rather than a list, so
        // this is the one place that wrapping could turn out to be meaningless.
        let dir = std::env::temp_dir().join(format!("ofm-names-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("names")).expect("temp dir");
        std::fs::write(
            dir.join("package.json"),
            r#"{"schema":"world","id":"n","name":"N","version":"1.0.0","license":"CC0-1.0"}"#,
        )
        .expect("manifest written");
        let template = entity_template(EntityKind::Names, Some("Brazil"));
        std::fs::write(
            dir.join("names").join("brazil.json"),
            serde_json::to_string(&json!({"schema": "names", "items": [template]})).unwrap(),
        )
        .expect("names written");

        let (package, errors) = crate::generator::load_world_package(&dir);

        assert!(errors.is_empty(), "the CLI's own output must load: {errors:?}");
        let names = package.names.expect("the pools have to reach the package");
        assert!(names.pools.contains_key("ENG"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_frontend_fixture_lists_every_field_every_definition_serializes() {
        // The editor defines its own TypeScript `TeamDef`/`PlayerDef`/… and
        // cannot read Rust structs. This file is the handshake: generated here
        // from the same populated instances, checked into the repo, asserted by
        // a vitest on the other side.
        //
        // It matters because the editor can only show what it can name. Nothing
        // is deleted when a field is missing here — the editor sends back the
        // objects it loaded, unknown properties included — but no form can
        // display it and no code can read it without a cast. `age` on a player
        // sat in that state: real, loadable, and invisible to the editor.
        //
        // Regenerate with:
        //   OFM_UPDATE_SCHEMA_FIXTURE=1 cargo test -p ofm_core --lib the_frontend_fixture
        let mut expected = serde_json::Map::new();
        for (kind, serialized) in populated_definitions() {
            let mut names: Vec<String> = keys(&serialized);
            names.sort();
            expected.insert(kind.schema_name().to_string(), json!(names));
        }
        let mut manifest_names: Vec<String> = keys(&serde_json::to_value(populated_meta()).unwrap());
        manifest_names.sort();
        expected.insert(EntityKind::World.schema_name().to_string(), json!(manifest_names));

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../")
            .join(SCHEMA_FIELDS_FIXTURE);
        let rendered = format!(
            "{}\n",
            serde_json::to_string_pretty(&Value::Object(expected.clone()))
                .expect("the fixture serializes")
        );

        if std::env::var("OFM_UPDATE_SCHEMA_FIXTURE").is_ok() {
            std::fs::create_dir_all(path.parent().expect("has a parent")).ok();
            std::fs::write(&path, &rendered).expect("the fixture is writable");
            return;
        }

        let on_disk = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Regenerate with \
                 OFM_UPDATE_SCHEMA_FIXTURE=1 cargo test -p ofm_core --lib the_frontend_fixture",
                path.display()
            )
        });
        assert_eq!(
            on_disk.trim(),
            rendered.trim(),
            "the package schema changed but {} is stale. Regenerate with \
             OFM_UPDATE_SCHEMA_FIXTURE=1 cargo test -p ofm_core --lib the_frontend_fixture, \
             then teach the editor's types about the new field.",
            SCHEMA_FIELDS_FIXTURE,
        );
    }

    /// A fully-populated manifest, for the same reasons as `populated_definitions`.
    fn populated_meta() -> WorldMetaDef {
        WorldMetaDef {
            id: "sample".into(),
            name: "Sample".into(),
            description: "A sample".into(),
            default_active_regions: vec!["europe".into()],
            default_active_competitions: vec!["sample".into()],
            base_year: Some(1962),
            version: "1.0.0".into(),
            author: "Author".into(),
            format_version: 1,
            license: "CC-BY-4.0".into(),
            game_min_version: "0.3.0".into(),
            package_type: "database".into(),
            logo: Some("assets/images/logo.png".into()),
            fallback_league: Some(Default::default()),
        }
    }

    #[test]
    fn the_manifest_offers_every_field_the_metadata_serializes() {
        // The drift that started this: `fallbackLeague` and `logo` reached the
        // editor's manifest (serialized from the struct) and never reached the
        // CLI's (a hand-written literal).
        let meta = populated_meta();
        let manifest = manifest_json(&meta).unwrap();
        let manifest_keys = keys(&manifest);
        let missing: Vec<String> = keys(&serde_json::to_value(&meta).unwrap())
            .into_iter()
            .filter(|k| !manifest_keys.contains(k))
            .collect();
        assert!(missing.is_empty(), "manifest omits {missing:?}");
        assert_eq!(manifest["schema"], json!("world"));
    }

    #[test]
    fn a_scaffolded_package_loads_clean() {
        // The strongest statement available: whatever we write, our own loader
        // accepts without a single complaint. A skeleton that needs repairing
        // before it validates is not a usable starting point.
        let dir = std::env::temp_dir().join(format!("ofm-scaffold-{}", uuid::Uuid::new_v4()));
        let meta = new_package_meta("Sample Package", "Author", "1.0.0", "database");
        scaffold_package(&dir, &meta).expect("scaffolding succeeds");

        let (package, errors) = crate::generator::load_world_package(&dir);

        assert!(errors.is_empty(), "a fresh package must validate: {errors:?}");
        assert_eq!(package.meta.expect("manifest loads").id, "sample-package");
        // The names stub has to be a real (empty) NamesDefinition, not merely a
        // file that parses. `ofm-cli new` used to write `{"schema":"names",
        // "items":[]}`, which loads with no error and is then silently dropped —
        // `names` came back `None`, so an author editing that file was editing
        // something the loader ignored.
        assert!(
            package.names.is_some(),
            "the scaffolded names file must load as a names definition, not be skipped"
        );
        std::fs::remove_dir_all(&dir).ok();
    }


    #[test]
    fn every_entity_kind_has_a_home_in_the_skeleton() {
        // Guards the enum against gaining a kind that scaffolding then ignores:
        // the file would never be created and the editor would show an empty tab
        // with no backing file.
        let dir = std::env::temp_dir().join(format!("ofm-scaffold-{}", uuid::Uuid::new_v4()));
        scaffold_package(&dir, &WorldMetaDef::default()).expect("scaffolding succeeds");

        for kind in EntityKind::ALL {
            let path = match kind.dir() {
                Some(sub) => dir.join(sub).join(kind.file_name()),
                None => dir.join(kind.file_name()),
            };
            assert!(path.exists(), "{} was not scaffolded", kind.schema_name());
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn slugify_matches_the_ids_the_loader_will_accept() {
        assert_eq!(slugify("Trendyol Süper Lig 25/26"), "trendyol-s-per-lig-25-26");
        assert_eq!(slugify("  Man Utd  "), "man-utd");
        assert_eq!(slugify("já--foi"), "j-foi");
        // Whatever it produces has to be usable as a filename, because the id
        // becomes `<id>.ofm` under the packages directory. A name carrying a
        // slash or a traversal token must not survive into the slug — that is
        // the shape of failure behind #414.
        for name in ["Süper Lig 25/26", "A/B", "../evil", "a\\b", "Ünïcodé"] {
            let slug = slugify(name);
            assert!(
                !slug.contains('/')
                    && !slug.contains('\\')
                    && !slug.contains("..")
                    && !slug.contains('\0'),
                "slugify({name:?}) produced an unusable id: {slug:?}"
            );
        }
    }

}
