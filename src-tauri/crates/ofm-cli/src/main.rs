use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use ofm_core::generator::{
    entity_template, export_directory_to_ofm, load_world_package, load_world_package_from_ofm,
    new_package_meta, read_package_manifest_from_ofm, scaffold_package, slugify, validate_package,
    EntityKind as CoreEntityKind,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "ofm-cli",
    version,
    about = "OpenFoot Manager — package authoring tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new package project directory
    New {
        name: String,
        /// Output directory (default: ./<name>)
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long, default_value = "")]
        author: String,
        #[arg(long, default_value = "1.0.0")]
        version: String,
        #[arg(long, default_value = "database", value_parser = ["database", "patch", "assets"])]
        r#type: String,
    },
    /// Print an annotated schema template for an entity type
    Schema {
        entity: EntityKind,
    },
    /// Add a scaffolded entity file to an existing package directory
    Add {
        entity: EntityKind,
        /// Display name: pre-fills the entity and generates the filename slug
        name: Option<String>,
        /// Package directory (default: current directory)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Append to this file in the entity subdirectory instead of creating a new file
        #[arg(long)]
        append_to: Option<String>,
        /// Output format for new files
        #[arg(long, default_value = "json", value_parser = ["json", "yaml"])]
        format: String,
    },
    /// Validate a package directory or .ofm archive
    Validate {
        path: PathBuf,
    },
    /// Pack a package directory into a .ofm archive
    Pack {
        dir: PathBuf,
        /// Output path (default: <id>.ofm in current directory)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Show metadata from a .ofm file
    Info {
        file: PathBuf,
    },
}

/// Clap's view of the entity list. It exists only so `clap` can parse the
/// argument; every question about what an entity *is* goes to `ofm_core`.
#[derive(ValueEnum, Clone, Debug)]
enum EntityKind {
    World,
    Team,
    Player,
    Staff,
    Confederation,
    Country,
    Competition,
    Names,
}

