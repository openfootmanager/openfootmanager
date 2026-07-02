//! Tiered simulation: competitions outside the player's active scope are
//! "dormant" and resolved by scoreline only — no minute-by-minute events,
//! player match stats, morale, stamina, or news. This keeps the wider world
//! moving cheaply while the player's region is simulated in full.

use crate::game::Game;
use crate::national_team::simulate_scoreline;
use domain::league::FixtureStatus;
use rand::Rng;

/// Resolve every fixture due `today` in the dormant competition at
/// `competition_index` with a scoreline-only model: update the fixture result
/// and league standings, and advance any group/knockout progression — but skip
/// the expensive per-player post-match work that the full engine path does.
pub(super) fn simulate_dormant_competition_day(
    game: &mut Game,
    competition_index: usize,
    today: &str,
    rng: &mut impl Rng,
) {
    let due: Vec<(usize, String, String, String)> = game.competitions[competition_index]
        .fixtures
        .iter()
        .enumerate()
        .filter(|(_, fixture)| {
            fixture.date == today && fixture.status == FixtureStatus::Scheduled
        })
        .map(|(index, fixture)| {
            (
                index,
                fixture.id.clone(),
                fixture.home_team_id.clone(),
                fixture.away_team_id.clone(),
            )
        })
        .collect();

    for (fixture_index, fixture_id, home_team_id, away_team_id) in due {
        let home_strength = crate::catchup::club_strength(&game.players, &home_team_id);
        let away_strength = crate::catchup::club_strength(&game.players, &away_team_id);
        let (home_goals, away_goals) = simulate_scoreline(home_strength, away_strength, rng);
        let competition = &mut game.competitions[competition_index];
        // Level knockout ties are settled by a simulated shootout so the
        // bracket advances with a real winner instead of defaulting to home.
        let penalties = (home_goals == away_goals
            && competition.is_knockout_fixture(&fixture_id))
        .then(|| crate::national_team::simulate_shootout(home_strength, away_strength, rng));
        crate::catchup::apply_simulated_result(
            competition,
            fixture_index,
            &home_team_id,
            &away_team_id,
            home_goals,
            away_goals,
            penalties,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::simulate_dormant_competition_day;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        Fixture, FixtureCompetition, FixtureStatus, KnockoutRoundState, League, StandingEntry,
    };
    use domain::manager::Manager;
    use domain::team::Team;
    use rand::{rngs::StdRng, SeedableRng};

    fn make_team(id: &str) -> Team {
        Team::new(
            id.to_string(),
            format!("{id} FC"),
            id.to_uppercase(),
            "England".to_string(),
            "Town".to_string(),
            "Ground".to_string(),
            20_000,
        )
    }

    fn make_dormant_game(today: &str) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2030, 8, 10, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(
            clock,
            manager,
            vec![make_team("home"), make_team("away")],
            vec![],
            vec![],
            vec![],
        );
        game.competitions = vec![League {
            id: "dormant-league".to_string(),
            name: "Dormant League".to_string(),
            season: 2030,
            fixtures: vec![Fixture {
                id: "fix-1".to_string(),
                competition_id: "dormant-league".to_string(),
                matchday: 1,
                date: today.to_string(),
                home_team_id: "home".to_string(),
                away_team_id: "away".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            standings: vec![
                StandingEntry::new("home".to_string()),
                StandingEntry::new("away".to_string()),
            ],
            ..League::default()
        }];
        game
    }

    #[test]
    fn dormant_competition_day_plays_due_fixtures_and_updates_standings() {
        let today = "2030-08-10";
        let mut game = make_dormant_game(today);
        let mut rng = StdRng::seed_from_u64(7);

        simulate_dormant_competition_day(&mut game, 0, today, &mut rng);

        let competition = &game.competitions[0];
        let fixture = &competition.fixtures[0];
        assert_eq!(fixture.status, FixtureStatus::Completed);
        let result = fixture.result.as_ref().expect("fixture should have a result");
        // Cheap path: scoreline only, no per-minute report.
        assert!(result.report.is_none());
        assert!(result.home_scorers.is_empty());

        // Both clubs recorded exactly one played match in the standings.
        let total_played: u32 = competition.standings.iter().map(|entry| entry.played).sum();
        assert_eq!(total_played, 2);
    }

    #[test]
    fn dormant_knockout_draw_is_settled_by_shootout() {
        // Regression: a level knockout tie used to persist with no shootout
        // score, so advance_knockout_competition_round always sent the home
        // side through. Scan seeds until the scoreline model produces a draw
        // and assert the shootout decided it.
        let today = "2030-08-10";
        let mut saw_draw = false;
        for seed in 0..500 {
            let mut game = make_dormant_game(today);
            let competition = &mut game.competitions[0];
            competition.fixtures[0].competition = FixtureCompetition::Cup;
            competition.knockout_rounds = vec![KnockoutRoundState {
                id: "round-1".to_string(),
                name: "Final".to_string(),
                fixture_ids: vec!["fix-1".to_string()],
                bye_team_ids: Vec::new(),
                completed: false,
            }];
            let mut rng = StdRng::seed_from_u64(seed);

            simulate_dormant_competition_day(&mut game, 0, today, &mut rng);

            let result = game.competitions[0].fixtures[0]
                .result
                .as_ref()
                .expect("fixture should have a result");
            if result.home_goals == result.away_goals {
                saw_draw = true;
                let home_pens = result.home_penalties.expect("level knockout needs pens");
                let away_pens = result.away_penalties.expect("level knockout needs pens");
                assert_ne!(home_pens, away_pens, "shootout must have a winner (seed {seed})");
                break;
            }
            assert!(
                result.home_penalties.is_none() && result.away_penalties.is_none(),
                "decisive results must not carry a shootout (seed {seed})"
            );
        }
        assert!(saw_draw, "expected at least one drawn knockout in 500 seeds");
    }

    #[test]
    fn dormant_competition_day_ignores_fixtures_on_other_dates() {
        let mut game = make_dormant_game("2030-08-10");
        let mut rng = StdRng::seed_from_u64(1);

        simulate_dormant_competition_day(&mut game, 0, "2030-08-11", &mut rng);

        assert_eq!(
            game.competitions[0].fixtures[0].status,
            FixtureStatus::Scheduled
        );
    }
}
