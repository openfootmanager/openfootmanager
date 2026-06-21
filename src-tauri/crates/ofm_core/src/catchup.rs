//! Mid-season catch-up simulation.
//!
//! When a game starts and a competition's season began before the game's
//! anchor date (July 1), this module fills in the missing matchdays with
//! quick scoreline-only results so the player joins a living, in-progress
//! season rather than a blank table.

use chrono::{DateTime, NaiveDate, Utc};
use domain::league::{FixtureStatus, League, MatchResult};
use domain::player::Player;

const CATCHUP_XI: usize = 11;

/// Average OVR of a club's best XI, used as scoreline strength. Falls back to a
/// neutral rating when the club has no players on the books.
pub(crate) fn club_strength(players: &[Player], club_id: &str) -> f64 {
    let mut ovrs: Vec<u8> = players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(club_id))
        .map(|p| p.ovr)
        .collect();
    if ovrs.is_empty() {
        return 50.0;
    }
    ovrs.sort_unstable_by(|a, b| b.cmp(a));
    let count = ovrs.len().min(CATCHUP_XI);
    let total: u32 = ovrs.iter().take(count).map(|&o| u32::from(o)).sum();
    total as f64 / count as f64
}

/// Apply a pre-computed scoreline to a fixture, updating standings and
/// advancing group/knockout state. Shared by the catch-up and dormant paths.
pub(crate) fn apply_simulated_result(
    competition: &mut League,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    home_goals: u8,
    away_goals: u8,
) {
    let fixture = &mut competition.fixtures[fixture_index];
    fixture.status = FixtureStatus::Completed;
    let counts = fixture.counts_for_league_standings();
    fixture.result = Some(MatchResult {
        home_goals,
        away_goals,
        home_scorers: Vec::new(),
        away_scorers: Vec::new(),
        report: None,
    });
    if counts {
        if let Some(entry) = competition
            .standings
            .iter_mut()
            .find(|e| e.team_id == home_team_id)
        {
            entry.record_result(home_goals, away_goals);
        }
        if let Some(entry) = competition
            .standings
            .iter_mut()
            .find(|e| e.team_id == away_team_id)
        {
            entry.record_result(away_goals, home_goals);
        }
    }
    crate::group_stage::process_completed_fixture(competition, fixture_index);
    crate::schedule::advance_knockout_competition_round(competition);
}

/// Simulate all fixtures in `competition` whose date is before `cutoff`,
/// filling in random scorelines and updating standings. Called once at
/// new-game creation for leagues that started before the game's anchor date.
pub fn simulate_past_fixtures(competition: &mut League, players: &[Player], cutoff: DateTime<Utc>) {
    let cutoff_date = cutoff.date_naive();
    let mut rng = rand::rng();

    // Precompute strength once per participant to avoid O(fixtures × players).
    let strengths: std::collections::HashMap<String, f64> = competition
        .participant_ids
        .iter()
        .map(|id| (id.clone(), club_strength(players, id)))
        .collect();

    let due: Vec<(usize, String, String)> = competition
        .fixtures
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.status == FixtureStatus::Scheduled
                && NaiveDate::parse_from_str(&f.date, "%Y-%m-%d")
                    .map(|d| d < cutoff_date)
                    .unwrap_or(false)
        })
        .map(|(i, f)| (i, f.home_team_id.clone(), f.away_team_id.clone()))
        .collect();

    for (idx, home_id, away_id) in due {
        let home_strength = strengths.get(&home_id).copied().unwrap_or(50.0);
        let away_strength = strengths.get(&away_id).copied().unwrap_or(50.0);
        let (home_goals, away_goals) =
            crate::national_team::simulate_scoreline(home_strength, away_strength, &mut rng);
        apply_simulated_result(competition, idx, &home_id, &away_id, home_goals, away_goals);
    }
}
