//! Game-takeover bootstrapping: resolving the simulation scope for a chosen
//! team and seeding the appropriate stats/world state for a fresh season start
//! or a mid-season takeover. Faithful extraction; behaviour is locked by the
//! bootstrap tests in `game::tests`.

use domain::stats::StatsState;
use ofm_core::game::Game;

use super::regions::{competition_required_region_ids, infer_team_region_id};
use super::startup::StartPhase;
use super::util::{
    default_league_name, long_date_format, preseason_league_year, preseason_season_start,
};

pub(crate) fn resolve_simulation_scope(
    game: &Game,
    team_id: &str,
    requested_region_ids: Option<Vec<String>>,
    requested_competition_ids: Option<Vec<String>>,
) -> Result<(Vec<String>, Vec<String>), String> {
    use std::collections::BTreeSet;

    let managed_team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;

    let mut active_region_ids: BTreeSet<String> = requested_region_ids
        .unwrap_or_default()
        .into_iter()
        .collect();
    active_region_ids.insert(infer_team_region_id(managed_team));

    let mut active_competition_ids: BTreeSet<String> = requested_competition_ids
        .unwrap_or_default()
        .into_iter()
        .filter(|competition_id| {
            game.competitions
                .iter()
                .any(|competition| competition.id == *competition_id)
        })
        .collect();

    for competition in game.competitions.iter().filter(|competition| {
        competition
            .participant_ids
            .iter()
            .any(|participant_id| participant_id == team_id)
    }) {
        active_competition_ids.insert(competition.id.clone());
    }

    if active_competition_ids.is_empty() {
        for competition in &game.competitions {
            let required_regions = competition_required_region_ids(competition);
            if required_regions.is_empty()
                || required_regions
                    .iter()
                    .all(|region_id| active_region_ids.contains(region_id))
            {
                active_competition_ids.insert(competition.id.clone());
            }
        }
    }

    for competition in game
        .competitions
        .iter()
        .filter(|competition| active_competition_ids.contains(&competition.id))
    {
        for region_id in competition_required_region_ids(competition) {
            active_region_ids.insert(region_id);
        }
    }

    let mut resolved_region_ids: Vec<String> = active_region_ids.into_iter().collect();
    resolved_region_ids.sort();

    let mut resolved_competition_ids: Vec<String> = active_competition_ids.into_iter().collect();
    resolved_competition_ids.sort_by_key(|competition_id| {
        game.competitions
            .iter()
            .find(|competition| competition.id == *competition_id)
            .map(|competition| competition.priority)
            .unwrap_or(u32::MAX)
    });

    Ok((resolved_region_ids, resolved_competition_ids))
}

fn has_existing_world_context(game: &Game, stats_state: &StatsState) -> bool {
    !game.competitions.is_empty()
        || game.league.is_some()
        || !game.news.is_empty()
        || !stats_state.player_matches.is_empty()
        || !stats_state.team_matches.is_empty()
}

fn bootstrap_existing_world_takeover(
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

    ofm_core::ai_hiring::seed_ai_managers(game);

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let incumbent_manager_id = game
        .teams
        .iter()
        .find(|candidate| candidate.id == team_id)
        .and_then(|candidate| candidate.manager_id.clone());

    if incumbent_manager_id.as_deref() != Some(game.manager.id.as_str()) {
        let fired = ofm_core::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
        if !fired {
            if let Some(team) = game
                .teams
                .iter_mut()
                .find(|candidate| candidate.id == team_id)
            {
                team.manager_id = None;
            }
        }
        ofm_core::job_offers::hire_manager(game, team_id, &takeover_date)?;
    }

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    ofm_core::player_events::generate_takeover_contract_review_message(game);
    ofm_core::season_context::refresh_game_context(game);

    Ok(stats_state)
}

fn bootstrap_season_start(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
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
    ofm_core::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let league_name = default_league_name();
    let mut league = ofm_core::schedule::generate_league(
        &league_name,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    );
    let friendlies = ofm_core::schedule::generate_preseason_friendlies(&team_ids, season_start, 4);
    ofm_core::schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    ofm_core::season_context::refresh_game_context(game);

    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = ofm_core::messages::welcome_message(&team_name, team_id, &date_str);
    game.messages.push(welcome_msg);

    let season_msg = ofm_core::messages::season_schedule_message(
        &league_name,
        &season_start.format(&long_date_format()).to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(ofm_core::news::season_preview_article(
        &team_names,
        &date_str,
    ));

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &date_str);
    game.messages.push(staff_msg);

    ofm_core::player_events::generate_takeover_contract_review_message(game);

    Ok(StatsState::default())
}

fn competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
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

fn completed_competitive_fixture_count_for_team(game: &Game, team_id: &str) -> usize {
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

fn bootstrap_midseason_takeover(game: &mut Game, team_id: &str) -> Result<StatsState, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;
    let team_name = team.name.clone();

    ofm_core::ai_hiring::seed_ai_managers(game);

    let season_start = preseason_season_start(&game.clock);
    let league_name = default_league_name();
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    game.league = Some(ofm_core::schedule::generate_league(
        &league_name,
        preseason_league_year(&game.clock),
        &team_ids,
        season_start,
    ));
    game.clock.current_date = season_start;
    ofm_core::season_context::refresh_game_context(game);

    let total_fixtures = competitive_fixture_count_for_team(game, team_id);
    let target_completed = (total_fixtures / 2).max(1);
    let mut stats_state = StatsState::default();
    let mut safeguard_days = 0usize;
    while completed_competitive_fixture_count_for_team(game, team_id) < target_completed {
        let mut captures = Vec::new();
        ofm_core::turn::process_day_with_capture(game, &mut |capture| captures.push(capture));
        for capture in captures {
            stats_state.append(capture);
        }
        safeguard_days += 1;
        if safeguard_days > 240 {
            break;
        }
    }

    let takeover_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let _ = ofm_core::firing::fire_ai_manager_for_team(game, team_id, &takeover_date);
    ofm_core::job_offers::hire_manager(game, team_id, &takeover_date)?;

    let staff_msg = ofm_core::messages::staff_advice_message(&team_name, team_id, &takeover_date);
    game.messages.push(staff_msg);
    ofm_core::player_events::generate_takeover_contract_review_message(game);
    ofm_core::season_context::refresh_game_context(game);

    Ok(stats_state)
}

pub(crate) fn bootstrap_team_selection(
    game: &mut Game,
    team_id: &str,
    start_phase: StartPhase,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let stats_state = if has_existing_world_context(game, &stats_state) {
        bootstrap_existing_world_takeover(game, team_id, stats_state)?
    } else {
        match start_phase {
            StartPhase::SeasonStart => bootstrap_season_start(game, team_id)?,
            StartPhase::MidSeason => bootstrap_midseason_takeover(game, team_id)?,
        }
    };

    ofm_core::transfers::seed_opening_ai_loan_market(game);
    Ok(stats_state)
}
