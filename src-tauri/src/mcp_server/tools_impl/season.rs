//! MCP tool implementations: season

use mcp_results::season::{
    AvailableJobs, AwardWinner, JobApplication, JobApplicationOutcome, JobOpening, SeasonAdvanced,
    SeasonAwards, SeasonStatus,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use ofm_core::game::Game;
use ofm_core::state::Career;
use std::sync::Arc;

/// Where the user's season stands, for the agent's status tools.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SeasonProgress {
    /// `season_advance` will roll the season over.
    Complete,
    /// League fixtures left in the tables that decide the season (`end_of_season::season_tables`).
    OwnLeagueLeft(usize),
    /// Those tables are played out, but the season is not over: another division of his country is
    /// still playing.
    OtherDivisionsStillPlaying,
    NoLeague,
}

/// The verdict is the game's own rule, `end_of_season::is_season_complete`, the one
/// `season_advance` is refused by, and the count is over the same tables it judges
/// (`season_tables`). Counting the `game.league` mirror told an agent "complete" while a lower
/// division still had rounds to play (Brazil's second division starts two months after its first),
/// and "your league has played all its fixtures" through an Argentine Clausura, the mirror being
/// the finished Apertura.
pub(crate) fn season_progress(game: &Game) -> SeasonProgress {
    if ofm_core::end_of_season::is_season_complete(game) {
        return SeasonProgress::Complete;
    }
    let tables = ofm_core::end_of_season::season_tables(game);
    if tables.is_empty() {
        return SeasonProgress::NoLeague;
    }
    let remaining_fixtures = tables
        .iter()
        .flat_map(|table| &table.fixtures)
        .filter(|fixture| {
            fixture.counts_for_league_standings()
                && fixture.status != domain::league::FixtureStatus::Completed
        })
        .count();
    if remaining_fixtures == 0 {
        SeasonProgress::OtherDivisionsStillPlaying
    } else {
        SeasonProgress::OwnLeagueLeft(remaining_fixtures)
    }
}

// ─── season_check_complete ──────────────────────────────────────────────────

pub fn season_check_complete(ctx: Arc<McpContext>) -> Result<SeasonStatus, String> {
    let game = require_game(&ctx.state_manager)?;
    Ok(season_status(&game))
}

