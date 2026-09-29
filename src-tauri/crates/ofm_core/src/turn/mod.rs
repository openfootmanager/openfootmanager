mod dormant;
mod news;
mod post_match;
mod round_summary;
pub(crate) mod squad;

use crate::board_objectives;
use crate::game::Game;
use crate::live_match_manager;
use crate::player_events;
use crate::random_events;
use crate::scouting;
use crate::training;
use crate::transfers;
use chrono::Datelike;
use domain::league::FixtureStatus;
use domain::stats::StatsState;
use log::{debug, info};

// Re-export public items
pub use news::generate_matchday_news;
pub use post_match::{apply_match_report, apply_match_report_with_capture};
pub use round_summary::{
    NotableUpset, RoundResultSummary, RoundSummary, StandingDelta, TopScorerDelta,
    build_round_summary,
};

/// Progress injury recovery by one day for all currently injured players.
/// Players with 1 day remaining are cleared (fully recovered).
fn progress_injury_recovery(game: &mut Game) {
    for player in game.players.iter_mut() {
        if let Some(mut injury) = player.injury.take()
            && injury.days_remaining > 1
        {
            injury.days_remaining -= 1;
            player.injury = Some(injury);
        }
    }
}

fn competition_is_active(game: &Game, competition: &domain::league::League) -> bool {
    game.competition_in_active_scope(competition)
}

fn competition_indices_due_today(game: &Game, today: &str) -> Vec<usize> {
    if !game.competitions.is_empty() {
        return game
            .competitions
            .iter()
            .enumerate()
            // National-team tournaments are simulated by the national-team
            // engine, never the club match engine.
            .filter(|(_, competition)| {
                competition.kind != domain::league::CompetitionType::InternationalNation
            })
            .filter(|(_, competition)| competition_is_active(game, competition))
            .filter(|(_, competition)| {
                competition.fixtures.iter().any(|fixture| {
                    fixture.date == today && fixture.status == FixtureStatus::Scheduled
                })
            })
            .map(|(index, _)| index)
            .collect();
    }

    if game.league.as_ref().is_some_and(|league| {
        league
            .fixtures
            .iter()
            .any(|fixture| fixture.date == today && fixture.status == FixtureStatus::Scheduled)
    }) {
        vec![0]
    } else {
        Vec::new()
    }
}

/// Competitions OUTSIDE the active scope that have fixtures due today. These are
/// resolved cheaply (scoreline only) so the dormant world keeps moving. Returns
/// empty when no scope is configured (everything is active → nothing dormant).
fn dormant_competition_indices_due_today(game: &Game, today: &str) -> Vec<usize> {
    if game.competitions.is_empty() {
        return Vec::new();
    }
    game.competitions
        .iter()
        .enumerate()
        .filter(|(_, competition)| {
            competition.kind != domain::league::CompetitionType::InternationalNation
        })
        .filter(|(_, competition)| !competition_is_active(game, competition))
        .filter(|(_, competition)| {
            competition
                .fixtures
                .iter()
                .any(|fixture| fixture.date == today && fixture.status == FixtureStatus::Scheduled)
        })
        .map(|(index, _)| index)
        .collect()
}

