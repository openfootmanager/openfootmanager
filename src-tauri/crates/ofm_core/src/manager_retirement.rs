//! Unemployed managers leave the active pool, so `game.managers` stops growing for ever.
//!
//! The pool is topped up every season (retiring players become managers, sackings add more) and
//! nothing took anyone out. A manager who retires here moves whole into `game.retired_managers`,
//! which the save writes to its own table; the directory and the market never see them again,
//! while a club's history still can.

use chrono::NaiveDate;
use domain::manager::Manager;

use crate::game::Game;

/// The age at which an unemployed manager stops looking for work. Generated managers are 35–64.
const RETIREMENT_AGE: u32 = 65;

/// The most unemployed managers per club the market keeps; the season-end floor is two per club.
const CEILING_PER_CLUB: usize = 4;

fn age_on(manager: &Manager, today: NaiveDate) -> Option<u32> {
    let born = NaiveDate::parse_from_str(&manager.date_of_birth, "%Y-%m-%d").ok()?;
    today.years_since(born)
}

/// Moves old and surplus unemployed managers into the archive. Returns how many moved.
///
/// The user's manager and anyone employed stay. Past the ceiling the least reputable go first,
/// the id breaking a tie so the same world retires the same people.
pub fn retire_unemployed_managers(game: &mut Game) -> usize {
    let user_id = if game.manager_id.is_empty() {
        &game.manager.id
    } else {
        &game.manager_id
    };
    let today = game.clock.current_date.date_naive();
    let unemployed = |manager: &&Manager| manager.id != *user_id && manager.team_id.is_none();

    let aged_out: Vec<String> = game
        .managers
        .iter()
        .filter(unemployed)
        .filter(|m| age_on(m, today).is_some_and(|age| age >= RETIREMENT_AGE))
        .map(|m| m.id.clone())
        .collect();

    let mut remaining: Vec<&Manager> = game
        .managers
        .iter()
        .filter(unemployed)
        .filter(|m| !aged_out.contains(&m.id))
        .collect();
    // Most reputable first, the earlier id winning a tie; whatever is past the ceiling retires.
    remaining.sort_by(|a, b| {
        b.reputation
            .cmp(&a.reputation)
            .then_with(|| a.id.cmp(&b.id))
    });
    let ceiling = game.teams.len() * CEILING_PER_CLUB;
    let mut surplus: Vec<String> = remaining
        .iter()
        .skip(ceiling)
        .map(|m| m.id.clone())
        .collect();
    surplus.reverse();

    let retiring: Vec<String> = aged_out.into_iter().chain(surplus).collect();
    archive(game, &retiring);
    retiring.len()
}

/// Moves `ids` out of the active pool into the archive, in the order given.
fn archive(game: &mut Game, ids: &[String]) {
    for id in ids {
        if let Some(position) = game.managers.iter().position(|m| m.id == *id) {
            game.retired_managers.push(game.managers.remove(position));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};

    fn game_in_2031() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 6, 30, 12, 0, 0).unwrap());
        let user = Manager::new(
            "user".into(),
            "Una".into(),
            "User".into(),
            "1950-01-01".into(),
            "England".into(),
        );
        Game::new(clock, user, vec![], vec![], vec![], vec![])
    }

    fn club(id: &str) -> domain::team::Team {
        domain::team::Team::new(
            id.into(),
            id.into(),
            id.into(),
            "ENG".into(),
            "City".into(),
            "Ground".into(),
            10_000,
        )
    }

    fn unemployed(id: &str, born: &str, reputation: u32) -> Manager {
        let mut manager = Manager::new(
            id.into(),
            "Ex".into(),
            id.into(),
            born.into(),
            "England".into(),
        );
        manager.reputation = reputation;
        manager
    }

    fn ids(managers: &[Manager]) -> Vec<&str> {
        managers.iter().map(|m| m.id.as_str()).collect()
    }

    /// Given an unemployed manager who turned 65 this year, a younger one, an employed 70-year-old and a 75-year-old user
    /// When the season-end retirement runs
    /// Then only the unemployed 65-year-old is archived, whole, and the others stay.
    #[test]
    fn an_unemployed_manager_of_retirement_age_is_archived_whole() {
        let mut game = game_in_2031();
        game.teams = vec![club("t1")];
        let mut old = unemployed("old", "1966-06-30", 400);
        old.career_stats.trophies = 3;
        let mut employed = unemployed("employed", "1961-01-01", 400);
        employed.team_id = Some("club".into());
        game.managers
            .extend([old, unemployed("young", "1966-07-01", 400), employed]);

        let moved = retire_unemployed_managers(&mut game);

        assert_eq!(moved, 1);
        assert_eq!(ids(&game.retired_managers), ["old"]);
        assert_eq!(game.retired_managers[0].career_stats.trophies, 3);
        assert_eq!(ids(&game.managers), ["user", "young", "employed"]);
    }

    /// Given more unemployed managers than the ceiling allows
    /// When the retirement runs
    /// Then the least reputable go first, a tie going to the later id, and the ceiling's worth stay.
    #[test]
    fn a_surplus_of_unemployed_managers_retires_the_least_reputable_first() {
        let mut game = game_in_2031();
        game.teams = vec![club("t1")];
        // Ceiling is 4 for one club; six candidates, two tied at the bottom.
        for (id, reputation) in [
            ("a", 900),
            ("b", 800),
            ("c", 700),
            ("d", 600),
            ("e", 100),
            ("f", 100),
        ] {
            game.managers.push(unemployed(id, "1990-01-01", reputation));
        }

        let moved = retire_unemployed_managers(&mut game);

        assert_eq!(moved, 2);
        assert_eq!(ids(&game.retired_managers), ["f", "e"]);
        assert_eq!(ids(&game.managers), ["user", "a", "b", "c", "d"]);
    }

    /// Given a manager already archived
    /// When the retirement runs again
    /// Then nobody moves twice and the archive is unchanged.
    #[test]
    fn retiring_twice_moves_nobody_twice() {
        let mut game = game_in_2031();
        game.teams = vec![club("t1")];
        game.managers.push(unemployed("old", "1950-01-01", 400));

        assert_eq!(retire_unemployed_managers(&mut game), 1);
        assert_eq!(retire_unemployed_managers(&mut game), 0);
        assert_eq!(ids(&game.retired_managers), ["old"]);
    }
}
