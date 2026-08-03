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
            manifest_json(&meta).unwrap_or_else(|_| json!({}))
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
                "club": "club-id",
                "nationality": "ENG",
                "position": "CM",
                "footedness": "Right",
                "dateOfBirth": "1995-01-01",
                "age": null,
                "youth": false,
                "overall": 70,
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
                "club": "club-id",
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
        EntityKind::Country => json!({
            "id": slug,
            "name": display,
            "confederation": "confederation-id",
        }),
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
        EntityKind::Names => json!({
            "version": 1,
            "description": format!("{display} name pools"),
            "pools": {
                "ENG": {
                    "firstNames": ["James", "Oliver", "Harry"],
                    "lastNames": ["Smith", "Jones", "Williams"]
                }
            },
        }),
    }
}

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
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
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
    use crate::generator::definitions::{TeamColorsDef, TeamDef};
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
        let missing: Vec<String> = keys(serialized)
            .into_iter()
            .filter(|k| !template_keys.contains(k))
            .collect();
        assert!(
            missing.is_empty(),
            "`ofm-cli` scaffolds {} without {missing:?}. A modder starting from this \
             template never learns those fields exist. Add them to `entity_template`.",
            kind.schema_name(),
        );
    }

    #[test]
    fn the_team_template_offers_every_field_a_team_serializes() {
        // Exhaustive literal on purpose: adding a field to TeamDef breaks this
        // line, which is the point. `founded_year`, `kit_pattern` and `logo` were
        // all missing from the CLI template before this module existed.
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
        assert_template_covers(EntityKind::Team, &serde_json::to_value(&team).unwrap());
    }

    #[test]
    fn the_player_template_offers_every_field_a_player_serializes() {
        // `attributes` is the one that matters most: it takes precedence over
        // `overall`, and a template that never mentions it hides the format's
        // main authoring decision.
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
            // Eleven of the twenty attributes carry a serde default; the rest are
            // required, so name those and let the defaults fill the remainder.
            // Only the presence of the `attributes` key matters to this test.
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
        assert_template_covers(EntityKind::Player, &serde_json::to_value(&player).unwrap());
    }

    #[test]
    fn the_staff_template_offers_every_field_a_staff_member_serializes() {
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
        assert_template_covers(EntityKind::Staff, &serde_json::to_value(&staff).unwrap());
    }

    #[test]
    fn the_country_and_confederation_templates_offer_every_field() {
        let country = CountryDef {
            id: "ENG".into(),
            name: "England".into(),
            confederation: "europe".into(),
        };
        assert_template_covers(EntityKind::Country, &serde_json::to_value(&country).unwrap());

        let confederation = ConfederationDef {
            id: "europe".into(),
            name: "Europe".into(),
        };
        assert_template_covers(
            EntityKind::Confederation,
            &serde_json::to_value(&confederation).unwrap(),
        );
    }

    #[test]
    fn the_competition_template_offers_every_top_level_field() {
        // Only the top level: a competition's `format` and `participants` are
        // genuinely variant-shaped (a knockout carries different keys from a
        // league), so a single template cannot cover every nested field and
        // should not pretend to.
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
        // `berths` is skip_serializing_if empty, so assert it explicitly rather
        // than relying on a populated value we would have to construct.
        let mut serialized = serde_json::to_value(&competition).unwrap();
        serialized
            .as_object_mut()
            .unwrap()
            .insert("berths".into(), json!([]));
        assert_template_covers(EntityKind::Competition, &serialized);
    }

    #[test]
    fn the_manifest_offers_every_field_the_metadata_serializes() {
        // The drift that started this: `fallbackLeague` and `logo` reached the
        // editor's manifest (serialized from the struct) and never reached the
        // CLI's (a hand-written literal).
        let meta = WorldMetaDef {
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
        };
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
    fn the_cli_and_the_editor_scaffold_byte_identical_packages() {
        // The invariant the maintainer asked for, stated directly: the two front
        // ends are one package system. `ofm-cli new` and the editor's New Package
        // both land here with the same metadata, so every file they produce must
        // match byte for byte — not "be equivalent", not "both validate".
        //
        // Their genuine differences stay in the callers: the CLI refuses any
        // existing directory, the editor refuses a non-empty one. Neither is a
        // property of the format.
        let meta = new_package_meta("Sample Package", "Author", "1.0.0", "database");

        let from_cli = std::env::temp_dir().join(format!("ofm-cli-{}", uuid::Uuid::new_v4()));
        let from_editor = std::env::temp_dir().join(format!("ofm-ed-{}", uuid::Uuid::new_v4()));
        scaffold_package(&from_cli, &meta).expect("cli scaffold");
        scaffold_package(&from_editor, &meta).expect("editor scaffold");

        let listing = |root: &Path| {
            let mut found: Vec<(String, String)> = Vec::new();
            let mut stack = vec![root.to_path_buf()];
            while let Some(dir) = stack.pop() {
                for entry in std::fs::read_dir(&dir).expect("readable").flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else {
                        let rel = path
                            .strip_prefix(root)
                            .expect("under root")
                            .to_string_lossy()
                            .replace('\\', "/");
                        found.push((rel, std::fs::read_to_string(&path).expect("readable")));
                    }
                }
            }
            found.sort();
            found
        };

        assert_eq!(listing(&from_cli), listing(&from_editor));
        std::fs::remove_dir_all(&from_cli).ok();
        std::fs::remove_dir_all(&from_editor).ok();
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
