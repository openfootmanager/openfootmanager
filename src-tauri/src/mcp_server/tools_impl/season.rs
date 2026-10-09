//! MCP tool implementations: season

use mcp_results::season::{
    AvailableJobs, AwardWinner, JobApplication, JobApplicationOutcome, JobOpening, SeasonAdvanced,
    SeasonAwards, SeasonStatus,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use std::sync::Arc;

// ─── season_check_complete ──────────────────────────────────────────────────

pub fn season_check_complete(ctx: Arc<McpContext>) -> Result<SeasonStatus, String> {
    let game = require_game(&ctx.state_manager)?;

    let Some(league) = &game.league else {
        return Ok(SeasonStatus::NoLeague {});
    };
    let remaining_fixtures = league
        .fixtures
        .iter()
        .filter(|f| f.status != domain::league::FixtureStatus::Completed)
        .count();
    if remaining_fixtures == 0 && !league.fixtures.is_empty() {
        Ok(SeasonStatus::Complete {})
    } else {
        Ok(SeasonStatus::InProgress { remaining_fixtures })
    }
}

// ─── season_advance ─────────────────────────────────────────────────────────

pub fn season_advance(ctx: Arc<McpContext>) -> Result<SeasonAdvanced, String> {
    let output = season_advance_for_state(&ctx.state_manager)?;
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }
    Ok(output)
}

fn season_advance_for_state(
    state: &ofm_core::state::StateManager,
) -> Result<SeasonAdvanced, String> {
    let response = crate::commands::season::advance_to_next_season_internal(state)?;
    Ok(SeasonAdvanced {
        fired: response["action"] == "fired",
        summary: response["summary"].clone(),
    })
}

// ─── help_find_tool ─────────────────────────────────────────────────────────

// ─── season_get_awards ──────────────────────────────────────────────────────

pub fn season_get_awards(ctx: Arc<McpContext>) -> Result<SeasonAwards, String> {
    let game = require_game(&ctx.state_manager)?;
    let awards = ofm_core::season_awards::compute_season_awards(&game);

    let winners = |entries: &[ofm_core::season_awards::AwardEntry]| -> Vec<AwardWinner> {
        entries
            .iter()
            .map(|e| AwardWinner {
                name: e.player_name.clone(),
                team: e.team_name.clone(),
                value: e.value,
            })
            .collect()
    };

    Ok(SeasonAwards {
        golden_boot: winners(&awards.golden_boot),
        assist_king: winners(&awards.assist_king),
        player_of_year: winners(&awards.player_of_year),
        clean_sheet_king: winners(&awards.clean_sheet_king),
        most_appearances: winners(&awards.most_appearances),
        young_player: winners(&awards.young_player),
        manager_of_season: awards
            .manager_of_season
            .iter()
            .map(|e| AwardWinner {
                name: e.manager_name.clone(),
                team: e.team_name.clone(),
                value: e.value,
            })
            .collect(),
    })
}

// ─── jobs_available ─────────────────────────────────────────────────────────

// ─── jobs_available ─────────────────────────────────────────────────────────

pub fn jobs_available(ctx: Arc<McpContext>) -> Result<AvailableJobs, String> {
    let game = require_game(&ctx.state_manager)?;
    let jobs = ofm_core::job_offers::get_available_jobs(&game);

    Ok(AvailableJobs {
        jobs: jobs
            .into_iter()
            .map(|job| JobOpening {
                team_id: job.team_id,
                team_name: job.team_name,
                city: job.city,
                reputation: job.reputation,
                last_league_position: job.last_league_position,
            })
            .collect(),
    })
}

// ─── jobs_apply ──────────────────────────────────────────────────────────────

// ─── jobs_apply ──────────────────────────────────────────────────────────────

pub fn jobs_apply(ctx: Arc<McpContext>, team_id: String) -> Result<JobApplication, String> {
    // `apply_for_job` reports its outcome as a value rather than an error, and
    // the old code committed unconditionally — so running it in place changes
    // nothing except closing the window between the read and the write.
    let result = ctx
        .state_manager
        .update_game(|game| ofm_core::job_offers::apply_for_job(game, &team_id))
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    use ofm_core::job_offers::JobApplicationResult as Applied;
    let outcome = match result {
        Applied::Hired => JobApplicationOutcome::Hired,
        Applied::Rejected => JobApplicationOutcome::Rejected,
        Applied::InvalidTeam => JobApplicationOutcome::InvalidTeam,
        Applied::AlreadyEmployed => JobApplicationOutcome::AlreadyEmployed,
        Applied::SameTeam => JobApplicationOutcome::SameTeam,
        Applied::NotBetterClub => JobApplicationOutcome::NotBetterClub,
    };

    Ok(JobApplication { outcome })
}

