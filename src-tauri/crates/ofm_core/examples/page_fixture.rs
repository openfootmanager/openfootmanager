//! Writes the game the frontend's page harness mounts its pages on: a small career, generated from
//! a fixed seed and begun the way the app begins one, and each command's answer for it.
//!
//! `npm run fixture:pages` regenerates it from the repository root. The output is a TypeScript module, not
//! JSON, so that every value is checked against the frontend's hand-written type with `satisfies`:
//! a field Rust adds or renames, or one the frontend expects and Rust no longer sends, fails
//! `npm run build` instead of drifting silently.

use std::fmt::Write as _;

use domain::manager::Manager;
use ofm_core::career::{CareerScope, begin_career};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, ReputationBasis, WorldGenConfig, generate_world_data_seeded_with,
};
use ofm_core::slices::players::{PlayersPageQuery, query_page};
use ofm_core::slices::staff::query_staff;
use ofm_core::slices::teams::{TeamsDirectoryQuery, query_directory};
use ofm_core::world::build::{PastHistory, build_game_from_world_data};
use ofm_core::world::start_date_for_year;
use serde::Serialize;
use serde_json::{Value, json};

const SEED: u64 = 20_261_018;
/// A single-division nation, so the fixture is one league.
const NATION: &str = "SCO";
const CLUBS: usize = 8;
/// Not a World Cup year: those open on 1 June instead of 1 July.
const OPENING_YEAR: i32 = 2031;
/// Two past seasons give the history panels something to show without bloating the file.
const HISTORY_YEARS: u32 = 2;

fn main() -> Result<(), String> {
    let out_path = std::env::args()
        .nth(1)
        .ok_or("usage: page_fixture <output .ts path>")?;
    let game = build_career()?;
    let module = render_module(&game)?;
    std::fs::write(&out_path, &module).map_err(|error| format!("{out_path}: {error}"))?;
    println!("wrote {out_path} ({} KB)", module.len() / 1024);
    Ok(())
}

/// The app's path for a generated world: build the game, then begin the career at a club.
fn build_career() -> Result<Game, String> {
    let sources = DefinitionSources::embedded_only();
    let mut config = WorldGenConfig::standard();
    config.nations.retain(|nation| nation.code == NATION);
    config.clubs_per_division = CLUBS;
    // Left unset, players are aged and contracts dated by today's date, and the file would change
    // with the day it is generated on.
    config.opening_year = u32::try_from(OPENING_YEAR).ok();
    let world = generate_world_data_seeded_with(SEED, &config, &sources);

    let clock = GameClock::new(start_date_for_year(OPENING_YEAR)?);
    let manager = Manager::new(
        "page-harness-manager".to_string(),
        "Alex".to_string(),
        "Morgan".to_string(),
        "1980-04-12".to_string(),
        "Scotland".to_string(),
    );
    let history = PastHistory {
        start_year: OPENING_YEAR,
        depth_years: HISTORY_YEARS,
    };
    let (mut game, stats) =
        build_game_from_world_data(clock, manager, history, world, ReputationBasis::Stored);
    let club = ofm_core::world::strongest_club(&game.teams, NATION)
        .map(str::to_string)
        .ok_or_else(|| format!("the world has no {NATION} club"))?;
    begin_career(&mut game, &club, CareerScope::default(), stats)?;
    // What the app's state manager stamps on an installed career; the frontend sends it back.
    game.career_token = 1;
    Ok(game)
}

