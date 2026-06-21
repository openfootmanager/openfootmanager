use crate::game::Game;
use domain::league::CompetitionState;
use domain::world_history::WorldCupChampionRecord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Deserialize)]
pub struct CompetitionsQuery {}

/// Minimal player name info needed to display top-scorer rows.
#[derive(Debug, Serialize)]
pub struct PlayerNameEntry {
    pub match_name: String,
    pub full_name: String,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompetitionsView {
    pub competitions: Vec<CompetitionState>,
    /// Club team names keyed by ID (only teams appearing in competitions).
    pub team_names: BTreeMap<String, String>,
    /// National team names keyed by ID.
    pub national_team_names: BTreeMap<String, String>,
    /// i18n keys for national team nation names, keyed by national team ID.
    /// Frontend resolves these via `t(key)` and the `nations.nationalTeamTemplate`
    /// template rather than displaying the raw English `national_team_names` entry.
    pub national_team_name_keys: BTreeMap<String, String>,
    /// Name info for every player who has scored in a competition fixture.
    pub player_names: BTreeMap<String, PlayerNameEntry>,
    pub world_cup_champions: Vec<WorldCupChampionRecord>,
    pub manager_team_id: Option<String>,
    pub active_competition_ids: Vec<String>,
}

pub fn query_competitions(game: &Game, _query: &CompetitionsQuery) -> CompetitionsView {
    let competitions = game.competitions.clone();

    // Build club team lookup from all competitions' participants and fixtures.
    let club_team_ids: BTreeSet<&str> = competitions
        .iter()
        .flat_map(|c| {
            c.participant_ids
                .iter()
                .map(String::as_str)
                .chain(
                    c.fixtures
                        .iter()
                        .flat_map(|f| [f.home_team_id.as_str(), f.away_team_id.as_str()]),
                )
        })
        .collect();

    let team_map: BTreeMap<&str, &str> = game
        .teams
        .iter()
        .map(|t| (t.id.as_str(), t.name.as_str()))
        .collect();

    let team_names: BTreeMap<String, String> = team_map
        .iter()
        .filter(|(id, _)| club_team_ids.contains(*id))
        .map(|(id, name)| (id.to_string(), name.to_string()))
        .collect();

    let national_team_names: BTreeMap<String, String> = game
        .national_teams
        .iter()
        .map(|nt| (nt.id.clone(), nt.name.clone()))
        .collect();

    let national_team_name_keys: BTreeMap<String, String> = game
        .national_teams
        .iter()
        .filter_map(|nt| nt.name_key.as_ref().map(|key| (nt.id.clone(), key.clone())))
        .collect();

    // Collect scorer IDs across all competition fixtures.
    let scorer_ids: BTreeSet<&str> = competitions
        .iter()
        .flat_map(|c| c.fixtures.iter())
        .filter_map(|f| f.result.as_ref())
        .flat_map(|r| {
            r.home_scorers
                .iter()
                .chain(r.away_scorers.iter())
                .map(|s| s.player_id.as_str())
        })
        .collect();

    let player_names: BTreeMap<String, PlayerNameEntry> = game
        .players
        .iter()
        .filter(|p| scorer_ids.contains(p.id.as_str()))
        .map(|p| {
            let team_name = p
                .team_id
                .as_deref()
                .and_then(|tid| team_map.get(tid))
                .map(|n| n.to_string());
            (
                p.id.clone(),
                PlayerNameEntry {
                    match_name: p.match_name.clone(),
                    full_name: p.full_name.clone(),
                    team_id: p.team_id.clone(),
                    team_name,
                },
            )
        })
        .collect();

    CompetitionsView {
        competitions,
        team_names,
        national_team_names,
        national_team_name_keys,
        player_names,
        world_cup_champions: game.world_history.world_cup_champions.clone(),
        manager_team_id: game.manager.team_id.clone(),
        active_competition_ids: game.active_competition_ids.clone(),
    }
}
