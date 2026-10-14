//! The generated calendar: the one question a rollover or a world build has to answer, and the
//! reload a save goes through between seasons.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use domain::league::{FixtureCompetition, FixtureStatus};
use ofm_core::calendar_booking::too_close;
use ofm_core::game::Game;

/// Pairs of a club's matches still to be played that sit closer than the minimum rest.
pub fn unplayed_clashes(game: &Game) -> Vec<String> {
    let today = game.clock.current_date.date_naive();
    let mut by_club: BTreeMap<&str, Vec<(NaiveDate, &str, &str)>> = BTreeMap::new();
    for competition in &game.competitions {
        for fixture in &competition.fixtures {
            if fixture.competition == FixtureCompetition::InternationalNation
                || fixture.status != FixtureStatus::Scheduled
            {
                continue;
            }
            let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") else {
                continue;
            };
            if date < today {
                continue;
            }
            for club in [&fixture.home_team_id, &fixture.away_team_id] {
                by_club
                    .entry(club)
                    .or_default()
                    .push((date, &competition.id, &fixture.id));
            }
        }
    }
    let mut clashes = Vec::new();
    for (club, mut rows) in by_club {
        rows.sort();
        for pair in rows.windows(2) {
            if too_close(pair[0].0, pair[1].0) {
                clashes.push(format!(
                    "{club}: {} {} and {} {}",
                    pair[0].1, pair[0].0, pair[1].1, pair[1].0
                ));
            }
        }
    }
    clashes
}

/// The game as a save and a load return it.
pub fn reloaded(game: &Game) -> Game {
    let saved = serde_json::to_vec(game).expect("a game serialises");
    serde_json::from_slice(&saved).expect("a saved game loads")
}