/// One export per command the harness answers, each checked against its frontend type.
fn render_module(game: &Game) -> Result<String, String> {
    let club = game
        .manager
        .team_id
        .clone()
        .ok_or("the career has no club")?;
    let squad = ofm_core::slices::squad::query_squad(game, &club);
    let squad_ids: Vec<String> = squad.iter().map(|entry| entry.player.id.clone()).collect();

    let mut module = String::from(HEADER);
    export(&mut module, "gameState", "GameStateData", game)?;
    export(&mut module, "squad", "PlayerData[]", &squad)?;
    export(
        &mut module,
        "squadFitness",
        "SquadFitness",
        &ofm_core::training::squad_fitness(game, &club),
    )?;
    export(
        &mut module,
        "clubLeagueStanding",
        "ClubLeagueStanding | null",
        &ofm_core::slices::teams::query_club_league_standing(game, &club),
    )?;
    export(
        &mut module,
        "contractWarningStages",
        "Record<string, ContractWarningStage | null>",
        &ofm_core::contracts::contract_warning_stages(game, &squad_ids),
    )?;
    export(
        &mut module,
        "financeSnapshot",
        "BackendFinanceSnapshotResponseData[\"snapshot\"]",
        &ofm_core::finances::team_finance_snapshot(game, &club)
            .ok_or("the club has no finances")?,
    )?;
    export(
        &mut module,
        "financePreviews",
        "BackendFinanceSnapshotResponseData[\"previews\"]",
        &ofm_core::finances::finance_action_previews(game, &club).unwrap_or_default(),
    )?;
    export(
        &mut module,
        "availableJobs",
        "JobOpportunity[]",
        &ofm_core::job_offers::get_available_jobs(game),
    )?;
    export(
        &mut module,
        "ratingTiers",
        "RatingTier[]",
        &ofm_core::player_tier::rating_tiers(),
    )?;
    export(
        &mut module,
        "positionGroups",
        "PositionGroupEntry[]",
        &ofm_core::position_group::position_groups(),
    )?;
    export(
        &mut module,
        "careerStage",
        "CareerStage",
        &ofm_core::career_stage::career_stage(game),
    )?;
    export_world_lists(&mut module, game, &club)?;
    Ok(module)
}

/// The world lists' answers: the Players page's first page, the Teams directory and the Staff page.
fn export_world_lists(module: &mut String, game: &Game, club: &str) -> Result<(), String> {
    // The query the Players page sends on opening (`DEFAULT_QUERY` in PlayersListTab.tsx). It is
    // exported with the page, and the harness answers only this query, so a page that starts
    // sending another one shows as unanswered rather than getting this page.
    let players_query = json!({
        "search": null, "position": null, "positions": [],
        "age_min": null, "age_max": null, "ovr_min": null, "ovr_max": null,
        "value_min": null, "value_max": null, "in_budget": false,
        "scope": "all", "team_id": null, "status": "all",
        "sort_key": "ovr", "sort_asc": false, "page": 1, "page_size": 30,
    });
    let parsed: PlayersPageQuery =
        serde_json::from_value(players_query.clone()).map_err(|error| error.to_string())?;
    export(
        module,
        "playersPageQuery",
        "PlayersPageQuery",
        &players_query,
    )?;
    export(
        module,
        "playersPage",
        "PlayersPage",
        &query_page(game, &parsed),
    )?;
    export(
        module,
        "teamsDirectory",
        "TeamsDirectory",
        &query_directory(game, &TeamsDirectoryQuery { search: None }),
    )?;
    export(module, "staff", "StaffSlice", &query_staff(game, club))
}

const HEADER: &str = "\
// Generated by `npm run fixture:pages` (src-tauri/crates/ofm_core/examples/page_fixture.rs).
// Do not edit: regenerate it. Every value `satisfies` the frontend type its command returns, so
// `npm run build` fails when the backend and the frontend stop agreeing on a shape.

import type { ClubLeagueStanding } from \"../../components/teamProfile/TeamProfile.types\";
import type { CareerStage } from \"../../services/careerService\";
import type { ContractWarningStage } from \"../../services/contractService\";
import type { BackendFinanceSnapshotResponseData } from \"../../services/financeService\";
import type { JobOpportunity } from \"../../services/jobService\";
import type { PlayersPage, PlayersPageQuery } from \"../../services/playersService\";
import type { PositionGroupEntry } from \"../../services/positionService\";
import type { RatingTier, SquadFitness } from \"../../services/squadService\";
import type { StaffSlice } from \"../../services/staffService\";
import type { TeamsDirectory } from \"../../services/teamsService\";
import type { GameStateData, PlayerData } from \"../../store/types\";
";

/// Appends `export const name = <value as JSON> satisfies ty;`. A JSON object or array is a
/// TypeScript literal, and a literal under `satisfies` keeps its literal types and is checked for
/// excess keys, which a typed JSON import is not.
fn export(module: &mut String, name: &str, ty: &str, value: &impl Serialize) -> Result<(), String> {
    // Through `Value` so a HashMap's keys come out sorted and the file is the same on every run.
    let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    write!(module, "\nexport const {name} = ").map_err(|error| error.to_string())?;
    write_layout(module, &value, 0)?;
    writeln!(module, " satisfies {ty};").map_err(|error| error.to_string())
}

