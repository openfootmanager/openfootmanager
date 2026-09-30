//! The three ways a manager is put into a world that already exists.
//!
//! What differs between them is what the world already contains. A world with
//! competitions, news or stats of its own gets a takeover; a fresh one gets a
//! league and an opening-day inbox; and a mid-season start is built as if the
//! club were already half a season in.

use domain::stats::StatsState;

use crate::game::Game;
use crate::world::{preseason_league_year, preseason_season_start};

/// Stored on the generated league as its locale-neutral `name`, and shown only
/// when a client cannot resolve [`DEFAULT_LEAGUE_NAME_KEY`].
const DEFAULT_LEAGUE_NAME: &str = "Premier Division";

/// What the UI actually displays, via `League::name_key` and the `league`
/// message param. Mirrors `division_tier_name` / `division_tier_name_key`.
const DEFAULT_LEAGUE_NAME_KEY: &str = "tournaments.competitions.premierDivision";

/// The date format for backend-generated dates handed to the frontend: an
/// unambiguous, locale-neutral ISO day. `src/lib/dateFormatting.ts` renders it
/// in the player's own locale — a `%B` month name here would be English
/// whatever language they picked.
const ISO_DATE_FORMAT: &str = "%Y-%m-%d";

pub(super) fn has_existing_world_context(game: &Game, stats_state: &StatsState) -> bool {
    !game.competitions.is_empty()
        || game.league.is_some()
        || !game.news.is_empty()
        || !stats_state.player_matches.is_empty()
        || !stats_state.team_matches.is_empty()
}

pub(super) fn bootstrap_existing_world_takeover(
    game: &mut Game,
    team_id: &str,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    crate::ai_hiring::seed_ai_managers(game);

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let incumbent_manager_id = game
        .teams
        .iter()
        .find(|candidate| candidate.id == team_id)
        .and_then(|candidate| candidate.manager_id.clone());

    if incumbent_manager_id.as_deref() != Some(game.manager.id.as_str()) {
        let fired = crate::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
        if !fired
            && let Some(team) = game
                .teams
                .iter_mut()
                .find(|candidate| candidate.id == team_id)
        {
            team.manager_id = None;
        }
        crate::job_offers::hire_manager(game, team_id, &takeover_date)?;
    }

    let staff_msg = crate::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    crate::player_events::generate_takeover_contract_review_message(game);
    crate::season_context::refresh_game_context(game);

    Ok(stats_state)
}

pub(super) fn bootstrap_season_start(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    game.manager.hire(team_id.to_string());
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == team_id) {
        t.manager_id = Some(game.manager.id.clone());
    }
    game.manager_id = game.manager.id.clone();
    crate::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = crate::schedule::generate_league(
        DEFAULT_LEAGUE_NAME,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    league.name_key = Some(DEFAULT_LEAGUE_NAME_KEY.to_string());
    let friendlies = crate::schedule::generate_preseason_friendlies(&team_ids, season_start, 4);
    crate::schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    crate::season_context::refresh_game_context(game);

    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = crate::messages::welcome_message(&team_name, team_id, &date_str);
    game.messages.push(welcome_msg);

    // Both params are resolved frontend-side: the league name is a translation
    // key, and the ISO date is formatted in the player's locale.
    let season_msg = crate::messages::season_schedule_message(
        DEFAULT_LEAGUE_NAME_KEY,
        &season_start.format(ISO_DATE_FORMAT).to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news
        .push(crate::news::season_preview_article(&team_names, &date_str));

    let staff_msg = crate::messages::staff_advice_message(&team_name, team_id, &date_str);
    game.messages.push(staff_msg);

    crate::player_events::generate_takeover_contract_review_message(game);

    Ok(StatsState::default())
}

pub(super) fn competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
    game.league
        .as_ref()
        .map(|league| {
            league
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.counts_for_league_standings()
                        && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                })
                .count()
        })
        .unwrap_or_default()
}

pub(super) fn completed_competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
    game.league
        .as_ref()
        .map(|league| {
            league
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.counts_for_league_standings()
                        && fixture.status == domain::league::FixtureStatus::Completed
                        && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                })
                .count()
        })
        .unwrap_or_default()
}

pub(super) fn bootstrap_midseason_takeover(
    game: &mut Game,
    team_id: &str,
) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    crate::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = crate::schedule::generate_league(
        DEFAULT_LEAGUE_NAME,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    league.name_key = Some(DEFAULT_LEAGUE_NAME_KEY.to_string());
    game.league = Some(league);
    game.clock.current_date = season_start;
    crate::season_context::refresh_game_context(game);

    let total_fixtures = competitive_fixture_count_for_team(game, team_id);
    let target_completed = (total_fixtures / 2).max(1);
    let mut stats_state = StatsState::default();
    let mut safeguard_days = 0usize;
    while completed_competitive_fixture_count_for_team(game, team_id) < target_completed {
        let mut captures = Vec::new();
        crate::turn::process_day_with_capture(game, &mut |capture| captures.push(capture));
        for capture in captures {
            stats_state.append(capture);
        }
        safeguard_days += 1;
        if safeguard_days > 240 {
            break;
        }
    }

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let _ = crate::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
    crate::job_offers::hire_manager(game, team_id, &takeover_date)?;

    let staff_msg = crate::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    crate::player_events::generate_takeover_contract_review_message(game);
    crate::season_context::refresh_game_context(game);

    Ok(stats_state)
}
