mod indices;
mod projection;

use super::search_text::{contains_folded, fold_needle};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub division_tier: Option<u32>,
    pub teams: Vec<TeamCard>,
}

/// Map key that groups teams by league identity and tier metadata.
type LeagueKey = (String, String, Option<String>, Option<String>, Option<u32>);

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

/// A club's place in the league table it contests, for the team profile.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClubLeagueStanding {
    pub competition_id: String,
    pub competition_name: String,
    pub league_pos: u32,
    pub standing: TeamStanding,
}

pub fn query_club_league_standing(game: &Game, club_id: &str) -> Option<ClubLeagueStanding> {
    let table = crate::end_of_season::club_league_table(game, club_id)?;
    let sorted = table.sorted_standings();
    let index = sorted.iter().position(|entry| entry.team_id == club_id)?;
    Some(ClubLeagueStanding {
        competition_id: table.id.clone(),
        competition_name: table.name.clone(),
        league_pos: index as u32 + 1,
        standing: projection::project_standing(&sorted[index]),
    })
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
        .map(fold_needle);

    let mut regions: BTreeMap<String, BTreeMap<LeagueKey, Vec<TeamCard>>> = BTreeMap::new();

    for team in &game.teams {
        if let Some(needle) = &needle
            && !contains_folded(&team.name, needle)
            && !contains_folded(&team.city, needle)
        {
            continue;
        }

        let league = league_by_team.get(team.id.as_str()).copied();
        let region_id = league
            .and_then(|l| l.region_id.clone())
            .unwrap_or_else(|| region_for_code(&team.country).to_string());
        let (league_id, league_name, league_name_key, league_country_id, division_tier) = league
            .map(|l| {
                let division = l
                    .calendar
                    .as_ref()
                    .and_then(|calendar| calendar.season.division.as_ref());
                (
                    l.id.clone(),
                    l.name.clone(),
                    l.name_key.clone(),
                    l.country_id.clone(),
                    division
                        .map(|identity| identity.tier)
                        .filter(|tier| *tier > 0),
                )
            })
            .unwrap_or_else(|| {
                (
                    UNGROUPED_LEAGUE_ID.to_string(),
                    String::new(),
                    None,
                    None,
                    None,
                )
            });

        let card = project_card(team, &players_by_team, &standing_by_team);

        regions
            .entry(region_id)
            .or_default()
            .entry((
                league_id,
                league_name,
                league_name_key,
                league_country_id,
                division_tier,
            ))
            .or_default()
            .push(card);
    }

    let region_groups = regions
        .into_iter()
        .map(|(region_id, leagues_map)| build_region_group(region_id, leagues_map))
        .collect();
    TeamsDirectory {
        regions: region_groups,
    }
}

fn build_region_group(
    region_id: String,
    leagues_map: BTreeMap<LeagueKey, Vec<TeamCard>>,
) -> RegionGroup {
    let mut leagues: Vec<LeagueGroup> = leagues_map
        .into_iter()
        .map(|((id, name, name_key, country_id, division_tier), teams)| {
            build_league_group(id, name, name_key, country_id, division_tier, teams)
        })
        .collect();

    leagues.sort_by(|a, b| match (a.id.as_str(), b.id.as_str()) {
        (UNGROUPED_LEAGUE_ID, UNGROUPED_LEAGUE_ID) => Ordering::Equal,
        (UNGROUPED_LEAGUE_ID, _) => Ordering::Greater,
        (_, UNGROUPED_LEAGUE_ID) => Ordering::Less,
        _ => a.name.cmp(&b.name),
    });

    let team_count = leagues.iter().map(|l| l.teams.len()).sum();
    RegionGroup {
        id: region_id,
        leagues,
        team_count,
    }
}

fn build_league_group(
    id: String,
    name: String,
    name_key: Option<String>,
    country_id: Option<String>,
    division_tier: Option<u32>,
    mut teams: Vec<TeamCard>,
) -> LeagueGroup {
    teams.sort_by(|a, b| {
        let pa = if a.league_pos == 0 {
            u32::MAX
        } else {
            a.league_pos
        };
        let pb = if b.league_pos == 0 {
            u32::MAX
        } else {
            b.league_pos
        };
        pa.cmp(&pb).then_with(|| a.team.name.cmp(&b.team.name))
    });
    LeagueGroup {
        id,
        name,
        name_key,
        country_id,
        division_tier,
        teams,
    }
}
