//! Mid-season catch-up simulation.
//!
//! When a game starts and a competition's season began before the game's
//! anchor date (July 1), this module fills in the missing matchdays with
//! quick scoreline-only results so the player joins a living, in-progress
//! season rather than a blank table.

use crate::game::Game;
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
/// `penalties` carries a simulated shootout score for level knockout ties so
/// the round advances with a real winner instead of defaulting to home.
pub(crate) fn apply_simulated_result(
    competition: &mut League,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    home_goals: u8,
    away_goals: u8,
    penalties: Option<(u8, u8)>,
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
        home_penalties: penalties.map(|(home, _)| home),
        away_penalties: penalties.map(|(_, away)| away),
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

    let due: Vec<(usize, String, String, String)> = competition
        .fixtures
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.status == FixtureStatus::Scheduled
                && NaiveDate::parse_from_str(&f.date, "%Y-%m-%d")
                    .map(|d| d < cutoff_date)
                    .unwrap_or(false)
        })
        .map(|(i, f)| {
            (
                i,
                f.id.clone(),
                f.home_team_id.clone(),
                f.away_team_id.clone(),
            )
        })
        .collect();

    for (idx, fixture_id, home_id, away_id) in due {
        let home_strength = strengths.get(&home_id).copied().unwrap_or(50.0);
        let away_strength = strengths.get(&away_id).copied().unwrap_or(50.0);
        let (home_goals, away_goals) =
            crate::national_team::simulate_scoreline(home_strength, away_strength, &mut rng);
        let penalties = (home_goals == away_goals && competition.is_knockout_fixture(&fixture_id))
            .then(|| {
                crate::national_team::simulate_shootout(home_strength, away_strength, &mut rng)
            });
        apply_simulated_result(
            competition,
            idx,
            &home_id,
            &away_id,
            home_goals,
            away_goals,
            penalties,
        );
    }
}

/// Resolve fixtures left `Scheduled` with a date that has already passed.
///
/// Called on every save load. Distinct from [`simulate_past_fixtures`], which runs once at career
/// creation to fill in a season that began before the game's anchor date — this is a repair for
/// fixtures that *should* have been played during the career and were not.
///
/// It exists because #608 skipped whole days: on any day the player watched their own match, every
/// other competition's fixtures due that day were never simulated. Nothing picks them up afterwards,
/// because every "due today" check matches on `date == today` — so the table stays permanently
/// short, a knockout bracket stalls with no winner, and `is_season_complete` does not even notice,
/// since it only asks about fixtures still to come.
///
/// Deliberately unconditional rather than gated on a save-format version. This is a data-integrity
/// invariant — no fixture stays `Scheduled` in the past — not a one-time migration, so it also heals
/// any future regression of the same class. It is idempotent: once repaired there is nothing to
/// find, and the count it returns is zero, so a load does not keep rewriting the save.
///
/// `InProgress` is left alone: a save taken mid-match may have a resumable session, and resolving it
/// by scoreline would discard it.
///
/// Returns how many fixtures were repaired, for the caller to log and to decide whether to resave.
pub fn repair_stranded_fixtures(game: &mut Game) -> usize {
    let today = game.clock.current_date.date_naive();
    let stranded = |competition: &League| {
        competition
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.status == FixtureStatus::Scheduled
                    && NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d")
                        .map(|date| date < today)
                        .unwrap_or(false)
            })
            .count()
    };

    let total: usize = game.competitions.iter().map(stranded).sum();
    if total == 0 {
        return 0;
    }

    // Taken out so the scoreline model can read `game.players` while the competitions are mutated.
    let mut competitions = std::mem::take(&mut game.competitions);
    for competition in &mut competitions {
        simulate_past_fixtures(competition, &game.players, game.clock.current_date);
    }
    game.competitions = competitions;

    total
}

#[cfg(test)]
mod tests {
    use super::apply_simulated_result;
    use domain::league::{
        CompetitionFormat, CompetitionRules, Fixture, FixtureCompetition, FixtureStatus,
        KnockoutRoundState, League,
    };

