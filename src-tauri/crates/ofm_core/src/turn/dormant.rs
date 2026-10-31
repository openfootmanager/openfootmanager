//! Tiered simulation: competitions outside the player's active scope are
//! "dormant" and resolved by scoreline only — no minute-by-minute events,
//! player match stats, morale, stamina, or news. This keeps the wider world
//! moving cheaply while the player's region is simulated in full.

use std::collections::HashMap;

use domain::league::{FixtureStatus, League};
use domain::player::Player;

/// Resolve every fixture due `today` in the dormant `competition` with a scoreline-only model:
/// update the fixture result and league standings, and advance any group/knockout progression —
/// but skip the expensive per-player post-match work that the full engine path does.
/// `players_by_club` is every club's players, as `match_day_eligibility::players_by_club` finds
/// them.
pub(super) fn simulate_dormant_competition_day(
    competition: &mut League,
    seed: u64,
    players_by_club: &HashMap<&str, Vec<&Player>>,
    today: &str,
) {
    let due: Vec<usize> = competition
        .fixtures
        .iter()
        .enumerate()
        .filter(|(_, fixture)| fixture.date == today && fixture.status == FixtureStatus::Scheduled)
        .map(|(index, _)| index)
        .collect();

    for fixture_index in due {
        // A stream per fixture, keyed by the competition and the two clubs: fixtures are not
        // played in any order a save is bound to keep.
        let fixture = &competition.fixtures[fixture_index];
        let tag = format!(
            "dormant/{}/{}/{}",
            competition.id, fixture.home_team_id, fixture.away_team_id
        );
        let mut rng = crate::seed::rng_for_seed(seed, &tag, today);
        crate::catchup::resolve_fixture_with_strengths(
            competition,
            fixture_index,
            |team_id| {
                crate::catchup::club_strength_among(
                    players_by_club.get(team_id).map_or(&[][..], Vec::as_slice),
                )
            },
            &mut rng,
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
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;

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
                postponement: None,
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

    fn play_day(game: &mut Game, today: &str) {
        let clubs = crate::match_day_eligibility::players_by_club(&game.players);
        simulate_dormant_competition_day(&mut game.competitions[0], game.seed, &clubs, today);
    }

    fn player(id: &str, team_id: &str, ovr: u8) -> Player {
        let level = 60;
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Midfielder,
            PlayerAttributes {
                pace: level,
                stamina: level,
                strength: level,
                agility: level,
                passing: level,
                shooting: level,
                tackling: level,
                dribbling: level,
                defending: level,
                positioning: level,
                vision: level,
                decisions: level,
                composure: level,
                aggression: level,
                teamwork: level,
                leadership: level,
                handling: level,
                reflexes: level,
                aerial: level,
            },
        );
        player.team_id = Some(team_id.to_string());
        player.ovr = ovr;
        player
    }

    /// Two fixtures on one dormant day: a strong home side against a weak one that has a player
    /// on loan from it, and a club with nobody on its books against a mid-table side.
    fn two_fixture_dormant_game(today: &str) -> Game {
        let mut game = make_dormant_game(today);
        game.teams.push(make_team("empty"));
        game.teams.push(make_team("middle"));
        for i in 0..14 {
            game.players.push(player(&format!("home{i}"), "home", 80));
            game.players
                .push(player(&format!("middle{i}"), "middle", 65));
        }
        for i in 0..6 {
            game.players.push(player(&format!("away{i}"), "away", 45));
        }
        let mut loanee = player("loanee", "away", 90);
        loanee.active_loan = Some(domain::player::ActiveLoan {
            parent_team_id: "home".to_string(),
            loan_team_id: "away".to_string(),
            start_date: "2030-07-01".to_string(),
            end_date: "2031-06-30".to_string(),
            wage_contribution_pct: 100,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        game.players.push(loanee);
        let competition = &mut game.competitions[0];
        let mut second = competition.fixtures[0].clone();
        second.id = "fix-2".to_string();
        second.home_team_id = "empty".to_string();
        second.away_team_id = "middle".to_string();
        competition.fixtures.push(second);
        competition
            .standings
            .push(StandingEntry::new("empty".to_string()));
        competition
            .standings
            .push(StandingEntry::new("middle".to_string()));
        game
    }

    /// Given a dormant day whose clubs have players (one of them on loan) and one club with nobody
    /// on its books, when the day is played from the players grouped once, then every fixture and
    /// table entry is what settling each fixture by scanning the players gives, seed after seed.
    #[test]
    fn a_dormant_day_from_grouped_players_settles_as_the_player_scan_does() {
        let today = "2030-08-10";
        for seed in 0..40 {
            let mut grouped = two_fixture_dormant_game(today);
            grouped.seed = seed;
            let mut scanned = grouped.clone();

            play_day(&mut grouped, today);
            for fixture_index in 0..2 {
                let fixture = &scanned.competitions[0].fixtures[fixture_index];
                let tag = format!(
                    "dormant/{}/{}/{}",
                    scanned.competitions[0].id, fixture.home_team_id, fixture.away_team_id
                );
                let mut rng = scanned.rng_for(&tag, today);
                crate::catchup::resolve_fixture_by_scoreline(
                    &scanned.players,
                    &mut scanned.competitions[0],
                    fixture_index,
                    &mut rng,
                );
            }

            assert_eq!(
                format!("{:?}", grouped.competitions[0].fixtures),
                format!("{:?}", scanned.competitions[0].fixtures),
                "seed {seed}"
            );
            assert_eq!(
                format!("{:?}", grouped.competitions[0].standings),
                format!("{:?}", scanned.competitions[0].standings),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn dormant_competition_day_plays_due_fixtures_and_updates_standings() {
        let today = "2030-08-10";
        let mut game = make_dormant_game(today);

        play_day(&mut game, today);

        let competition = &game.competitions[0];
        let fixture = &competition.fixtures[0];
        assert_eq!(fixture.status, FixtureStatus::Completed);
        let result = fixture
            .result
            .as_ref()
            .expect("fixture should have a result");
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
            game.seed = seed;

            play_day(&mut game, today);

            let result = game.competitions[0].fixtures[0]
                .result
                .as_ref()
                .expect("fixture should have a result");
            if result.home_goals == result.away_goals {
                saw_draw = true;
                let home_pens = result.home_penalties.expect("level knockout needs pens");
                let away_pens = result.away_penalties.expect("level knockout needs pens");
                assert_ne!(
                    home_pens, away_pens,
                    "shootout must have a winner (seed {seed})"
                );
                break;
            }
            assert!(
                result.home_penalties.is_none() && result.away_penalties.is_none(),
                "decisive results must not carry a shootout (seed {seed})"
            );
        }
        assert!(
            saw_draw,
            "expected at least one drawn knockout in 500 seeds"
        );
    }

    #[test]
    fn dormant_competition_day_ignores_fixtures_on_other_dates() {
        let mut game = make_dormant_game("2030-08-10");

        play_day(&mut game, "2030-08-11");

        assert_eq!(
            game.competitions[0].fixtures[0].status,
            FixtureStatus::Scheduled
        );
    }
}