impl From<&EntityKind> for CoreEntityKind {
    fn from(kind: &EntityKind) -> Self {
        match kind {
            EntityKind::World => CoreEntityKind::World,
            EntityKind::Team => CoreEntityKind::Team,
            EntityKind::Player => CoreEntityKind::Player,
            EntityKind::Staff => CoreEntityKind::Staff,
            EntityKind::Confederation => CoreEntityKind::Confederation,
            EntityKind::Country => CoreEntityKind::Country,
            EntityKind::Competition => CoreEntityKind::Competition,
            EntityKind::Names => CoreEntityKind::Names,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Commands::New {
            name,
            dir,
            author,
            version,
            r#type,
        } => cmd_new(&name, dir.as_deref(), &author, &version, &r#type),
        Commands::Schema { entity } => cmd_schema(&entity),
        Commands::Add {
            entity,
            name,
            dir,
            append_to,
            format,
        } => cmd_add(
            &entity,
            name.as_deref(),
            dir.as_deref(),
            append_to.as_deref(),
            &format,
        ),
        Commands::Validate { path } => cmd_validate(&path),
        Commands::Pack { dir, output } => cmd_pack(&dir, output.as_deref()),
        Commands::Info { file } => cmd_info(&file),
    };
    std::process::exit(exit_code);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------





fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Schema constants — annotated JSONC reference (not loadable JSON)
// ---------------------------------------------------------------------------

const SCHEMA_WORLD: &str = r##"// World manifest — save as package.json at the root of your package directory.
// This is an annotated reference. Use 'ofm-cli new' to scaffold a real file.
{
  "schema": "world",           // required: always "world"
  "id": "my-package",          // required: stable slug used for file naming
  "name": "My Package",        // required: display name shown in-game
  "description": "",           // optional: short description
  "version": "1.0.0",          // required: semver string
  "author": "Your Name",       // required
  "license": "CC-BY-4.0",      // required: SPDX identifier
  "packageType": "database",   // required: "database" | "patch" | "assets"
  "gameMinVersion": "",        // optional: minimum game version, e.g. "0.3.0"
  "formatVersion": 1,          // required: always 1
  "baseYear": null,             // optional: integer season year, e.g. 2024
  "defaultActiveRegions": [],  // optional: region IDs enabled by default
  "defaultActiveCompetitions": [], // optional: competition IDs enabled by default
  "fallbackLeague": null       // optional: overrides for the auto-generated league
                               // when a database package has teams but no
                               // competitions, e.g.
                               // { "name": "My League", "legs": 2,
                               //   "scope": "Domestic" }
}"##;

const SCHEMA_TEAM: &str = r##"// Team entity — place inside teams/*.json in the "items" array.
// This is an annotated reference. Use 'ofm-cli add team "Club Name"' to scaffold a real file.
{
  "id": "my-club",            // required: stable slug (auto-uuid if empty)
  "name": "My Club FC",       // required: full display name
  "shortName": "MCF",         // required: 2-5 char abbreviation
  "city": "London",           // required
  "country": "ENG",           // required: country ID matching your countries/*.json
  "colors": {
    "primary": "#cc0000",     // required: hex color
    "secondary": "#ffffff"    // required: hex color
  },
  "playStyle": "Balanced",    // optional: "Balanced" | "Attacking" | "Defensive" | "Counter" | "Pressing"
  "stadiumName": "My Arena",  // optional
  "reputationRange": [300, 900],      // optional: [min, max] 0-1000
  "financeRange": [500000, 10000000], // optional: [min, max] budget in euros
  "logo": null                // optional: relative path to logo image asset
}"##;

const SCHEMA_PLAYER: &str = r##"// Player entity — place inside players/*.json in the "items" array.
// This is an annotated reference. Use 'ofm-cli add player "First Last"' to scaffold a real file.
{
  "id": "player-slug",        // required: stable slug (auto-uuid if empty)
  "name": "First Last",       // required: full display name
  "firstName": "First",       // required
  "lastName": "Last",         // required
  "club": "club-id",          // required: team ID matching your teams/*.json
  "nationality": "ENG",       // required: country ID
  "position": "CM",           // required: GK | CB | LB | RB | CDM | CM | CAM | LW | RW | ST
  "footedness": "Right",      // optional: Right | Left | Both (defaults to Right)
  "dateOfBirth": "1995-01-01",// optional: ISO date (YYYY-MM-DD)
  "age": null,                // optional: integer (used if dateOfBirth absent)
  "youth": false,             // optional: true places the player in the club's youth/academy squad
  "photo": null,              // optional: relative path to a player photo asset
  "overall": 70,              // optional: 1-99 overall rating
  "attributes": null          // optional: detailed attribute object
}"##;

const SCHEMA_STAFF: &str = r##"// Staff entity — place inside staff/*.json in the "items" array.
// This is an annotated reference. Use 'ofm-cli add staff "First Last"' to scaffold a real file.
{
  "id": "staff-slug",         // required: stable slug (auto-uuid if empty)
  "firstName": "First",       // required
  "lastName": "Last",         // required
  "club": "club-id",          // optional: team ID; empty/omitted = unattached / free agent
  "nationality": "ENG",       // required: country ID
  "role": "Coach",            // required: AssistantManager | Coach | Scout | Physio
  "specialization": null,     // optional (coaches): Fitness | Technique | Tactics | Defending
                              //            | Attacking | GoalKeeping | Youth
  "dateOfBirth": "1975-01-01",// optional: ISO date (YYYY-MM-DD)
  "age": null,                // optional: integer (used if dateOfBirth absent)
  "attributes": null          // optional: { "coaching": 60, "judgingAbility": 60,
                              //             "judgingPotential": 60, "physiotherapy": 60 }
}"##;

const SCHEMA_CONFEDERATION: &str = r##"// Confederation entity — place inside confederations/*.json in the "items" array.
// This is an annotated reference. Use 'ofm-cli add confederation "Name"' to scaffold a real file.
{
  "id": "uefa",  // required: stable slug
  "name": "UEFA" // required: display name
}"##;

const SCHEMA_COUNTRY: &str = r##"// Country entity — place inside countries/*.json in the "items" array.
// This is an annotated reference. Use 'ofm-cli add country "Name"' to scaffold a real file.
{
  "id": "ENG",               // required: stable slug / country code
  "name": "England",         // required: display name
  "confederation": "uefa"    // required: confederation ID matching your confederations/*.json
}"##;

const SCHEMA_COMPETITION: &str = r##"// Competition entity — place inside competitions/*.json in the "items" array.
// This is an annotated reference. Use 'ofm-cli add competition "Name"' to scaffold a real file.
{
  "id": "eng-premier-league",  // required: stable slug
  "name": "Premier League",    // required: display name
  "type": "League",            // required: "League" | "Cup" | "ContinentalClub" |
                                //   "InternationalClub" | "InternationalNation" | "FriendlyCup"
  "scope": "Domestic",         // required: "Domestic" | "Regional" | "Continental" | "International"
  "countryId": "ENG",          // optional: country ID (Domestic/Regional competitions)
  "regionId": null,            // optional: region ID (Regional/Continental competitions)
  "priority": 1,               // optional: scheduling priority (higher = scheduled first)
  "format": {
    "kind": "LeagueTable",     // required: "LeagueTable" | "Knockout" | "GroupAndKnockout"
    "legs": 2                  // optional: round-robin legs for LeagueTable/groups (default 2)
    // GroupAndKnockout only:
    // "groupSize": 4,         // clubs per group
    // "qualifiersPerGroup": 2,// clubs advancing per group
    // "bestThirdQualifiers": 0// best runners-up also advancing
  },
  // Participant selection - use "selector" for rule-based or "explicit" for fixed list:
  "participants": {
    "selector": {
      "kind": "topByReputation", // "topByReputation" | "allInCountry" | "allInRegion" | "championsOf"
      "country": "ENG",          // required for topByReputation / allInCountry
      "count": 20                // number of teams to select
      // "region": "europe",     // required for allInRegion
      // "sourceCompetition": "eng-championship" // required for ChampionsOf
    }
    // OR fixed list:
    // "explicit": ["club-a", "club-b", "club-c"]
  },
  "seasonStartMonth": 8,  // optional: 1-12 (default 8 = August)
  "seasonStartDay": 1,    // optional: 1-31 (default 1)
  // Berths - qualification spots this competition awards into others:
  "berths": [
    // { "rule": "TopN", "count": 4, "targetCompetition": "ucl", "fromTop": true }
  ]
}"##;

const SCHEMA_NAMES: &str = r##"// Names entity — place inside names/*.json in the "items" array.
// Each entry is a NamesDefinition providing locale-specific name pools.
// This is an annotated reference. Use 'ofm-cli add names' to scaffold a real file.
{
  "version": 1,                 // required: always 1
  "description": "Name pools",  // optional
  "pools": {
    // One entry per country ID. The engine merges all pools from all packages.
    "ENG": {
      "first_names": ["James", "Oliver", "Harry"],
      "last_names":  ["Smith", "Jones", "Williams"]
    },
    "BRA": {
      "first_names": ["Lucas", "Gabriel", "Pedro"],
      "last_names":  ["Silva", "Santos", "Oliveira"]
    }
  }
}"##;

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_new(name: &str, dir: Option<&Path>, author: &str, version: &str, pkg_type: &str) -> i32 {
    let pkg_dir = dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(name));

    // The CLI refuses any existing directory; the editor refuses only a
    // non-empty one. That difference is a UI choice, so it stays here rather
    // than in the shared scaffolder.
    if pkg_dir.exists() {
        eprintln!(
            "{} Directory already exists: {}",
            "error:".red().bold(),
            pkg_dir.display()
        );
        return 1;
    }

    let meta = new_package_meta(name, author, version, pkg_type);
    if let Err(e) = scaffold_package(&pkg_dir, &meta) {
        eprintln!("{} Failed to create package: {}", "error:".red().bold(), e);
        return 1;
    }

    println!("{} Created package directory: {}", "✓".green().bold(), pkg_dir.display());
    println!("  Edit {} to fill in metadata.", "package.json".cyan());
    println!("  Add entities with {}.", "ofm-cli add <entity> \"Name\"".cyan());
    println!("  Validate with {}.", "ofm-cli validate .".cyan());
    0
}

fn entity_schema_text(entity: &EntityKind) -> &'static str {
    match entity {
        EntityKind::World => SCHEMA_WORLD,
        EntityKind::Team => SCHEMA_TEAM,
        EntityKind::Player => SCHEMA_PLAYER,
        EntityKind::Staff => SCHEMA_STAFF,
        EntityKind::Confederation => SCHEMA_CONFEDERATION,
        EntityKind::Country => SCHEMA_COUNTRY,
        EntityKind::Competition => SCHEMA_COMPETITION,
        EntityKind::Names => SCHEMA_NAMES,
    }
}

fn cmd_schema(entity: &EntityKind) -> i32 {
    println!("{}", entity_schema_text(entity));
    0
}

fn cmd_add(
    entity: &EntityKind,
    name: Option<&str>,
    dir: Option<&Path>,
    append_to: Option<&str>,
    format: &str,
) -> i32 {
    if matches!(entity, EntityKind::World) {
        eprintln!(
            "{} Use 'ofm-cli new' to create a world manifest, not 'add'.",
            "error:".red().bold()
        );
        return 1;
    }

    let pkg_dir = dir.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let kind = CoreEntityKind::from(entity);
    // `World` is rejected above, so every kind reaching here has a directory.
    let sub = kind.dir().expect("non-world entities live in a subdirectory");
    let schema = kind.schema_name();
    let template = entity_template(kind, name);

    if let Some(target_file) = append_to {
        // --append-to names a file inside the entity subdirectory; reject path
        // separators / traversal so it can't escape into the wider filesystem.
        if target_file.is_empty()
            || target_file.contains('/')
            || target_file.contains('\\')
            || target_file.contains("..")
        {
            eprintln!(
                "{} --append-to must be a plain filename inside {}/",
                "error:".red().bold(),
                sub
            );
            return 1;
        }
        let target = pkg_dir.join(sub).join(target_file);
        if !target.exists() {
            eprintln!("{} File not found: {}", "error:".red().bold(), target.display());
            return 1;
        }
        let raw = match std::fs::read_to_string(&target) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{} Failed to read {}: {}", "error:".red().bold(), target.display(), e);
                return 1;
            }
        };
        let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("json");
        let mut value: Value = if ext == "yaml" || ext == "yml" {
            match serde_yaml::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{} Failed to parse YAML: {}", "error:".red().bold(), e);
                    return 1;
                }
            }
        } else {
            match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{} Failed to parse JSON: {}", "error:".red().bold(), e);
                    return 1;
                }
            }
        };

        if value.get("items").is_none() {
            value["items"] = json!([]);
        }
        match value["items"].as_array_mut() {
            Some(arr) => arr.push(template),
            None => {
                eprintln!(
                    "{} 'items' is not an array in {}",
                    "error:".red().bold(),
                    target.display()
                );
                return 1;
            }
        }

        let result = if ext == "yaml" || ext == "yml" {
            serde_yaml::to_string(&value)
                .map_err(|e| e.to_string())
                .and_then(|s| std::fs::write(&target, s).map_err(|e| e.to_string()))
        } else {
            write_json(&target, &value)
        };
        if let Err(e) = result {
            eprintln!("{} Failed to write {}: {}", "error:".red().bold(), target.display(), e);
            return 1;
        }
        println!("{} Appended {} to {}", "✓".green().bold(), schema, target.display());
    } else {
        let slug = name
            .map(slugify)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("new-{}", schema));
        let filename = if format == "yaml" {
            format!("{}.yaml", slug)
        } else {
            format!("{}.json", slug)
        };
        let entity_subdir = pkg_dir.join(sub);
        let target = entity_subdir.join(&filename);

        if target.exists() {
            eprintln!(
                "{} File already exists: {}\n  Use {} to append to it instead.",
                "error:".red().bold(),
                target.display(),
                format!("--append-to {}", filename).cyan()
            );
            return 1;
        }

        if !entity_subdir.exists() {
            if let Err(e) = std::fs::create_dir_all(&entity_subdir) {
                eprintln!("{} Failed to create directory: {}", "error:".red().bold(), e);
                return 1;
            }
        }

        let container = json!({ "schema": schema, "items": [template] });

        let result = if format == "yaml" {
            serde_yaml::to_string(&container)
                .map_err(|e| e.to_string())
                .and_then(|s| std::fs::write(&target, s).map_err(|e| e.to_string()))
        } else {
            write_json(&target, &container)
        };
        if let Err(e) = result {
            eprintln!("{} Failed to write {}: {}", "error:".red().bold(), target.display(), e);
            return 1;
        }
        println!("{} Created {} at {}", "✓".green().bold(), schema, target.display());
    }
    0
}