/// Opens the top two levels onto their own lines and writes everything deeper compact: one key of
/// the game per line, one player or club per line. Fully pretty-printed, the file was 887 KB;
/// this keeps it near its compact size while a regenerated player still diffs as one line.
fn write_layout(out: &mut String, value: &Value, depth: usize) -> Result<(), String> {
    const OPENED_LEVELS: usize = 2;
    let indent = "  ".repeat(depth + 1);
    match value {
        Value::Object(map) if depth < OPENED_LEVELS && !map.is_empty() => {
            out.push_str("{\n");
            for (key, child) in map {
                let key = serde_json::to_string(key).map_err(|error| error.to_string())?;
                write!(out, "{indent}{key}: ").map_err(|error| error.to_string())?;
                write_layout(out, child, depth + 1)?;
                out.push_str(",\n");
            }
            write!(out, "{}}}", "  ".repeat(depth)).map_err(|error| error.to_string())
        }
        Value::Array(items) if depth < OPENED_LEVELS && !items.is_empty() => {
            out.push_str("[\n");
            for item in items {
                out.push_str(&indent);
                write_layout(out, item, depth + 1)?;
                out.push_str(",\n");
            }
            write!(out, "{}]", "  ".repeat(depth)).map_err(|error| error.to_string())
        }
        _ => {
            let compact = serde_json::to_string(value).map_err(|error| error.to_string())?;
            out.push_str(&compact);
            Ok(())
        }
    }
}

/// The first line where two texts part, 1-based, with each side's line ("" past its end); `None`
/// when they are the same.
#[cfg(test)]
fn first_difference<'a>(checked_in: &'a str, fresh: &'a str) -> Option<(usize, &'a str, &'a str)> {
    let mut checked_in_lines = checked_in.split('\n');
    let mut fresh_lines = fresh.split('\n');
    let mut number = 0;
    loop {
        number += 1;
        match (checked_in_lines.next(), fresh_lines.next()) {
            (None, None) => return None,
            (old, new) if old != new => {
                return Some((number, old.unwrap_or(""), new.unwrap_or("")));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The checked-in module, where `npm run fixture:pages` writes it from the repository root.
    const CHECKED_IN: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../src/dev/pageHarness/fixture.generated.ts"
    );

    /// Given page_fixture.rs, the world generator and the commands' answers as they stand
    /// When the page harness's career is generated afresh
    /// Then it is byte for byte the checked-in fixture.generated.ts, so a change on the Rust side
    /// can't land without the regenerated file the frontend's pages are mounted on.
    #[test]
    fn the_checked_in_fixture_matches_a_fresh_generation() {
        let checked_in = std::fs::read_to_string(CHECKED_IN).expect("read fixture.generated.ts");
        let game = build_career().expect("build the page harness career");
        let fresh = render_module(&game).expect("render the fixture module");

        if let Some((line, old, new)) = first_difference(&checked_in, &fresh) {
            panic!(
                "src/dev/pageHarness/fixture.generated.ts is stale: run `npm run fixture:pages` \
                 and commit it.\nFirst difference, line {line}:\n  checked in: {}\n  fresh:      {}",
                old.chars().take(200).collect::<String>(),
                new.chars().take(200).collect::<String>(),
            );
        }
    }

    /// Given two texts that part on their second line
    /// When they are compared
    /// Then the second line is reported, with each side's text.
    #[test]
    fn the_first_differing_line_is_reported() {
        assert_eq!(first_difference("a\nb\nc", "a\nB\nc"), Some((2, "b", "B")));
    }

    /// Given one text that is the other with a line added at its end
    /// When they are compared
    /// Then the added line is reported against an empty one.
    #[test]
    fn a_line_added_at_the_end_is_reported() {
        assert_eq!(first_difference("a\n", "a\nb\n"), Some((2, "", "b")));
    }

    /// Given two identical texts
    /// When they are compared
    /// Then no difference is reported.
    #[test]
    fn identical_texts_have_no_difference() {
        assert_eq!(first_difference("a\nb\n", "a\nb\n"), None);
    }
}
