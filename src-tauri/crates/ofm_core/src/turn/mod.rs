mod news;
mod post_match;
mod round_summary;

use crate::board_objectives;
use crate::game::Game;
use crate::player_events;
use crate::random_events;
use crate::scouting;
use crate::training;
use crate::transfers;
use chrono::Datelike;
use domain::league::FixtureStatus;
use domain::player::Position as DomainPosition;
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

/// Process a single day advance.
pub fn process_day(game: &mut Game) {
    process_day_with_capture(game, &mut |_| {});
}

pub fn process_day_with_capture<F>(game: &mut Game, on_capture: &mut F)
where
    F: FnMut(StatsState),
{
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let has_match_today = game.league.as_ref().is_some_and(|league| {
        league
            .fixtures
            .iter()
            .any(|f| f.date == today && f.status == FixtureStatus::Scheduled)
    });

    if has_match_today {
        info!("[turn] process_day {}: matchday", today);
        simulate_matchday_with_capture(game, &today, on_capture);
    } else {
        let weekday_num = game.clock.current_date.weekday().num_days_from_monday();
        training::process_training(game, weekday_num);
        training::check_squad_fitness_warnings(game);
    }

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
    transfers::generate_incoming_transfer_offers(game);
    crate::ai_hiring::update_ai_manager_satisfaction(game);

    news::generate_weekly_digest_news(game, &today);
    news::generate_pre_match_messages(game, &today);

    crate::firing::check_manager_firing(game);
    crate::ai_hiring::process_vacant_ai_clubs(game);
    crate::job_offers::check_job_offers(game);

    debug!("[turn] process_day {}: complete, advancing clock", today);
    game.clock.advance_days(1);
    crate::season_context::refresh_game_context(game);
}

/// Called after a live match finishes to complete the day:
/// generates matchday news, pre-match messages, and advances the clock by one day.
pub fn finish_live_match_day(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    info!("[turn] finish_live_match_day: {}", today);
    generate_matchday_news(game, &today);

    crate::contracts::process_contract_expiries(game);

    board_objectives::generate_objectives(game);
    board_objectives::update_objective_progress(game);

    player_events::check_player_events(game);
    progress_injury_recovery(game);
    random_events::check_random_events(game);
    scouting::process_scouting(game);
    transfers::generate_incoming_transfer_offers(game);
    crate::ai_hiring::update_ai_manager_satisfaction(game);
    news::generate_weekly_digest_news(game, &today);
    news::generate_pre_match_messages(game, &today);

    crate::firing::check_manager_firing(game);
    crate::ai_hiring::process_vacant_ai_clubs(game);
    crate::job_offers::check_job_offers(game);

    game.clock.advance_days(1);
    crate::season_context::refresh_game_context(game);
}

// ---------------------------------------------------------------------------
// Domain → Engine type conversion
// ---------------------------------------------------------------------------

fn build_engine_team(game: &Game, team_id: &str) -> engine::TeamData {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (name, formation, play_style) = match team {
        Some(t) => (
            t.name.clone(),
            t.formation.clone(),
            match t.play_style {
                domain::team::PlayStyle::Attacking => engine::PlayStyle::Attacking,
                domain::team::PlayStyle::Defensive => engine::PlayStyle::Defensive,
                domain::team::PlayStyle::Possession => engine::PlayStyle::Possession,
                domain::team::PlayStyle::Counter => engine::PlayStyle::Counter,
                domain::team::PlayStyle::HighPress => engine::PlayStyle::HighPress,
                _ => engine::PlayStyle::Balanced,
            },
        ),
        None => (
            "Unknown".into(),
            "4-4-2".into(),
            engine::PlayStyle::Balanced,
        ),
    };

    let players: Vec<engine::PlayerData> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .map(|p| {
            let pos = match p.position.to_group_position() {
                DomainPosition::Goalkeeper => engine::Position::Goalkeeper,
                DomainPosition::Defender => engine::Position::Defender,
                DomainPosition::Midfielder => engine::Position::Midfielder,
                DomainPosition::Forward => engine::Position::Forward,
                _ => engine::Position::Midfielder,
            };
            engine::PlayerData {
                id: p.id.clone(),
                name: p.match_name.clone(),
                position: pos,
                condition: p.condition,
                fitness: p.fitness,
                pace: p.attributes.pace,
                stamina: p.attributes.stamina,
                strength: p.attributes.strength,
                agility: p.attributes.agility,
                passing: p.attributes.passing,
                shooting: p.attributes.shooting,
                tackling: p.attributes.tackling,
                dribbling: p.attributes.dribbling,
                defending: p.attributes.defending,
                positioning: p.attributes.positioning,
                vision: p.attributes.vision,
                decisions: p.attributes.decisions,
                composure: p.attributes.composure,
                aggression: p.attributes.aggression,
                teamwork: p.attributes.teamwork,
                leadership: p.attributes.leadership,
                handling: p.attributes.handling,
                reflexes: p.attributes.reflexes,
                aerial: p.attributes.aerial,
                traits: p.traits.iter().map(|t| format!("{:?}", t)).collect(),
            }
        })
        .collect();

    engine::TeamData {
        id: team_id.to_string(),
        name,
        formation,
        play_style,
        players,
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
    debug!(
        "[turn] simulate_other_matches: date={}, skip={:?}",
        today, skip_fixture
    );
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
    let (home_team_id, away_team_id) = {
        let f = &game.league.as_ref().unwrap().fixtures[idx];
        (f.home_team_id.clone(), f.away_team_id.clone())
    };

    let home_name = game
        .teams
        .iter()
        .find(|t| t.id == home_team_id)
        .map(|t| t.name.as_str())
        .unwrap_or("?");
    let away_name = game
        .teams
        .iter()
        .find(|t| t.id == away_team_id)
        .map(|t| t.name.as_str())
        .unwrap_or("?");
    debug!(
        "[turn] simulate_single_match: {} vs {} (fixture #{})",
        home_name, away_name, idx
    );

    let home_data = build_engine_team(game, &home_team_id);
    let away_data = build_engine_team(game, &away_team_id);
    let config = engine::MatchConfig::default();
    let report = engine::simulate(&home_data, &away_data, &config);

    info!(
        "[turn] match result: {} {} - {} {} (fixture #{})",
        home_name, report.home_goals, report.away_goals, away_name, idx
    );
    apply_match_report_with_capture(game, idx, &home_team_id, &away_team_id, &report, on_capture);
}