fn simulate_competition_day_with_capture<F>(
    game: &mut Game,
    competition_index: usize,
    today: &str,
    on_capture: &mut F,
) where
    F: FnMut(StatsState),
{
    if competition_index >= game.competitions.len() {
        simulate_matchday_with_capture(game, today, on_capture);
        return;
    }

    // Simulation still reads the competition out of the legacy `game.league`
    // slot, so it has to be put there; move it rather than cloning it. A
    // `League` owns its fixtures, standings, transfer log and tournament state,
    // and the clone was discarded a few lines later anyway. Nothing reachable
    // from `simulate_matchday_with_capture` reads `game.competitions` — the
    // dormant path that does is driven separately from `process_day` — so the
    // vacated slot is never observed before it is filled back in.
    //
    // The move goes away once simulation takes the competition directly, or
    // once the legacy slot does; until then the borrow checker will not let it
    // be a borrow, because simulation needs the rest of `game` mutably too.
    game.league = Some(std::mem::take(&mut game.competitions[competition_index]));
    simulate_matchday_with_capture(game, today, on_capture);
    // Unconditional on purpose. While this cloned, an empty slot here meant the restore was
    // skipped and `game.competitions[index]` still held the original — harmless. Now the
    // competition is only in `game.league`, so skipping the restore would leave the slot holding
    // `League::default()` and lose that competition's fixtures, standings, transfer log and
    // tournament state, silently and permanently. Nothing on this path clears `game.league`, so
    // this cannot fire; if it ever does, failing loudly beats emptying a competition.
    let updated_competition = game
        .league
        .take()
        .expect("simulate_matchday must leave the competition in the legacy slot");
    game.competitions[competition_index] = updated_competition;
    game.sync_legacy_league();
}

/// A day at the training ground, for every club that is not playing.
///
/// Lives here rather than inline because a day has two entry points and both owe
/// the world the same one. `finish_live_match_day` used to run no training at
/// all, so on the day the player watched their own fixture, nobody in the game
/// recovered — the eighteen clubs with nothing on included.
fn run_training_ground(game: &mut Game) {
    let weekday_num = game.clock.current_date.weekday().num_days_from_monday();
    crate::ai_training::apply_ai_training_policies(game, weekday_num);
    training::process_training(game, weekday_num);
    training::check_squad_fitness_warnings(game);
    // Not a session, and deliberately not skipped for the clubs playing today:
    // a league plays whole rounds on one date, so a club whose review day landed
    // on its matchday would never review at all. It runs after active matches,
    // so those results are part of the form it reads. Dormant scoreline-only
    // competitions resolve later in `process_day` and enter the next review.
    crate::ai_tactics::apply_ai_tactical_reviews(game, weekday_num);
}

/// Everything both endings of a day do, in the order they both did it.
///
/// `process_day` and `finish_live_match_day` were two hand-maintained copies of this sequence,
/// and the live one had drifted: it never simulated the other competitions due that day, the
/// dormant tier, the internationals or the World Cup. Because a fixture is only ever due on an
/// exact date match, everything it skipped was skipped permanently. Sharing the tail means a step
/// can no longer be in one ending and not the other.
fn process_day_common(game: &mut Game, today: &str) {
    // AI clubs decide on their running-down contracts first, so a renewal made
    // on the day a contract ends lands before the expiry that would release him.
    let weekday_num = game.clock.current_date.weekday().num_days_from_monday();
    crate::ai_contracts::apply_ai_contract_decisions(game, weekday_num);
    crate::contracts::process_contract_expiries(game);

    // Weekly financial processing (wages, matchday income, warnings)
    crate::finances::process_weekly_finances(game);

    // Board objectives (generate if missing, update progress)
    board_objectives::generate_objectives(game);
    board_objectives::update_objective_progress(game);

    // Player conversations, random events, and scouting
    player_events::check_player_events(game);
    progress_injury_recovery(game);
    random_events::check_random_events(game);
    scouting::process_scouting(game);
    transfers::process_pending_transfer_registrations(game);
    transfers::process_pending_loan_registrations(game);
    transfers::generate_incoming_transfer_offers(game);
    // After every step above that can take a player away from a club — expiry,
    // registrations and the AI market — and the loan returns that opened the day.
    crate::ai_contracts::apply_ai_squad_planning(game, weekday_num);
    crate::squad_floor::keep_squads_at_the_floor(game);
    crate::generator::process_available_staff_market(game);
    crate::ai_hiring::update_ai_manager_satisfaction(game);

    news::generate_weekly_digest_news(game, today);
    news::generate_pre_match_messages(game, today);

    crate::firing::check_manager_firing(game);
    crate::ai_hiring::process_vacant_ai_clubs(game);
    crate::job_offers::check_job_offers(game);
}

