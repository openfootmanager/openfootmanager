use crate::game::Game;
use crate::player_rating::{effective_rating_for_assignment, formation_slots, natural_ovr};
use domain::league::{Fixture, FixtureStatus, StandingEntry};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundSummary {
    pub matchday: u32,
    pub is_complete: bool,
    pub pending_fixture_count: u32,
    pub completed_results: Vec<RoundResultSummary>,
    pub standings_delta: Vec<StandingDelta>,
    pub notable_upset: Option<NotableUpset>,
    pub top_scorer_delta: Vec<TopScorerDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundResultSummary {
    pub fixture_id: String,
    pub home_team_id: String,
    pub home_team_name: String,
    pub away_team_id: String,
    pub away_team_name: String,
    pub home_goals: u8,
    pub away_goals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StandingDelta {
    pub team_id: String,
    pub team_name: String,
    pub previous_position: u32,
    pub current_position: u32,
    pub points: u32,
    pub points_delta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotableUpset {
    pub fixture_id: String,
    pub favorite_team_id: String,
    pub favorite_team_name: String,
    pub favorite_strength: f64,
    pub underdog_team_id: String,
    pub underdog_team_name: String,
    pub underdog_strength: f64,
    pub strength_gap: f64,
    pub home_goals: u8,
    pub away_goals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopScorerDelta {
    pub player_id: String,
    pub player_name: String,
    pub team_id: String,
    pub previous_rank: u32,
    pub current_rank: u32,
    pub previous_goals: u32,
    pub current_goals: u32,
}

#[derive(Clone)]
struct ScorerSnapshot {
    player_id: String,
    player_name: String,
    team_id: String,
    previous_goals: u32,
    current_goals: u32,
}

pub fn build_round_summary(
    game: &Game,
    matchday: u32,
    previous_standings: &[StandingEntry],
) -> Option<RoundSummary> {
    let league = game.league.as_ref()?;
    let round_fixtures: Vec<&Fixture> = league
        .fixtures
        .iter()
        .filter(|fixture| fixture.matchday == matchday && fixture.counts_for_league_standings())
        .collect();

    if round_fixtures.is_empty() {
        return None;
    }

    let completed_fixtures: Vec<&Fixture> = round_fixtures
        .iter()
        .copied()
        .filter(|fixture| fixture.status == FixtureStatus::Completed && fixture.result.is_some())
        .collect();

    if completed_fixtures.is_empty() {
        return None;
    }

    let completed_results = completed_fixtures
        .iter()
        .filter_map(|fixture| {
            let result = fixture.result.as_ref()?;
            Some(RoundResultSummary {
                fixture_id: fixture.id.clone(),
                home_team_id: fixture.home_team_id.clone(),
                home_team_name: team_name(game, &fixture.home_team_id),
                away_team_id: fixture.away_team_id.clone(),
                away_team_name: team_name(game, &fixture.away_team_id),
                home_goals: result.home_goals,
                away_goals: result.away_goals,
            })
        })
        .collect();

    let pending_fixture_count = (round_fixtures.len() - completed_fixtures.len()) as u32;

    Some(RoundSummary {
        matchday,
        is_complete: pending_fixture_count == 0,
        pending_fixture_count,
        completed_results,
        standings_delta: build_standings_delta(
            game,
            previous_standings,
            &league.sorted_standings(),
        ),
        notable_upset: build_notable_upset(game, &completed_fixtures),
        top_scorer_delta: build_top_scorer_delta(game, &completed_fixtures),
    })
}

fn build_standings_delta(
    game: &Game,
    previous_standings: &[StandingEntry],
    current_standings: &[StandingEntry],
) -> Vec<StandingDelta> {
    let previous_sorted = sort_standings(previous_standings.to_vec());
    let current_sorted = sort_standings(current_standings.to_vec());

    let previous_positions: HashMap<&str, u32> = previous_sorted
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.team_id.as_str(), index as u32 + 1))
        .collect();
    let previous_points: HashMap<&str, u32> = previous_sorted
        .iter()
        .map(|entry| (entry.team_id.as_str(), entry.points))
        .collect();

    current_sorted
        .iter()
        .enumerate()
        .map(|(index, entry)| StandingDelta {
            team_id: entry.team_id.clone(),
            team_name: team_name(game, &entry.team_id),
            previous_position: previous_positions
                .get(entry.team_id.as_str())
                .copied()
                .unwrap_or(index as u32 + 1),
            current_position: index as u32 + 1,
            points: entry.points,
            points_delta: entry.points as i32
                - previous_points
                    .get(entry.team_id.as_str())
                    .copied()
                    .unwrap_or(entry.points) as i32,
        })
        .collect()
}

fn build_notable_upset(game: &Game, fixtures: &[&Fixture]) -> Option<NotableUpset> {
    fixtures
        .iter()
        .filter_map(|fixture| {
            let result = fixture.result.as_ref()?;
            // A level scoreline is only a non-result when nothing breaks the
            // tie. A knockout decided on penalties is level on goals but still
            // has a winner, so it can be a giant-killing upset.
            let decided_by_penalties =
                result.home_penalties.is_some() && result.away_penalties.is_some();
            if result.home_goals == result.away_goals && !decided_by_penalties {
                return None;
            }

            let home_strength = team_strength(game, &fixture.home_team_id);
            let away_strength = team_strength(game, &fixture.away_team_id);

            let (
                favorite_team_id,
                favorite_team_name,
                favorite_strength,
                underdog_team_id,
                underdog_team_name,
                underdog_strength,
            ) = if home_strength > away_strength {
                (
                    fixture.home_team_id.clone(),
                    team_name(game, &fixture.home_team_id),
                    home_strength,
                    fixture.away_team_id.clone(),
                    team_name(game, &fixture.away_team_id),
                    away_strength,
                )
            } else if away_strength > home_strength {
                (
                    fixture.away_team_id.clone(),
                    team_name(game, &fixture.away_team_id),
                    away_strength,
                    fixture.home_team_id.clone(),
                    team_name(game, &fixture.home_team_id),
                    home_strength,
                )
            } else {
                return None;
            };

            let winner_id = if result.advancing_is_home() {
                fixture.home_team_id.as_str()
            } else {
                fixture.away_team_id.as_str()
            };

            if winner_id != underdog_team_id {
                return None;
            }

            Some(NotableUpset {
                fixture_id: fixture.id.clone(),
                favorite_team_id,
                favorite_team_name,
                favorite_strength,
                underdog_team_id,
                underdog_team_name,
                underdog_strength,
                strength_gap: (favorite_strength - underdog_strength).abs(),
                home_goals: result.home_goals,
                away_goals: result.away_goals,
            })
        })
        .max_by(|left, right| {
            left.strength_gap
                .partial_cmp(&right.strength_gap)
                .unwrap_or(Ordering::Equal)
        })
}

fn build_top_scorer_delta(game: &Game, fixtures: &[&Fixture]) -> Vec<TopScorerDelta> {
    let round_goals = round_goal_counts(fixtures);
    let scorers: Vec<ScorerSnapshot> = game
        .players
        .iter()
        .filter_map(|player| {
            let current_goals = player.stats.goals;
            let round_goal_count = round_goals.get(&player.id).copied().unwrap_or(0);
            let previous_goals = current_goals.saturating_sub(round_goal_count);

            if current_goals == 0 && previous_goals == 0 {
                return None;
            }

            Some(ScorerSnapshot {
                player_id: player.id.clone(),
                player_name: player.match_name.clone(),
                team_id: player.team_id.clone().unwrap_or_default(),
                previous_goals,
                current_goals,
            })
        })
        .collect();

    let previous_ranks = scorer_ranks(&scorers, |scorer| scorer.previous_goals);
    let current_ranks = scorer_ranks(&scorers, |scorer| scorer.current_goals);

    let mut deltas: Vec<TopScorerDelta> = scorers
        .into_iter()
        .map(|scorer| TopScorerDelta {
            player_id: scorer.player_id.clone(),
            player_name: scorer.player_name,
            team_id: scorer.team_id,
            previous_rank: previous_ranks.get(&scorer.player_id).copied().unwrap_or(0),
            current_rank: current_ranks.get(&scorer.player_id).copied().unwrap_or(0),
            previous_goals: scorer.previous_goals,
            current_goals: scorer.current_goals,
        })
        .collect();

    deltas.sort_by(|left, right| {
        left.current_rank
            .cmp(&right.current_rank)
            .then(left.player_name.cmp(&right.player_name))
    });

    deltas
}

fn scorer_ranks(
    scorers: &[ScorerSnapshot],
    goals_for_rank: fn(&ScorerSnapshot) -> u32,
) -> HashMap<String, u32> {
    let mut sorted = scorers.to_vec();
    sorted.sort_by(|left, right| {
        goals_for_rank(right)
            .cmp(&goals_for_rank(left))
            .then(left.player_name.cmp(&right.player_name))
            .then(left.player_id.cmp(&right.player_id))
    });

    sorted
        .iter()
        .enumerate()
        .map(|(index, scorer)| (scorer.player_id.clone(), index as u32 + 1))
        .collect()
}

fn round_goal_counts(fixtures: &[&Fixture]) -> HashMap<String, u32> {
    let mut counts = HashMap::new();

    for fixture in fixtures {
        let Some(result) = fixture.result.as_ref() else {
            continue;
        };

        for scorer in result.home_scorers.iter().chain(result.away_scorers.iter()) {
            *counts.entry(scorer.player_id.clone()).or_insert(0) += 1;
        }
    }

    counts
}

fn sort_standings(mut standings: Vec<StandingEntry>) -> Vec<StandingEntry> {
    standings.sort_by(|left, right| {
        right
            .points
            .cmp(&left.points)
            .then(right.goal_difference().cmp(&left.goal_difference()))
            .then(right.goals_for.cmp(&left.goals_for))
    });
    standings
}

fn team_strength(game: &Game, team_id: &str) -> f64 {
    let team = game.teams.iter().find(|team| team.id == team_id);
    match team {
        Some(team) if !team.starting_xi_ids.is_empty() => {
            let slots = formation_slots(&team.formation);
            let rated_players: Vec<f64> = team
                .starting_xi_ids
                .iter()
                .enumerate()
                .filter_map(|(index, player_id)| {
                    let player = game.players.iter().find(|player| &player.id == player_id)?;
                    let slot = slots
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| player.natural_position.clone());
                    Some(effective_rating_for_assignment(player, &slot))
                })
                .collect();

            if rated_players.is_empty() {
                0.0
            } else {
                rated_players.iter().sum::<f64>() / rated_players.len() as f64
            }
        }
        _ => {
            let selected_players = game
                .players
                .iter()
                .filter(|player| player.team_id.as_deref() == Some(team_id))
                .collect::<Vec<_>>();

            if selected_players.is_empty() {
                0.0
            } else {
                selected_players
                    .iter()
                    .map(|player| natural_ovr(player))
                    .sum::<f64>()
                    / selected_players.len() as f64
            }
        }
    }
}

fn team_name(game: &Game, team_id: &str) -> String {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.to_string())
}