fn cmd_validate(path: &Path) -> i32 {
    if !path.exists() {
        eprintln!("{} Path not found: {}", "error:".red().bold(), path.display());
        return 1;
    }
    println!("Validating {}...", path.display());

    let is_ofm = path.extension().and_then(|e| e.to_str()) == Some("ofm");
    let (pkg, mut errors) = if is_ofm {
        load_world_package_from_ofm(path)
    } else {
        load_world_package(path)
    };
    if is_ofm {
        // The archive load is the runtime read path and stays permissive on
        // purpose, so that an already-installed package keeps working. Validating
        // is this command's whole job, though, and a `.ofm` is the shape a
        // package is actually shared in — so ask for the full set explicitly.
        // Same set the directory branch gets, from the same place, so the two
        // answers cannot diverge for the same package.
        errors.extend(validate_package(&pkg));
    }

    if errors.is_empty() {
        println!(
            "{} Valid — {} teams, {} players, {} competitions, {} countries, {} confederations",
            "✓".green().bold(),
            pkg.teams.len(),
            pkg.players.len(),
            pkg.competitions.len(),
            pkg.countries.len(),
            pkg.confederations.len(),
        );
        0
    } else {
        println!("{} {} error(s):", "✗".red().bold(), errors.len());
        for err in &errors {
            let params: Vec<String> =
                err.params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            let param_str = if params.is_empty() {
                String::new()
            } else {
                format!(" ({})", params.join(", "))
            };
            println!("  {} {}{}", err.file.yellow(), err.code.red(), param_str);
        }
        1
    }
}

