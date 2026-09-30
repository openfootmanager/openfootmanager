//! The rule that keeps a pyramid a pyramid.
//!
//! Promotion, relegation, berths and the season rollover all move clubs between
//! a country's league tables. Whatever they do, every club has to end up in
//! exactly one of them, and each division has to keep the size it was authored
//! with. This is the one place that says so; the rollover tests and the season
//! harness both ask it rather than each keeping a copy.

use std::collections::{BTreeMap, BTreeSet};

use domain::league::{CompetitionFormat, CompetitionScope, CompetitionType, League};

use crate::game::Game;

/// Each competition's participant count, keyed by id: the "authored size" that
/// [`ladder_violations`] holds every later state to. Take it once, before
/// anything moves.
pub fn division_sizes(game: &Game) -> BTreeMap<String, usize> {
    game.competitions
        .iter()
        .map(|competition| (competition.id.clone(), competition.participant_ids.len()))
        .collect()
}

/// Everything wrong with how `game`'s domestic league tables hold their clubs,
/// as messages. Empty means the ladder is sound.
///
/// Every club is registered with exactly one of its country's tables, every
/// division still has its authored size, and a split-season country's two halves
/// are still the same division played twice (identical rosters, not disjoint
/// ones).
pub fn ladder_violations(game: &Game, authored_sizes: &BTreeMap<String, usize>) -> Vec<String> {
    let mut violations = Vec::new();

    let mut clubs_by_nation: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for team in &game.teams {
        clubs_by_nation
            .entry(team.football_nation.as_str())
            .or_default()
            .insert(team.id.as_str());
    }

    let mut tables_by_country: BTreeMap<&str, Vec<&League>> = BTreeMap::new();
    for competition in &game.competitions {
        if competition.rules.format != CompetitionFormat::LeagueTable
            || competition.kind != CompetitionType::League
            || competition.scope != CompetitionScope::Domestic
        {
            continue;
        }
        if let Some(country) = competition.country_id.as_deref() {
            tables_by_country
                .entry(country)
                .or_default()
                .push(competition);
        }
    }

    // Walking only the countries that still have a table would never notice a
    // country whose leagues vanished: a rollover test once passed with both
    // Argentine leagues deleted after every season. So the country sets are
    // compared first, and a missing one is a violation in its own right.
    let represented: BTreeSet<&str> = tables_by_country.keys().copied().collect();
    let expected: BTreeSet<&str> = clubs_by_nation.keys().copied().collect();
    for nation in expected.difference(&represented) {
        violations.push(format!("{nation} has clubs but no league table"));
    }
    for country in represented.difference(&expected) {
        violations.push(format!("{country} has a league table but no clubs"));
    }

    for (country, tables) in &tables_by_country {
        let split_season = crate::nations::is_split_season_country(country);
        let mut union: BTreeSet<&str> = BTreeSet::new();

        for table in tables {
            let clubs: BTreeSet<&str> = table.participant_ids.iter().map(String::as_str).collect();
            if clubs.len() != table.participant_ids.len() {
                violations.push(format!(
                    "{} lists a club twice: {:?}",
                    table.id, table.participant_ids
                ));
            }
            match authored_sizes.get(&table.id) {
                Some(&size) if size != table.participant_ids.len() => violations.push(format!(
                    "{} changed size from {size} to {}",
                    table.id,
                    table.participant_ids.len()
                )),
                None => violations.push(format!("{} was not in the authored sizes", table.id)),
                Some(_) => {}
            }
            if !split_season && !union.is_disjoint(&clubs) {
                violations.push(format!(
                    "{} shares clubs with another {country} league",
                    table.id
                ));
            }
            union.extend(clubs);
        }

        if split_season {
            let first: BTreeSet<&str> = tables[0]
                .participant_ids
                .iter()
                .map(String::as_str)
                .collect();
            for table in &tables[1..] {
                let roster: BTreeSet<&str> =
                    table.participant_ids.iter().map(String::as_str).collect();
                if roster != first {
                    violations.push(format!("{country} halves drifted apart at {}", table.id));
                }
            }
        }

        if let Some(expected_clubs) = clubs_by_nation.get(country)
            && &union != expected_clubs
        {
            let missing: Vec<&&str> = expected_clubs.difference(&union).collect();
            let stray: Vec<&&str> = union.difference(expected_clubs).collect();
            violations.push(format!(
                "every {country} club must be in exactly one league table \
                 (in no table: {missing:?}, in a table but not a {country} club: {stray:?})"
            ));
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::world::test_fixtures::{manager_for, nation_team};
    use crate::world::{ensure_multi_competition_foundations_with, start_date_for_year};

    /// Two two-tier nations and an Argentine split-season one, at a division
    /// size of four — every shape the rule has to tell apart.
    fn small_world() -> Game {
        let mut teams = Vec::new();
        for (nation, count) in [("ENG", 8), ("ES", 8), ("AR", 4)] {
            for index in 0..count {
                teams.push(nation_team(
                    &format!("{}-{index:02}", nation.to_lowercase()),
                    nation,
                    1000 - index as u32,
                ));
            }
        }
        let clock = GameClock::new(start_date_for_year(2033).expect("a valid start year"));
        let mut game = Game::new(clock, manager_for("eng-00"), teams, vec![], vec![], vec![]);
        ensure_multi_competition_foundations_with(&mut game, 4);
        game
    }

    fn table<'a>(game: &'a mut Game, id: &str) -> &'a mut League {
        game.competitions
            .iter_mut()
            .find(|competition| competition.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
    }

    #[test]
    fn a_freshly_built_pyramid_has_no_violations() {
        let game = small_world();
        let sizes = division_sizes(&game);
        assert_eq!(ladder_violations(&game, &sizes), Vec::<String>::new());
    }

    #[test]
    fn a_club_registered_in_two_tables_of_one_country_is_a_violation() {
        let mut game = small_world();
        let sizes = division_sizes(&game);
        let intruder = table(&mut game, "eng-d2").participant_ids[0].clone();
        table(&mut game, "eng-d1").participant_ids[0] = intruder;

        let violations = ladder_violations(&game, &sizes);

        assert!(
            violations
                .iter()
                .any(|message| message.contains("shares clubs")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_club_missing_from_every_table_is_a_violation() {
        let mut game = small_world();
        let sizes = division_sizes(&game);
        table(&mut game, "eng-d2").participant_ids.pop();

        let violations = ladder_violations(&game, &sizes);

        assert!(
            violations
                .iter()
                .any(|message| message.contains("exactly one league table")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_division_that_changes_size_is_a_violation() {
        let mut game = small_world();
        let sizes = division_sizes(&game);
        table(&mut game, "eng-d1").participant_ids.pop();

        let violations = ladder_violations(&game, &sizes);

        assert!(
            violations
                .iter()
                .any(|message| message.contains("changed size")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_country_whose_leagues_all_vanish_is_a_violation() {
        let mut game = small_world();
        let sizes = division_sizes(&game);
        game.competitions
            .retain(|competition| competition.country_id.as_deref() != Some("AR"));

        let violations = ladder_violations(&game, &sizes);

        assert!(
            violations
                .iter()
                .any(|message| message.contains("AR has clubs but no league table")),
            "{violations:?}"
        );
    }

    #[test]
    fn split_season_halves_that_drift_apart_are_a_violation() {
        let mut game = small_world();
        let sizes = division_sizes(&game);
        // Argentina plays one division twice, as Apertura and Clausura; its cup
        // shares the country id but is not a league table.
        for half in ["ar-d1-apertura", "ar-d1-clausura"] {
            assert!(
                game.competitions
                    .iter()
                    .any(|competition| competition.id == half),
                "{half} should exist"
            );
        }
        table(&mut game, "ar-d1-clausura").participant_ids[0] = "ar-ghost".to_string();

        let violations = ladder_violations(&game, &sizes);

        assert!(
            violations
                .iter()
                .any(|message| message.contains("halves drifted apart")),
            "{violations:?}"
        );
    }
}