/// The football that happens today outside the fixture the player was watching.
///
/// Every competition still holding a scheduled fixture for `today`, then the dormant tier, then
/// national-team football. Safe to call after the player's own match has been applied: a fixture
/// already `Completed` is no longer due, so nothing is played twice.
fn simulate_the_rest_of_the_world<F>(game: &mut Game, today: &str, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    let due = competition_indices_due_today(game, today);
    if !due.is_empty() {
        info!("[turn] {}: matchday in {} competition(s)", today, due.len());
    }
    for competition_index in due {
        simulate_competition_day_with_capture(game, competition_index, today, on_capture);
    }

    // Tiered simulation: competitions outside the active scope are resolved by
    // scoreline only, keeping the dormant world moving without the full engine.
    let dormant_competitions = dormant_competition_indices_due_today(game, today);
    if !dormant_competitions.is_empty() {
        let mut rng = rand::rng();
        for competition_index in dormant_competitions {
            dormant::simulate_dormant_competition_day(game, competition_index, today, &mut rng);
        }
    }

    // National-team football: window friendlies and any running World Cup.
    // Both self-filter by date, so they are no-ops on other days.
    crate::national_team::process_national_team_fixtures_due(game, today, &mut rand::rng());
    crate::world_cup::process_world_cup_fixtures_due(game, today, &mut rand::rng());
}

/// Process a single day advance.
pub fn process_day(game: &mut Game) {
    process_day_with_capture(game, &mut |_| {});
}

pub fn process_day_with_capture<F>(game: &mut Game, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    transfers::process_loan_development_reports(game);
    transfers::process_loan_returns(game);

    simulate_the_rest_of_the_world(game, &today, on_capture);

    // Unconditional, and after the matches: a fixture somewhere in the world says
    // nothing about whether *this* club trains. `run_training_ground` skips only
    // the clubs actually playing today.
    run_training_ground(game);

    process_day_common(game, &today);

    debug!("[turn] process_day {}: complete, advancing clock", today);
    game.clock.advance_days(1);
    crate::season_context::refresh_game_context(game);
}

/// Complete the day after the player's own match has been played and applied.
///
/// This runs *instead of* [`process_day`], not after it, so everything a day does has to happen
/// here too — including the football the player was not watching.
pub fn finish_live_match_day(game: &mut Game) {
    finish_live_match_day_with_capture(game, &mut |_| {});
}

/// [`finish_live_match_day`], keeping the stats produced by the matches it simulates.
pub fn finish_live_match_day_with_capture<F>(game: &mut Game, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    info!("[turn] finish_live_match_day: {}", today);
    transfers::process_loan_development_reports(game);
    transfers::process_loan_returns(game);
    generate_matchday_news(game, &today);

    // The rest of the world had a day too. The player's own fixture is already settled, so it is
    // no longer due and is not played twice; everything else due today would otherwise be
    // stranded, because a fixture is only ever due on an exact date match.
    simulate_the_rest_of_the_world(game, &today, on_capture);

    // The user's fixture is over; the rest of the world still had a day, and the
    // clubs that were not in it still had a session or a rest day.
    run_training_ground(game);

    process_day_common(game, &today);

    game.clock.advance_days(1);
    game.sync_legacy_league();
    crate::season_context::refresh_game_context(game);
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::finish_live_match_day;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::staff::{Staff, StaffAttributes, StaffRole};
    use domain::team::Team;

    fn make_team() -> Team {
        let mut team = Team::new(
            "team1".to_string(),
            "Test FC".to_string(),
            "TST".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            40_000,
        );
        team.finance = 5_000_000;
        team.wage_budget = 2_000_000;
        team
    }

    fn make_player() -> Player {
        let attrs = PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: 65,
            tackling: 65,
            dribbling: 65,
            defending: 65,
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: 20,
            reflexes: 30,
            aerial: 60,
        };
        let mut player = Player::new(
            "player1".to_string(),
            "Player".to_string(),
            "Test Player".to_string(),
            "1995-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attrs,
        );
        player.team_id = Some("team1".to_string());
        player.wage = 1_000;
        player
    }

    fn make_staff() -> Staff {
        let mut staff = Staff::new(
            "staff1".to_string(),
            "Staff".to_string(),
            "Coach".to_string(),
            "1980-01-01".to_string(),
            StaffRole::Coach,
            StaffAttributes {
                coaching: 70,
                judging_ability: 50,
                judging_potential: 50,
                physiotherapy: 30,
            },
        );
        staff.team_id = Some("team1".to_string());
        staff.nationality = "GB".to_string();
        staff.wage = 200;
        staff
    }

    #[test]
    fn finish_live_match_day_runs_weekly_finances_on_monday() {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let mut game = Game::new(
            clock,
            manager,
            vec![make_team()],
            vec![make_player()],
            vec![make_staff()],
            vec![],
        );
        let initial_finance = game.teams[0].finance;

        finish_live_match_day(&mut game);

        assert_eq!(game.teams[0].finance, initial_finance - 1_200);
    }
}