// ─── game_new ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::season_advance_for_state;
    use crate::commands::season::{advance_to_next_season_internal, tests::completed_checkpoint};
    use domain::league::FixtureStatus;
    use ofm_core::game::{BoardObjective, ObjectiveType};
    use ofm_core::state::StateManager;

    fn game_value(state: &StateManager) -> serde_json::Value {
        state
            .get_game(|game| serde_json::to_value(game).unwrap())
            .unwrap()
    }

    /// Given every required fixture completed, when the MCP adapter advances the season,
    /// then the season, table, fixtures, player stats and returned summary all roll over.
    #[test]
    fn season_advance_rolls_the_season_over() {
        let state = StateManager::new();
        state.set_game(completed_checkpoint());
        let advanced = season_advance_for_state(&state).unwrap();
        let text = advanced.to_string();
        let after = state.get_game(Clone::clone).unwrap();
        assert_eq!(after.competitions[0].season, 2026);
        assert!(after.competitions[0]
            .standings
            .iter()
            .all(|row| row.played == 0 && row.points == 0));
        assert_eq!(
            after.competitions[0]
                .fixtures
                .iter()
                .filter(|fixture| fixture.competition == domain::league::FixtureCompetition::League)
                .count(),
            2
        );
        assert!(after.competitions[0]
            .fixtures
            .iter()
            .all(|fixture| fixture.status == FixtureStatus::Scheduled && fixture.result.is_none()));
        assert!(after
            .players
            .iter()
            .all(|player| player.stats.goals == 0 && player.stats.appearances == 0));
        assert!(text.starts_with("## Season Advanced"), "{text}");
        assert!(text.contains("Completed Season Summary"), "{text}");
        assert!(!text.contains("Day Advanced"), "{text}");
    }

    /// Given a required scheduled fixture, when the MCP adapter requests rollover,
    /// then the exact season-not-complete key is returned without changing any game state.
    #[test]
    fn rolling_over_before_completion_is_refused() {
        let state = StateManager::new();
        let mut game = completed_checkpoint();
        game.competitions[0].fixtures[0].status = FixtureStatus::Scheduled;
        game.league = Some(game.competitions[0].clone());
        state.set_game(game);
        let before = game_value(&state);
        assert_eq!(
            season_advance_for_state(&state).unwrap_err(),
            "be.error.seasonNotComplete"
        );
        assert_eq!(game_value(&state), before);
    }

    /// Given a season just rolled over, when the MCP adapter requests another rollover,
    /// then it is refused without another prize payment, date tick or stats reset.
    #[test]
    fn rolling_over_twice_is_refused() {
        let state = StateManager::new();
        state.set_game(completed_checkpoint());
        season_advance_for_state(&state).unwrap();
        let before = game_value(&state);
        assert_eq!(
            season_advance_for_state(&state).unwrap_err(),
            "be.error.seasonNotComplete"
        );
        assert_eq!(game_value(&state), before);
    }

    /// Given a completed season with dismissal-triggering objectives,
    /// when the MCP adapter rolls over, then it returns the summary and firing together.
    #[test]
    fn the_board_can_fire_at_rollover() {
        let state = StateManager::new();
        let mut game = completed_checkpoint();
        game.manager.satisfaction = 20;
        game.manager.warning_stage = 1;
        game.board_objectives.push(BoardObjective {
            id: "objective".into(),
            objective_type: ObjectiveType::LeaguePosition,
            description: "Win the league".into(),
            target: 1,
            met: false,
        });
        state.set_game(game);
        let advanced = season_advance_for_state(&state).unwrap();
        let text = advanced.to_string();
        let after = state.get_game(Clone::clone).unwrap();
        assert_eq!(after.competitions[0].season, 2026);
        assert!(after.manager.team_id.is_none());
        assert!(text.contains("Completed Season Summary"), "{text}");
        assert!(text.contains("You have been fired"), "{text}");
        assert!(advanced.fired);
    }

    /// Given one database save loaded through two fresh readers,
    /// when the Tauri command and MCP adapter roll over their copies,
    /// then the complete games agree, including every generated fixture identity.
    #[test]
    fn the_tool_and_the_command_agree() {
        let directory = tempfile::tempdir().unwrap();
        let mut writer = db::save_manager::SaveManager::init(directory.path()).unwrap();
        let id = writer
            .create_save(&completed_checkpoint(), "Parity")
            .unwrap();
        drop(writer);
        let mut first_reader = db::save_manager::SaveManager::init(directory.path()).unwrap();
        let first = first_reader.load_game(&id).unwrap();
        drop(first_reader);
        let mut second_reader = db::save_manager::SaveManager::init(directory.path()).unwrap();
        let second = second_reader.load_game(&id).unwrap();
        let command = StateManager::new();
        let tool = StateManager::new();
        command.set_game(first);
        tool.set_game(second);
        advance_to_next_season_internal(&command).unwrap();
        season_advance_for_state(&tool).unwrap();
        let command_game = game_value(&command);
        let tool_game = game_value(&tool);
        let ids = |game: &serde_json::Value| {
            game["competitions"][0]["fixtures"]
                .as_array()
                .unwrap()
                .iter()
                .map(|fixture| fixture["id"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(&command_game),
            ids(&tool_game),
            "same checkpoint must regenerate identical fixture IDs"
        );
        assert_eq!(
            command_game, tool_game,
            "same checkpoint must produce an identical game"
        );
    }
}