fn cmd_pack(dir: &Path, output: Option<&Path>) -> i32 {
    if !dir.exists() {
        eprintln!("{} Directory not found: {}", "error:".red().bold(), dir.display());
        return 1;
    }

    println!("Validating {}...", dir.display());
    let (pkg, errors) = load_world_package(dir);
    if !errors.is_empty() {
        println!("{} Validation failed — fix these errors before packing:", "✗".red().bold());
        for err in &errors {
            let params: Vec<String> =
                err.params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            let param_str = if params.is_empty() {
                String::new()
            } else {
                format!(" ({})", params.join(", "))
            };
            println!("  {} {}{}", err.file.yellow(), err.code.red(), param_str);
        }
        return 1;
    }

    let id = pkg
        .meta
        .as_ref()
        .map(|m| m.id.clone())
        .unwrap_or_else(|| dir.file_name().and_then(|n| n.to_str()).unwrap_or("package").to_string());

    let out_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("{}.ofm", id)));

    println!("Packing → {}...", out_path.display());
    match export_directory_to_ofm(dir, &out_path) {
        Ok(()) => {
            let size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
            println!(
                "{} Packed {} ({} KB)",
                "✓".green().bold(),
                out_path.display(),
                size / 1024
            );
            0
        }
        Err(e) => {
            eprintln!("{} Pack failed: {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_info(file: &Path) -> i32 {
    if !file.exists() {
        eprintln!("{} File not found: {}", "error:".red().bold(), file.display());
        return 1;
    }

    let meta = read_package_manifest_from_ofm(file);
    let (pkg, errors) = load_world_package_from_ofm(file);

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    if let Some(m) = &meta {
        table.add_row(vec!["ID", &m.id]);
        table.add_row(vec!["Name", &m.name]);
        table.add_row(vec!["Version", &m.version]);
        table.add_row(vec!["Author", &m.author]);
        table.add_row(vec!["Type", &m.package_type]);
        table.add_row(vec!["License", &m.license]);
        if !m.game_min_version.is_empty() {
            table.add_row(vec!["Min game version", &m.game_min_version]);
        }
        if let Some(year) = m.base_year {
            table.add_row(vec!["Base year", &year.to_string()]);
        }
        table.add_row(vec!["Description", &m.description]);
    } else {
        table.add_row(vec!["Manifest", "not found"]);
    }

    table.add_row(vec!["Teams", &pkg.teams.len().to_string()]);
    table.add_row(vec!["Players", &pkg.players.len().to_string()]);
    table.add_row(vec!["Competitions", &pkg.competitions.len().to_string()]);
    table.add_row(vec!["Countries", &pkg.countries.len().to_string()]);
    table.add_row(vec!["Confederations", &pkg.confederations.len().to_string()]);

    if !errors.is_empty() {
        table.add_row(vec!["Errors", &errors.len().to_string()]);
    }

    println!("{}", table);

    if !errors.is_empty() {
        println!("{} {} validation error(s)", "⚠".yellow().bold(), errors.len());
        for err in &errors {
            println!("  {} {}", err.file.yellow(), err.code.red());
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every authorable entity in the World Editor must round-trip through the CLI
    // helpers. This guards against adding a UI entity (e.g. staff) without wiring
    // its directory, schema name, template, and annotated schema into the CLI.
    const ALL_ENTITIES: &[EntityKind] = &[
        EntityKind::World,
        EntityKind::Team,
        EntityKind::Player,
        EntityKind::Staff,
        EntityKind::Confederation,
        EntityKind::Country,
        EntityKind::Competition,
        EntityKind::Names,
    ];

    #[test]
    fn every_entity_has_a_nonempty_annotated_schema() {
        for entity in ALL_ENTITIES {
            assert!(
                !entity_schema_text(entity).trim().is_empty(),
                "{:?} is missing an annotated schema",
                entity
            );
        }
    }

    #[test]
    fn staff_entity_maps_to_staff_dir_and_schema() {
        let kind = CoreEntityKind::from(&EntityKind::Staff);
        assert_eq!(kind.dir(), Some("staff"));
        assert_eq!(kind.schema_name(), "staff");
    }

    #[test]
    fn staff_template_carries_role_and_split_name() {
        let tpl = entity_template(CoreEntityKind::from(&EntityKind::Staff), Some("Alex Ferguson"));
        assert_eq!(tpl["id"], "alex-ferguson");
        assert_eq!(tpl["firstName"], "Alex");
        assert_eq!(tpl["lastName"], "Ferguson");
        assert_eq!(tpl["role"], "Coach");
    }

    #[test]
    fn player_template_exposes_footedness_and_youth() {
        let tpl = entity_template(CoreEntityKind::from(&EntityKind::Player), Some("Sam Doe"));
        assert_eq!(tpl["footedness"], "Right");
        assert_eq!(tpl["youth"], false);
    }

    #[test]
    fn every_cli_entity_maps_to_a_distinct_core_kind() {
        // The clap enum exists only to parse an argument; everything downstream
        // asks `ofm_core`. If this mapping ever collided, `ofm-cli add country`
        // would happily scaffold a confederation and nothing else would notice.
        let cli = [
            EntityKind::World,
            EntityKind::Team,
            EntityKind::Player,
            EntityKind::Staff,
            EntityKind::Confederation,
            EntityKind::Country,
            EntityKind::Competition,
            EntityKind::Names,
        ];
        let mapped: Vec<CoreEntityKind> = cli.iter().map(CoreEntityKind::from).collect();

        // Same length as the core list: neither side may gain a kind alone.
        assert_eq!(mapped.len(), CoreEntityKind::ALL.len());
        for kind in CoreEntityKind::ALL {
            assert!(
                mapped.contains(kind),
                "{} is a package entity the CLI cannot name",
                kind.schema_name()
            );
        }
    }
}