// ---------------------------------------------------------------------------
// Matchday simulation using the engine crate
// ---------------------------------------------------------------------------

fn simulate_matchday_with_capture<F>(game: &mut Game, today: &str, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    info!("[turn] simulate_matchday: {}", today);
    simulate_other_matches_with_capture(game, today, None, on_capture);
    generate_matchday_news(game, today);
}

/// Simulate all scheduled matches for `today`, optionally skipping one fixture
/// (the user's live match). Called by both process_day and advance_time_with_mode.
pub fn simulate_other_matches(game: &mut Game, today: &str, skip_fixture: Option<usize>) {
    simulate_other_matches_with_capture(game, today, skip_fixture, &mut |_| {});
}

pub fn simulate_other_matches_with_capture<F>(
    game: &mut Game,
    today: &str,
    skip_fixture: Option<usize>,
    on_capture: &mut F,
) where
    F: FnMut(StatsState),
{
    let fixture_indices: Vec<usize> = game.league.as_ref().map_or(vec![], |league| {
        league
            .fixtures
            .iter()
            .enumerate()
            .filter(|(i, f)| {
                f.date == today
                    && f.status == FixtureStatus::Scheduled
                    && (skip_fixture != Some(*i))
            })
            .map(|(i, _)| i)
            .collect()
    });

    for idx in fixture_indices {
        simulate_single_match_with_capture(game, idx, on_capture);
    }
}

fn simulate_single_match_with_capture<F>(game: &mut Game, idx: usize, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    // One match path. Every fixture in an active competition is played by the
    // engine the player watches: eleven starters, a real bench, and a manager on
    // both touchlines reading the game at the same checkpoints. It used to be
    // `engine::simulate`, a one-shot with no command loop — so no substitution
    // and no tactical change was possible in any match but the player's own.
    // Extra time, and the engine's own shootout, follow the one knockout
    // predicate inside `play_unwatched_fixture`.
    match live_match_manager::play_unwatched_fixture(game, idx) {
        Ok(played) => apply_match_report_with_capture(
            game,
            idx,
            &played.home_team_id,
            &played.away_team_id,
            &played.report,
            on_capture,
        ),
        // A side nobody could field, after the kick-off gate found no academy
        // player and no free agent: the world has run out of players, which
        // ordinary squad management exists to prevent. The day still finishes —
        // the fixture is settled by scoreline, as the dormant tier settles every
        // fixture, rather than left scheduled in the past.
        Err(error) => {
            log::error!(
                "[turn] fixture {idx} could not be played live ({error}); settled by scoreline"
            );
            let league = game
                .league
                .as_mut()
                .expect("simulate_matchday runs with the competition in the legacy slot");
            crate::catchup::resolve_fixture_by_scoreline(
                &game.players,
                league,
                idx,
                &mut rand::rng(),
            );
        }
    }
}
