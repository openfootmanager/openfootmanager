use crate::game::Game;
use domain::league::{
    CompetitionScope, CompetitionType, League, StandingEntry,
};
use domain::player::Player;
use std::collections::HashMap;

pub(super) type LeagueByTeam<'a> = HashMap<&'a str, &'a League>;
pub(super) type StandingByTeam<'a> = HashMap<&'a str, (u32, &'a StandingEntry)>;

pub(super) fn index_players_by_team(players: &[Player]) -> HashMap<&str, Vec<&Player>> {
    let mut by_team: HashMap<&str, Vec<&Player>> = HashMap::new();
    for player in players {
        if let Some(team_id) = &player.team_id {
            by_team.entry(team_id.as_str()).or_default().push(player);
        }
    }
    by_team
}

pub(super) fn domestic_leagues(game: &Game) -> Vec<&League> {
    let filtered: Vec<&League> = game
        .competitions
        .iter()
        .filter(|c| {
            c.kind == CompetitionType::League && c.scope == CompetitionScope::Domestic
        })
        .collect();
    if !filtered.is_empty() {
        return filtered;
    }
    game.league.as_ref().map(|l| vec![l]).unwrap_or_default()
}

pub(super) fn index_league_membership<'a>(
    leagues: &[&'a League],
) -> (LeagueByTeam<'a>, StandingByTeam<'a>) {
    let mut league_by_team: LeagueByTeam = HashMap::new();
    let mut standing_by_team: StandingByTeam = HashMap::new();
    for league in leagues {
        for team_id in league_member_ids(league) {
            league_by_team.entry(team_id).or_insert(league);
        }
        let mut sorted: Vec<&StandingEntry> = league.standings.iter().collect();
        sorted.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(goal_diff(b).cmp(&goal_diff(a)))
                .then(b.goals_for.cmp(&a.goals_for))
        });
        for (idx, entry) in sorted.iter().enumerate() {
            standing_by_team
                .entry(entry.team_id.as_str())
                .or_insert((idx as u32 + 1, entry));
        }
    }
    (league_by_team, standing_by_team)
}

fn league_member_ids(league: &League) -> Vec<&str> {
    if !league.participant_ids.is_empty() {
        league.participant_ids.iter().map(String::as_str).collect()
    } else {
        league.standings.iter().map(|e| e.team_id.as_str()).collect()
    }
}

fn goal_diff(entry: &StandingEntry) -> i32 {
    entry.goals_for as i32 - entry.goals_against as i32
}