    fn make_knockout_cup() -> League {
        League {
            id: "cup".to_string(),
            name: "Cup".to_string(),
            season: 2030,
            rules: CompetitionRules {
                format: CompetitionFormat::Knockout,
                ..CompetitionRules::default()
            },
            fixtures: vec![Fixture {
                id: "fix-1".to_string(),
                competition_id: "cup".to_string(),
                matchday: 1,
                date: "2030-08-10".to_string(),
                home_team_id: "home".to_string(),
                away_team_id: "away".to_string(),
                competition: FixtureCompetition::Cup,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            knockout_rounds: vec![KnockoutRoundState {
                id: "round-1".to_string(),
                name: "Final".to_string(),
                fixture_ids: vec!["fix-1".to_string()],
                bye_team_ids: Vec::new(),
                completed: false,
            }],
            ..League::default()
        }
    }

    // Regression: level knockout results used to persist with no shootout
    // score, so the away side could never advance from a simulated draw.
    #[test]
    fn simulated_shootout_score_persists_and_decides_the_tie() {
        let mut cup = make_knockout_cup();
        apply_simulated_result(&mut cup, 0, "home", "away", 1, 1, Some((3, 4)));

        let result = cup.fixtures[0].result.as_ref().unwrap();
        assert_eq!(result.home_penalties, Some(3));
        assert_eq!(result.away_penalties, Some(4));
        assert!(!result.advancing_is_home());
        assert!(cup.knockout_rounds[0].completed);
    }

    #[test]
    fn decisive_result_carries_no_shootout() {
        let mut cup = make_knockout_cup();
        apply_simulated_result(&mut cup, 0, "home", "away", 2, 0, None);

        let result = cup.fixtures[0].result.as_ref().unwrap();
        assert_eq!(result.home_penalties, None);
        assert_eq!(result.away_penalties, None);
        assert!(result.advancing_is_home());
    }
    // ---------------------------------------------------------------------
    // Repairing fixtures that were stranded in the past (#608's aftermath).
    // ---------------------------------------------------------------------

    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::StandingEntry;
    use domain::manager::Manager;

    fn game_on(date: &str) -> Game {
        let parts: Vec<u32> = date.split('-').map(|p| p.parse().unwrap()).collect();
        let clock = GameClock::new(
            Utc.with_ymd_and_hms(parts[0] as i32, parts[1], parts[2], 12, 0, 0)
                .unwrap(),
        );
        let manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn league_with_fixture_on(date: &str) -> League {
        let mut league = League::new(
            "league1".to_string(),
            "Test League".to_string(),
            1,
            &["home".to_string(), "away".to_string()],
        );
        league.standings = vec![
            StandingEntry::new("home".to_string()),
            StandingEntry::new("away".to_string()),
        ];
        league.fixtures = vec![Fixture {
            id: "fix-1".to_string(),
            competition_id: "league1".to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: "home".to_string(),
            away_team_id: "away".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }];
        league
    }

    #[test]
    fn repairs_a_fixture_left_scheduled_in_the_past() {
        // #608 left fixtures `Scheduled` with a date that had passed. Nothing can pick them up
        // again — every due-today check matches on `date == today` — so the table is permanently
        // short and a knockout bracket permanently stalled.
        let mut game = game_on("2030-09-01");
        game.competitions = vec![league_with_fixture_on("2030-08-10")];

        let repaired = super::repair_stranded_fixtures(&mut game);

        assert_eq!(repaired, 1, "the stranded fixture is counted");
        assert_eq!(
            game.competitions[0].fixtures[0].status,
            FixtureStatus::Completed
        );
        assert!(game.competitions[0].fixtures[0].result.is_some());
        assert!(
            game.competitions[0]
                .standings
                .iter()
                .all(|entry| entry.played == 1),
            "the table records the match that was missing from it"
        );
    }

    #[test]
    fn leaves_today_and_the_future_alone() {
        // The repair must not eat the season. Only a date that has *passed* is unreachable; today's
        // fixtures are about to be played by the day's own code, and later ones are not due yet.
        let mut game = game_on("2030-09-01");
        game.competitions = vec![
            league_with_fixture_on("2030-09-01"),
            league_with_fixture_on("2030-09-08"),
        ];

        let repaired = super::repair_stranded_fixtures(&mut game);

        assert_eq!(repaired, 0, "nothing was stranded");
        for competition in &game.competitions {
            assert_eq!(
                competition.fixtures[0].status,
                FixtureStatus::Scheduled,
                "a fixture due today or later is not the repair's business"
            );
        }
    }

    #[test]
    fn is_idempotent_so_every_load_does_not_re_resave() {
        // It runs on every load, not behind a save-format gate, so the second run has to be a
        // no-op — otherwise every load reports a change and rewrites the save.
        let mut game = game_on("2030-09-01");
        game.competitions = vec![league_with_fixture_on("2030-08-10")];

        assert_eq!(super::repair_stranded_fixtures(&mut game), 1);
        let after_first = game.competitions[0].fixtures[0]
            .result
            .as_ref()
            .map(|result| (result.home_goals, result.away_goals));

        assert_eq!(
            super::repair_stranded_fixtures(&mut game),
            0,
            "a second load finds nothing left to repair"
        );
        assert_eq!(
            game.competitions[0].fixtures[0]
                .result
                .as_ref()
                .map(|result| (result.home_goals, result.away_goals)),
            after_first,
            "and does not re-roll the result it already recorded"
        );
    }

    #[test]
    fn leaves_an_in_progress_fixture_alone() {
        // A save taken mid-match has an `InProgress` fixture whose live session may be resumable.
        // Resolving it by scoreline would throw that away.
        let mut game = game_on("2030-09-01");
        let mut league = league_with_fixture_on("2030-08-10");
        league.fixtures[0].status = FixtureStatus::InProgress;
        game.competitions = vec![league];

        assert_eq!(super::repair_stranded_fixtures(&mut game), 0);
        assert_eq!(
            game.competitions[0].fixtures[0].status,
            FixtureStatus::InProgress
        );
    }
}
