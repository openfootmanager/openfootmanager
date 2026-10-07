//! MCP tool implementations: season

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use std::sync::Arc;

// ─── season_check_complete ──────────────────────────────────────────────────

pub fn season_check_complete(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    if let Some(league) = &game.league {
        let incomplete = league
            .fixtures
            .iter()
            .filter(|f| f.status != domain::league::FixtureStatus::Completed)
            .count();
        if incomplete == 0 && !league.fixtures.is_empty() {
            return Ok("## Season Status: Complete ✅\n\nAll fixtures played. Use `season_advance` to proceed.".to_string());
        }
        return Ok(format!(
            "## Season Status: In Progress\n\n**Remaining fixtures**: {}",
            incomplete
        ));
    }

    Ok("## Season Status: No league active.".to_string())
}

// ─── season_advance ─────────────────────────────────────────────────────────

pub fn season_advance(ctx: Arc<McpContext>) -> Result<String, String> {
    let output = season_advance_for_state(&ctx.state_manager)?;
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }
    Ok(output)
}

fn season_advance_for_state(state: &ofm_core::state::StateManager) -> Result<String, String> {
    let response = crate::commands::season::advance_to_next_season_internal(state)?;
    let dismissal = if response["action"] == "fired" {
        "\n\n**You have been fired.** Use `jobs_available` to find a new position."
    } else {
        ""
    };
    Ok(format!(
        "## Season Advanced\n\n### Completed Season Summary\n```json\n{:#}\n```{}\n\nUse `info_game_state` to inspect the regenerated season.",
        response["summary"], dismissal
    ))
}

// ─── help_find_tool ─────────────────────────────────────────────────────────

// ─── season_get_awards ──────────────────────────────────────────────────────

pub fn season_get_awards(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let awards = ofm_core::season_awards::compute_season_awards(&game);

    let mut output = String::from("## Season Awards\n\n");

    let categories = [
        ("🏆 Golden Boot", &awards.golden_boot),
        ("🅰️ Assist King", &awards.assist_king),
        ("⭐ Player of the Year", &awards.player_of_year),
        ("🧤 Clean Sheet King", &awards.clean_sheet_king),
        ("📋 Most Appearances", &awards.most_appearances),
        ("🌟 Young Player", &awards.young_player),
    ];

    for (title, entries) in &categories {
        if !entries.is_empty() {
            output.push_str(&format!(
                "### {}\n\n| # | Player | Team | Value |\n|---|--------|------|-------|\n",
                title
            ));
            for (i, e) in entries.iter().enumerate() {
                output.push_str(&format!(
                    "| {} | {} | {} | {:.1} |\n",
                    i + 1,
                    e.player_name,
                    e.team_name,
                    e.value
                ));
            }
            output.push('\n');
        }
    }

    if !awards.manager_of_season.is_empty() {
        output.push_str("### 👔 Manager of the Season\n\n| # | Manager | Team | Value |\n|---|---------|------|-------|\n");
        for (i, e) in awards.manager_of_season.iter().enumerate() {
            output.push_str(&format!(
                "| {} | {} | {} | {:.1} |\n",
                i + 1,
                e.manager_name,
                e.team_name,
                e.value
            ));
        }
    }

    Ok(output)
}

// ─── jobs_available ─────────────────────────────────────────────────────────

// ─── jobs_available ─────────────────────────────────────────────────────────

pub fn jobs_available(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let jobs = ofm_core::job_offers::get_available_jobs(&game);

    if jobs.is_empty() {
        return Ok("## Available Jobs\n\nNo job openings available right now.".to_string());
    }

    let mut output = format!("## Available Jobs ({} openings)\n\n| # | Team | City | Reputation | Last Position |\n|---|------|------|------------|---------------|\n", jobs.len());
    for (i, j) in jobs.iter().enumerate() {
        let pos = j
            .last_league_position
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());
        output.push_str(&format!(
            "| {} | {} ({}) | {} | {} | {} |\n",
            i + 1,
            j.team_name,
            j.team_id,
            j.city,
            j.reputation,
            pos
        ));
    }

    Ok(output)
}

// ─── jobs_apply ──────────────────────────────────────────────────────────────

// ─── jobs_apply ──────────────────────────────────────────────────────────────

pub fn jobs_apply(ctx: Arc<McpContext>, team_id: String) -> Result<String, String> {
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

    let result_text = match result {
        ofm_core::job_offers::JobApplicationResult::Hired => "✅ Hired! You are now the manager of this team.",
        ofm_core::job_offers::JobApplicationResult::Rejected => "❌ Rejected. The team chose another candidate.",
        ofm_core::job_offers::JobApplicationResult::InvalidTeam => "⚠️ Invalid team — no opening available.",
        ofm_core::job_offers::JobApplicationResult::AlreadyEmployed => "⚠️ You already have a team. Resign first.",
        ofm_core::job_offers::JobApplicationResult::SameTeam => "⚠️ You are already managing this team.",
        ofm_core::job_offers::JobApplicationResult::NotBetterClub => "⚠️ This club is not a step up from your current position. Only better clubs will consider an employed manager.",
    };

    Ok(format!("## Job Application Result\n\n{}", result_text))
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
        let text = season_advance_for_state(&state).unwrap();
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
        let text = season_advance_for_state(&state).unwrap();
        let after = state.get_game(Clone::clone).unwrap();
        assert_eq!(after.competitions[0].season, 2026);
        assert!(after.manager.team_id.is_none());
        assert!(text.contains("Completed Season Summary"), "{text}");
        assert!(text.contains("You have been fired"), "{text}");
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
