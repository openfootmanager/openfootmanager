mod indices;
mod projection;

use crate::game::Game;
use crate::nations::region_for_code;
use indices::{domestic_leagues, index_league_membership, index_players_by_team};
use projection::project_card;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub const UNGROUPED_LEAGUE_ID: &str = "__ungrouped";

#[derive(Debug, Clone, Deserialize)]
pub struct TeamsDirectoryQuery {
    pub search: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TeamsDirectory {
    pub regions: Vec<RegionGroup>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RegionGroup {
    pub id: String,
    pub leagues: Vec<LeagueGroup>,
    pub team_count: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LeagueGroup {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_id: Option<String>,
    pub teams: Vec<TeamCard>,
}

/// Map key that groups teams by their league: `(id, name, name_key, country_id)`.
type LeagueKey = (String, String, Option<String>, Option<String>);

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TeamCard {
    pub team: TeamCardTeam,
    pub roster_size: usize,
    pub avg_ovr: u8,
    pub total_value: u64,
    pub league_pos: u32,
    pub standing: Option<TeamStanding>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TeamCardTeam {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub city: String,
    pub country: String,
    pub colors: TeamCardColors,
    pub formation: String,
    pub play_style: String,
    pub founded_year: u32,
    pub reputation: u32,
    pub media: TeamCardMedia,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TeamCardColors {
    pub primary: String,
    pub secondary: String,
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct TeamCardMedia {
    pub logo: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TeamStanding {
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub points: u32,
}

pub fn query_directory(game: &Game, query: &TeamsDirectoryQuery) -> TeamsDirectory {
    let players_by_team = index_players_by_team(&game.players);
    let leagues = domestic_leagues(game);
    let (league_by_team, standing_by_team) = index_league_membership(&leagues);

    let needle = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase);

    let mut regions: BTreeMap<String, BTreeMap<LeagueKey, Vec<TeamCard>>> = BTreeMap::new();

    for team in &game.teams {
        if let Some(needle) = &needle
            && !team.name.to_ascii_lowercase().contains(needle)
            && !team.city.to_ascii_lowercase().contains(needle)
        {
            continue;
        }

        let league = league_by_team.get(team.id.as_str()).copied();
        let region_id = league
            .and_then(|l| l.region_id.clone())
            .unwrap_or_else(|| region_for_code(&team.country).to_string());
        let (league_id, league_name, league_name_key, league_country_id) = league
            .map(|l| {
                (
                    l.id.clone(),
                    l.name.clone(),
                    l.name_key.clone(),
                    l.country_id.clone(),
                )
            })
            .unwrap_or_else(|| (UNGROUPED_LEAGUE_ID.to_string(), String::new(), None, None));

        let card = project_card(team, &players_by_team, &standing_by_team);

        regions
            .entry(region_id)
            .or_default()
            .entry((league_id, league_name, league_name_key, league_country_id))
            .or_default()
            .push(card);
    }

    let region_groups = regions
        .into_iter()
        .map(|(region_id, leagues_map)| build_region_group(region_id, leagues_map))
        .collect();
    TeamsDirectory { regions: region_groups }
}

fn build_region_group(
    region_id: String,
    leagues_map: BTreeMap<LeagueKey, Vec<TeamCard>>,
) -> RegionGroup {
    let mut leagues: Vec<LeagueGroup> = leagues_map
        .into_iter()
        .map(|((id, name, name_key, country_id), teams)| {
            build_league_group(id, name, name_key, country_id, teams)
        })
        .collect();

    leagues.sort_by(|a, b| match (a.id.as_str(), b.id.as_str()) {
        (UNGROUPED_LEAGUE_ID, UNGROUPED_LEAGUE_ID) => Ordering::Equal,
        (UNGROUPED_LEAGUE_ID, _) => Ordering::Greater,
        (_, UNGROUPED_LEAGUE_ID) => Ordering::Less,
        _ => a.name.cmp(&b.name),
    });

    let team_count = leagues.iter().map(|l| l.teams.len()).sum();
    RegionGroup { id: region_id, leagues, team_count }
}

fn build_league_group(
    id: String,
    name: String,
    name_key: Option<String>,
    country_id: Option<String>,
    mut teams: Vec<TeamCard>,
) -> LeagueGroup {
    teams.sort_by(|a, b| {
        let pa = if a.league_pos == 0 { u32::MAX } else { a.league_pos };
        let pb = if b.league_pos == 0 { u32::MAX } else { b.league_pos };
        pa.cmp(&pb).then_with(|| a.team.name.cmp(&b.team.name))
    });
    LeagueGroup { id, name, name_key, country_id, teams }
}