fn season_status(game: &Game) -> SeasonStatus {
    match season_progress(game) {
        SeasonProgress::Complete => SeasonStatus::Complete {},
        SeasonProgress::OwnLeagueLeft(remaining_fixtures) => {
            SeasonStatus::InProgress { remaining_fixtures }
        }
        SeasonProgress::OtherDivisionsStillPlaying => SeasonStatus::OtherDivisionsStillPlaying {},
        SeasonProgress::NoLeague => SeasonStatus::NoLeague {},
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
    let response = crate::commands::season::advance_to_next_season_internal(state, Career::Active)?;
    Ok(SeasonAdvanced {
        fired: response["action"] == "fired",
        summary: response["summary"].clone(),
    })
}

// ─── help_find_tool ─────────────────────────────────────────────────────────

// ─── season_get_awards ──────────────────────────────────────────────────────

/// The club an award names, as the agent reads it. A winner at no club has no name stored (the
/// screens word it in the player's language), so the agent, who reads English, is told so.
fn award_team_label(team_name: &str) -> String {
    if team_name.is_empty() {
        "Free Agent".to_string()
    } else {
        team_name.to_string()
    }
}

pub fn season_get_awards(ctx: Arc<McpContext>) -> Result<SeasonAwards, String> {
    let game = require_game(&ctx.state_manager)?;
    // The screen's awards: the user's league (`end_of_season::user_division`), so an agent and the
    // player read the same race.
    let awards = ofm_core::season_awards::user_division_season_awards(&game);

    let winners = |entries: &[ofm_core::season_awards::AwardEntry]| -> Vec<AwardWinner> {
        entries
            .iter()
            .map(|e| AwardWinner {
                name: e.player_name.clone(),
                team: award_team_label(&e.team_name),
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
                team: award_team_label(&e.team_name),
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
        .update_game_for(ofm_core::state::Career::Active, |game| {
            ofm_core::job_offers::apply_for_job(game, &team_id)
        })?;

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
    use super::{award_team_label, season_advance_for_state, season_status};
    use crate::commands::season::{advance_to_next_season_internal, tests::completed_checkpoint};
    use domain::league::FixtureStatus;
    use mcp_results::season::SeasonStatus;
    use ofm_core::game::{BoardObjective, ObjectiveType};
    use ofm_core::state::StateManager;

    /// Given an award won by a player at no club (no team name stored)
    /// When the agent reads it
    /// Then the team column says "Free Agent", never a blank cell; a club keeps its name.
    #[test]
    fn an_award_won_at_no_club_reads_free_agent_to_the_agent() {
        assert_eq!(award_team_label(""), "Free Agent");
        assert_eq!(award_team_label("Alpha FC"), "Alpha FC");
    }

    /// The completed checkpoint with a second English division that has kicked off and still has a
    /// round to play after today (the shape of Brazil's staggered second division).
    fn with_a_second_division_still_playing() -> ofm_core::game::Game {
        use domain::league::{Fixture, League, MatchResult, StandingEntry};
        let mut game = completed_checkpoint();
        let played = Fixture {
            id: "second-first".into(),
            competition_id: "second".into(),
            date: "2025-06-01".into(),
            home_team_id: "b1".into(),
            away_team_id: "b2".into(),
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: 1,
                away_goals: 0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let to_come = Fixture {
            id: "second-return".into(),
            competition_id: "second".into(),
            date: "2025-06-20".into(),
            home_team_id: "b2".into(),
            away_team_id: "b1".into(),
            status: FixtureStatus::Scheduled,
            ..Default::default()
        };
        let mut b1 = StandingEntry::new("b1".into());
        b1.played = 1;
        let mut b2 = StandingEntry::new("b2".into());
        b2.played = 1;
        game.competitions.push(League {
            id: "second".into(),
            name: "Second Division".into(),
            season: 2025,
            participant_ids: vec!["b1".into(), "b2".into()],
            country_id: Some("ENG".into()),
            fixtures: vec![played, to_come],
            standings: vec![b1, b2],
            ..Default::default()
        });
        game
    }

    /// Given a split season whose Apertura is finished, the `game.league` mirror, while the
    ///      Clausura, which the user's club also plays in, has a fixture to come
    /// When the agent asks whether the season is complete
    /// Then it is in progress with that fixture left (it said the user's league had played all its
    ///      fixtures, reading the mirror)
    #[test]
    fn a_split_season_counts_the_table_still_playing() {
        use domain::league::{Fixture, League};
        let mut game = completed_checkpoint();
        game.competitions.push(League {
            id: "clausura".into(),
            name: "Clausura".into(),
            season: 2025,
            participant_ids: vec!["home".into(), "away".into()],
            country_id: Some("ENG".into()),
            fixtures: vec![Fixture {
                id: "clausura-first".into(),
                competition_id: "clausura".into(),
                date: "2025-06-20".into(),
                home_team_id: "home".into(),
                away_team_id: "away".into(),
                status: FixtureStatus::Scheduled,
                ..Default::default()
            }],
            ..Default::default()
        });

        assert_eq!(
            season_status(&game),
            SeasonStatus::InProgress {
                remaining_fixtures: 1
            }
        );
    }

    /// Given the user's league has played every fixture and nothing else in his country is playing
    /// When the agent asks whether the season is complete
    /// Then it is
    #[test]
    fn a_finished_season_reads_complete() {
        assert_eq!(
            season_status(&completed_checkpoint()),
            SeasonStatus::Complete {}
        );
    }

    /// Given the user's league has played every fixture but a second division of his country still
    ///      has a round to play
    /// When the agent asks whether the season is complete
    /// Then it is not: the other division is still playing, and `season_advance` is refused alike
    ///      (it read Complete, and the advance was then refused)
    #[test]
    fn a_division_still_playing_keeps_the_season_open() {
        let game = with_a_second_division_still_playing();

        assert_eq!(
            season_status(&game),
            SeasonStatus::OtherDivisionsStillPlaying {}
        );
        let state = StateManager::new();
        state.set_game(game);
        assert_eq!(
            season_advance_for_state(&state).map(|_| ()),
            Err("be.error.seasonNotComplete".to_string())
        );
    }

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
        advance_to_next_season_internal(&command, ofm_core::state::Career::Active).unwrap();
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
