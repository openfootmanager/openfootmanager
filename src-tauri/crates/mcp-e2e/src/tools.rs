//! One typed method per tool, and the catalog they were written against.
//!
//! `catalog_matches_the_live_server` compares [`CATALOG`] with the running server's `tools/list`,
//! so a tool added, renamed, or given another parameter or parameter type fails there until the
//! method is updated here.

use serde_json::{json, Map, Value};

use crate::client::{CallError, Client};
use crate::results::{
    club, contracts, game, help, inbox, info, live_match, scouting, season, squad, time, training,
    transfers,
};

/// The arguments of `game_new`, which has too many to pass one by one.
#[derive(Debug, Clone, Default)]
pub struct NewGame<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub nationality: &'a str,
    pub seed: Option<u64>,
    pub start_phase: Option<&'a str>,
    pub start_year: Option<i64>,
    pub team_id: Option<&'a str>,
    pub world_source: Option<&'a str>,
}

pub type Reply<T> = Result<T, CallError>;

/// (tool, (parameter, JSON type) pairs, required parameters), each sorted, as `tools/list` reports them.
pub type CatalogEntry = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
);

pub const CATALOG: &[CatalogEntry] = &[
    ("club_request_board_support", &[], &[]),
    ("club_request_marketing", &[], &[]),
    ("club_request_sponsor_pitch", &[], &[]),
    (
        "club_upgrade_facility",
        &[("facility", "string")],
        &["facility"],
    ),
    (
        "contract_clear_exit_intent",
        &[("player_id", "string")],
        &["player_id"],
    ),
    (
        "contract_delegate_renewals",
        &[
            ("max_contract_years", "integer"),
            ("max_wage_increase_pct", "integer"),
            ("player_ids", "array"),
        ],
        &["max_contract_years", "max_wage_increase_pct"],
    ),
    (
        "contract_preview_renewal",
        &[("player_id", "string"), ("weekly_wage", "integer")],
        &["player_id", "weekly_wage"],
    ),
    (
        "contract_preview_termination",
        &[("player_id", "string")],
        &["player_id"],
    ),
    (
        "contract_propose_renewal",
        &[
            ("contract_years", "integer"),
            ("player_id", "string"),
            ("weekly_wage", "integer"),
        ],
        &["contract_years", "player_id", "weekly_wage"],
    ),
    (
        "contract_set_exit_intent",
        &[("player_id", "string"), ("reason", "string")],
        &["player_id"],
    ),
    (
        "contract_terminate",
        &[("player_id", "string")],
        &["player_id"],
    ),
    ("game_delete_save", &[("save_id", "string")], &["save_id"]),
    ("game_exit", &[], &[]),
    ("game_export_world", &[], &[]),
    ("game_is_finished", &[], &[]),
    ("game_list_saves", &[], &[]),
    ("game_list_world_databases", &[], &[]),
    ("game_load_save", &[("save_id", "string")], &["save_id"]),
    (
        "game_new",
        &[
            ("first_name", "string"),
            ("last_name", "string"),
            ("nationality", "string"),
            ("seed", "integer"),
            ("start_phase", "string"),
            ("start_year", "integer"),
            ("team_id", "string"),
            ("world_source", "string"),
        ],
        &["first_name", "last_name", "nationality"],
    ),
    ("game_save", &[], &[]),
    ("game_select_team", &[("team_id", "string")], &["team_id"]),
    ("help_find_tool", &[("query", "string")], &["query"]),
    ("help_list_categories", &[], &[]),
    ("inbox_clear_old", &[], &[]),
    ("inbox_delete", &[("message_id", "string")], &["message_id"]),
    (
        "inbox_get_messages",
        &[("category", "string"), ("unread_only", "boolean")],
        &[],
    ),
    ("inbox_mark_all_read", &[], &[]),
    (
        "inbox_mark_read",
        &[("message_id", "string")],
        &["message_id"],
    ),
    (
        "inbox_resolve_action",
        &[
            ("action_id", "string"),
            ("message_id", "string"),
            ("option_id", "string"),
        ],
        &["action_id", "message_id"],
    ),
    ("info_finance_snapshot", &[("team_id", "string")], &[]),
    ("info_finances", &[], &[]),
    ("info_fixtures", &[], &[]),
    ("info_game_state", &[], &[]),
    ("info_game_summary", &[], &[]),
    ("info_match_preview", &[], &[]),
    ("info_news", &[], &[]),
    (
        "info_player_match_history",
        &[("limit", "integer"), ("player_id", "string")],
        &["player_id"],
    ),
    (
        "info_player_profile",
        &[("player_id", "string")],
        &["player_id"],
    ),
    (
        "info_player_stats",
        &[("player_id", "string")],
        &["player_id"],
    ),
    ("info_season_context", &[], &[]),
    ("info_standings", &[], &[]),
    (
        "info_team_match_history",
        &[("limit", "integer"), ("team_id", "string")],
        &["team_id"],
    ),
    ("info_team_profile", &[("team_id", "string")], &["team_id"]),
    ("info_team_stats", &[("team_id", "string")], &["team_id"]),
    ("jobs_apply", &[("team_id", "string")], &["team_id"]),
    ("jobs_available", &[], &[]),
    (
        "match_command",
        &[("command_json", "string")],
        &["command_json"],
    ),
    ("match_finish", &[], &[]),
    (
        "match_press_conference",
        &[("answers_json", "string")],
        &["answers_json"],
    ),
    ("match_snapshot", &[], &[]),
    (
        "match_start",
        &[
            ("allows_extra_time", "boolean"),
            ("competition_id", "string"),
            ("fixture_id", "string"),
            ("fixture_index", "integer"),
            ("mode", "string"),
        ],
        &["mode"],
    ),
    ("match_step", &[("minutes", "integer")], &["minutes"]),
    (
        "match_team_talk",
        &[("context", "string"), ("tone", "string")],
        &["context", "tone"],
    ),
    ("ping", &[], &[]),
    ("scout_get_reports", &[], &[]),
    (
        "scout_send",
        &[("player_id", "string"), ("scout_id", "string")],
        &["player_id", "scout_id"],
    ),
    (
        "scout_youth_cancel",
        &[("assignment_id", "string")],
        &["assignment_id"],
    ),
    (
        "scout_youth_reassign",
        &[("assignment_id", "string"), ("scout_id", "string")],
        &["assignment_id", "scout_id"],
    ),
    (
        "scout_youth_start",
        &[
            ("objective", "string"),
            ("region", "string"),
            ("scout_id", "string"),
            ("target_position", "string"),
        ],
        &["scout_id"],
    ),
    ("season_advance", &[], &[]),
    ("season_check_complete", &[], &[]),
    ("season_get_awards", &[], &[]),
    ("squad_auto_set_pieces", &[], &[]),
    ("squad_get", &[], &[]),
    (
        "squad_set_formation",
        &[("formation", "string")],
        &["formation"],
    ),
    (
        "squad_set_match_roles",
        &[
            ("captain", "string"),
            ("corner_taker", "string"),
            ("free_kick_taker", "string"),
            ("penalty_taker", "string"),
            ("vice_captain", "string"),
        ],
        &[],
    ),
    (
        "squad_set_play_style",
        &[("play_style", "string")],
        &["play_style"],
    ),
    (
        "squad_set_player_role",
        &[("player_id", "string"), ("squad_role", "string")],
        &["player_id", "squad_role"],
    ),
    (
        "squad_set_starting_xi",
        &[("player_ids", "array")],
        &["player_ids"],
    ),
    ("staff_get", &[], &[]),
    ("staff_hire", &[("staff_id", "string")], &["staff_id"]),
    ("staff_release", &[("staff_id", "string")], &["staff_id"]),
    ("time_advance", &[], &[]),
    ("time_check_blockers", &[], &[]),
    ("time_skip_to_match_day", &[], &[]),
    ("training_get", &[], &[]),
    (
        "training_set_focus_intensity",
        &[("focus", "string"), ("intensity", "string")],
        &["focus", "intensity"],
    ),
    (
        "training_set_groups",
        &[("groups_json", "string")],
        &["groups_json"],
    ),
    (
        "training_set_player_focus",
        &[("focus", "string"), ("player_id", "string")],
        &["player_id"],
    ),
    (
        "training_set_schedule",
        &[("schedule", "string")],
        &["schedule"],
    ),
    (
        "transfer_counter_offer",
        &[
            ("offer_id", "string"),
            ("player_id", "string"),
            ("requested_fee", "integer"),
        ],
        &["offer_id", "player_id", "requested_fee"],
    ),
    (
        "transfer_free_agent_offer",
        &[
            ("contract_years", "integer"),
            ("player_id", "string"),
            ("weekly_wage", "integer"),
        ],
        &["contract_years", "player_id", "weekly_wage"],
    ),
    (
        "transfer_free_agent_preview",
        &[("player_id", "string"), ("weekly_wage", "integer")],
        &["player_id", "weekly_wage"],
    ),
    (
        "transfer_make_bid",
        &[("fee", "integer"), ("player_id", "string")],
        &["fee", "player_id"],
    ),
    (
        "transfer_market_browse",
        &[
            ("listed_only", "boolean"),
            ("max_price", "integer"),
            ("position", "string"),
        ],
        &[],
    ),
    (
        "transfer_preview_bid",
        &[("fee", "integer"), ("player_id", "string")],
        &["fee", "player_id"],
    ),
    (
        "transfer_respond_to_offer",
        &[
            ("accept", "boolean"),
            ("offer_id", "string"),
            ("player_id", "string"),
        ],
        &["accept", "offer_id", "player_id"],
    ),
    (
        "transfer_toggle_listed",
        &[("player_id", "string")],
        &["player_id"],
    ),
    (
        "transfer_toggle_loan",
        &[("player_id", "string")],
        &["player_id"],
    ),
];

fn object<const N: usize>(entries: [(&str, Option<Value>); N]) -> Value {
    let map: Map<String, Value> = entries
        .into_iter()
        .filter_map(|(name, value)| Some((name.to_string(), value?)))
        .collect();
    Value::Object(map)
}

impl Client {
    pub fn club_request_board_support(&self) -> Reply<club::BoardSupport> {
        self.call("club_request_board_support", object([]))
    }

    pub fn club_request_marketing(&self) -> Reply<club::MarketingCampaign> {
        self.call("club_request_marketing", object([]))
    }

    pub fn club_request_sponsor_pitch(&self) -> Reply<club::SponsorPitch> {
        self.call("club_request_sponsor_pitch", object([]))
    }

    pub fn club_upgrade_facility(&self, facility: &str) -> Reply<club::FacilityUpgraded> {
        self.call(
            "club_upgrade_facility",
            object([("facility", Some(json!(facility)))]),
        )
    }

    pub fn contract_clear_exit_intent(
        &self,
        player_id: &str,
    ) -> Reply<contracts::ExitIntentCleared> {
        self.call(
            "contract_clear_exit_intent",
            object([("player_id", Some(json!(player_id)))]),
        )
    }

    pub fn contract_delegate_renewals(
        &self,
        max_contract_years: i64,
        max_wage_increase_pct: i64,
        player_ids: Option<&[String]>,
    ) -> Reply<contracts::RenewalsDelegated> {
        self.call(
            "contract_delegate_renewals",
            object([
                ("max_contract_years", Some(json!(max_contract_years))),
                ("max_wage_increase_pct", Some(json!(max_wage_increase_pct))),
                ("player_ids", player_ids.map(|v| json!(v))),
            ]),
        )
    }

    pub fn contract_preview_renewal(
        &self,
        player_id: &str,
        weekly_wage: i64,
    ) -> Reply<contracts::RenewalPreview> {
        self.call(
            "contract_preview_renewal",
            object([
                ("player_id", Some(json!(player_id))),
                ("weekly_wage", Some(json!(weekly_wage))),
            ]),
        )
    }

    pub fn contract_preview_termination(
        &self,
        player_id: &str,
    ) -> Reply<contracts::TerminationPreview> {
        self.call(
            "contract_preview_termination",
            object([("player_id", Some(json!(player_id)))]),
        )
    }

    pub fn contract_propose_renewal(
        &self,
        contract_years: i64,
        player_id: &str,
        weekly_wage: i64,
    ) -> Reply<contracts::RenewalProposed> {
        self.call(
            "contract_propose_renewal",
            object([
                ("contract_years", Some(json!(contract_years))),
                ("player_id", Some(json!(player_id))),
                ("weekly_wage", Some(json!(weekly_wage))),
            ]),
        )
    }

    pub fn contract_set_exit_intent(
        &self,
        player_id: &str,
        reason: Option<&str>,
    ) -> Reply<contracts::ExitIntentSet> {
        self.call(
            "contract_set_exit_intent",
            object([
                ("player_id", Some(json!(player_id))),
                ("reason", reason.map(|v| json!(v))),
            ]),
        )
    }

    pub fn contract_terminate(&self, player_id: &str) -> Reply<contracts::ContractTerminated> {
        self.call(
            "contract_terminate",
            object([("player_id", Some(json!(player_id)))]),
        )
    }

    pub fn game_delete_save(&self, save_id: &str) -> Reply<game::SaveDeleted> {
        self.call(
            "game_delete_save",
            object([("save_id", Some(json!(save_id)))]),
        )
    }

    pub fn game_exit(&self) -> Reply<game::ReturnedToMenu> {
        self.call("game_exit", object([]))
    }

    pub fn game_export_world(&self) -> Reply<game::WorldExported> {
        self.call("game_export_world", object([]))
    }

    pub fn game_is_finished(&self) -> Reply<info::GameStatus> {
        self.call("game_is_finished", object([]))
    }

    pub fn game_list_saves(&self) -> Reply<game::SaveList> {
        self.call("game_list_saves", object([]))
    }

    pub fn game_list_world_databases(&self) -> Reply<game::WorldDatabases> {
        self.call("game_list_world_databases", object([]))
    }

    pub fn game_load_save(&self, save_id: &str) -> Reply<game::SaveLoaded> {
        self.call(
            "game_load_save",
            object([("save_id", Some(json!(save_id)))]),
        )
    }

    pub fn game_new(&self, request: &NewGame<'_>) -> Reply<game::GameCreated> {
        self.call(
            "game_new",
            object([
                ("first_name", Some(json!(request.first_name))),
                ("last_name", Some(json!(request.last_name))),
                ("nationality", Some(json!(request.nationality))),
                ("seed", request.seed.map(|v| json!(v))),
                ("start_phase", request.start_phase.map(|v| json!(v))),
                ("start_year", request.start_year.map(|v| json!(v))),
                ("team_id", request.team_id.map(|v| json!(v))),
                ("world_source", request.world_source.map(|v| json!(v))),
            ]),
        )
    }

    pub fn game_save(&self) -> Reply<game::GameSaved> {
        self.call("game_save", object([]))
    }

    pub fn game_select_team(&self, team_id: &str) -> Reply<game::TeamSelected> {
        self.call(
            "game_select_team",
            object([("team_id", Some(json!(team_id)))]),
        )
    }

    pub fn help_find_tool(&self, query: &str) -> Reply<help::ToolSearch> {
        self.call("help_find_tool", object([("query", Some(json!(query)))]))
    }

    pub fn help_list_categories(&self) -> Reply<help::ToolCategories> {
        self.call("help_list_categories", object([]))
    }

    pub fn inbox_clear_old(&self) -> Reply<inbox::OldMessagesCleared> {
        self.call("inbox_clear_old", object([]))
    }

    pub fn inbox_delete(&self, message_id: &str) -> Reply<inbox::MessageDeleted> {
        self.call(
            "inbox_delete",
            object([("message_id", Some(json!(message_id)))]),
        )
    }

    pub fn inbox_get_messages(
        &self,
        category: Option<&str>,
        unread_only: Option<bool>,
    ) -> Reply<inbox::InboxMessages> {
        self.call(
            "inbox_get_messages",
            object([
                ("category", category.map(|v| json!(v))),
                ("unread_only", unread_only.map(|v| json!(v))),
            ]),
        )
    }

    pub fn inbox_mark_all_read(&self) -> Reply<inbox::AllMessagesMarkedRead> {
        self.call("inbox_mark_all_read", object([]))
    }

    pub fn inbox_mark_read(&self, message_id: &str) -> Reply<inbox::MessageMarkedRead> {
        self.call(
            "inbox_mark_read",
            object([("message_id", Some(json!(message_id)))]),
        )
    }

    pub fn inbox_resolve_action(
        &self,
        action_id: &str,
        message_id: &str,
        option_id: Option<&str>,
    ) -> Reply<inbox::ActionResolved> {
        self.call(
            "inbox_resolve_action",
            object([
                ("action_id", Some(json!(action_id))),
                ("message_id", Some(json!(message_id))),
                ("option_id", option_id.map(|v| json!(v))),
            ]),
        )
    }

    pub fn info_finance_snapshot(
        &self,
        team_id: Option<&str>,
    ) -> Reply<info::DetailedFinancialSnapshot> {
        self.call(
            "info_finance_snapshot",
            object([("team_id", team_id.map(|v| json!(v)))]),
        )
    }

    pub fn info_finances(&self) -> Reply<info::FinancialOverview> {
        self.call("info_finances", object([]))
    }

    pub fn info_fixtures(&self) -> Reply<info::Fixtures> {
        self.call("info_fixtures", object([]))
    }

    pub fn info_game_state(&self) -> Reply<info::GameState> {
        self.call("info_game_state", object([]))
    }

    pub fn info_game_summary(&self) -> Reply<info::GameSummary> {
        self.call("info_game_summary", object([]))
    }

    pub fn info_match_preview(&self) -> Reply<info::MatchPreview> {
        self.call("info_match_preview", object([]))
    }

    pub fn info_news(&self) -> Reply<info::RecentNews> {
        self.call("info_news", object([]))
    }

    pub fn info_player_match_history(
        &self,
        player_id: &str,
        limit: Option<i64>,
    ) -> Reply<info::PlayerMatchHistory> {
        self.call(
            "info_player_match_history",
            object([
                ("player_id", Some(json!(player_id))),
                ("limit", limit.map(|v| json!(v))),
            ]),
        )
    }

    pub fn info_player_profile(&self, player_id: &str) -> Reply<info::PlayerProfile> {
        self.call(
            "info_player_profile",
            object([("player_id", Some(json!(player_id)))]),
        )
    }

    pub fn info_player_stats(&self, player_id: &str) -> Reply<info::PlayerStats> {
        self.call(
            "info_player_stats",
            object([("player_id", Some(json!(player_id)))]),
        )
    }

    pub fn info_season_context(&self) -> Reply<info::SeasonContext> {
        self.call("info_season_context", object([]))
    }

    pub fn info_standings(&self) -> Reply<info::Standings> {
        self.call("info_standings", object([]))
    }

    pub fn info_team_match_history(
        &self,
        team_id: &str,
        limit: Option<i64>,
    ) -> Reply<info::TeamMatchHistory> {
        self.call(
            "info_team_match_history",
            object([
                ("team_id", Some(json!(team_id))),
                ("limit", limit.map(|v| json!(v))),
            ]),
        )
    }

    pub fn info_team_profile(&self, team_id: &str) -> Reply<info::TeamProfile> {
        self.call(
            "info_team_profile",
            object([("team_id", Some(json!(team_id)))]),
        )
    }

    pub fn info_team_stats(&self, team_id: &str) -> Reply<info::TeamStats> {
        self.call(
            "info_team_stats",
            object([("team_id", Some(json!(team_id)))]),
        )
    }

    pub fn jobs_apply(&self, team_id: &str) -> Reply<season::JobApplication> {
        self.call("jobs_apply", object([("team_id", Some(json!(team_id)))]))
    }

    pub fn jobs_available(&self) -> Reply<season::AvailableJobs> {
        self.call("jobs_available", object([]))
    }

    pub fn match_command(&self, command_json: &str) -> Reply<live_match::CommandApplied> {
        self.call(
            "match_command",
            object([("command_json", Some(json!(command_json)))]),
        )
    }

    pub fn match_finish(&self) -> Reply<live_match::MatchFinished> {
        self.call("match_finish", object([]))
    }

    pub fn match_press_conference(
        &self,
        answers_json: &str,
    ) -> Reply<live_match::PressConferenceComplete> {
        self.call(
            "match_press_conference",
            object([("answers_json", Some(json!(answers_json)))]),
        )
    }

    pub fn match_snapshot(&self) -> Reply<live_match::MatchSnapshot> {
        self.call("match_snapshot", object([]))
    }

    pub fn match_start(
        &self,
        mode: &str,
        allows_extra_time: Option<bool>,
        competition_id: Option<&str>,
        fixture_id: Option<&str>,
        fixture_index: Option<i64>,
    ) -> Reply<live_match::LiveMatchStarted> {
        self.call(
            "match_start",
            object([
                ("mode", Some(json!(mode))),
                ("allows_extra_time", allows_extra_time.map(|v| json!(v))),
                ("competition_id", competition_id.map(|v| json!(v))),
                ("fixture_id", fixture_id.map(|v| json!(v))),
                ("fixture_index", fixture_index.map(|v| json!(v))),
            ]),
        )
    }

    pub fn match_step(&self, minutes: i64) -> Reply<live_match::MatchAdvanced> {
        self.call("match_step", object([("minutes", Some(json!(minutes)))]))
    }

    pub fn match_team_talk(&self, context: &str, tone: &str) -> Reply<live_match::TeamTalkApplied> {
        self.call(
            "match_team_talk",
            object([
                ("context", Some(json!(context))),
                ("tone", Some(json!(tone))),
            ]),
        )
    }

    pub fn ping(&self) -> Reply<help::Pong> {
        self.call("ping", object([]))
    }

    pub fn scout_get_reports(&self) -> Reply<scouting::ScoutReports> {
        self.call("scout_get_reports", object([]))
    }

    pub fn scout_send(&self, player_id: &str, scout_id: &str) -> Reply<scouting::ScoutDispatched> {
        self.call(
            "scout_send",
            object([
                ("player_id", Some(json!(player_id))),
                ("scout_id", Some(json!(scout_id))),
            ]),
        )
    }

    pub fn scout_youth_cancel(
        &self,
        assignment_id: &str,
    ) -> Reply<scouting::YouthScoutingCancelled> {
        self.call(
            "scout_youth_cancel",
            object([("assignment_id", Some(json!(assignment_id)))]),
        )
    }

    pub fn scout_youth_reassign(
        &self,
        assignment_id: &str,
        scout_id: &str,
    ) -> Reply<scouting::YouthScoutingReassigned> {
        self.call(
            "scout_youth_reassign",
            object([
                ("assignment_id", Some(json!(assignment_id))),
                ("scout_id", Some(json!(scout_id))),
            ]),
        )
    }

    pub fn scout_youth_start(
        &self,
        scout_id: &str,
        objective: Option<&str>,
        region: Option<&str>,
        target_position: Option<&str>,
    ) -> Reply<scouting::YouthScoutingStarted> {
        self.call(
            "scout_youth_start",
            object([
                ("scout_id", Some(json!(scout_id))),
                ("objective", objective.map(|v| json!(v))),
                ("region", region.map(|v| json!(v))),
                ("target_position", target_position.map(|v| json!(v))),
            ]),
        )
    }

    pub fn season_advance(&self) -> Reply<season::SeasonAdvanced> {
        self.call("season_advance", object([]))
    }

    pub fn season_check_complete(&self) -> Reply<season::SeasonStatus> {
        self.call("season_check_complete", object([]))
    }

    pub fn season_get_awards(&self) -> Reply<season::SeasonAwards> {
        self.call("season_get_awards", object([]))
    }

    pub fn squad_auto_set_pieces(&self) -> Reply<squad::SetPiecesAssigned> {
        self.call("squad_auto_set_pieces", object([]))
    }

    pub fn squad_get(&self) -> Reply<squad::SquadOverview> {
        self.call("squad_get", object([]))
    }

    pub fn squad_set_formation(&self, formation: &str) -> Reply<squad::FormationChanged> {
        self.call(
            "squad_set_formation",
            object([("formation", Some(json!(formation)))]),
        )
    }

    pub fn squad_set_match_roles(
        &self,
        captain: Option<&str>,
        corner_taker: Option<&str>,
        free_kick_taker: Option<&str>,
        penalty_taker: Option<&str>,
        vice_captain: Option<&str>,
    ) -> Reply<squad::MatchRolesUpdated> {
        self.call(
            "squad_set_match_roles",
            object([
                ("captain", captain.map(|v| json!(v))),
                ("corner_taker", corner_taker.map(|v| json!(v))),
                ("free_kick_taker", free_kick_taker.map(|v| json!(v))),
                ("penalty_taker", penalty_taker.map(|v| json!(v))),
                ("vice_captain", vice_captain.map(|v| json!(v))),
            ]),
        )
    }

    pub fn squad_set_play_style(&self, play_style: &str) -> Reply<squad::PlayStyleChanged> {
        self.call(
            "squad_set_play_style",
            object([("play_style", Some(json!(play_style)))]),
        )
    }

    pub fn squad_set_player_role(
        &self,
        player_id: &str,
        squad_role: &str,
    ) -> Reply<squad::PlayerRoleUpdated> {
        self.call(
            "squad_set_player_role",
            object([
                ("player_id", Some(json!(player_id))),
                ("squad_role", Some(json!(squad_role))),
            ]),
        )
    }

    pub fn squad_set_starting_xi(&self, player_ids: &[String]) -> Reply<squad::StartingXiUpdated> {
        self.call(
            "squad_set_starting_xi",
            object([("player_ids", Some(json!(player_ids)))]),
        )
    }

    pub fn staff_get(&self) -> Reply<club::StaffList> {
        self.call("staff_get", object([]))
    }

    pub fn staff_hire(&self, staff_id: &str) -> Reply<club::StaffHired> {
        self.call("staff_hire", object([("staff_id", Some(json!(staff_id)))]))
    }

    pub fn staff_release(&self, staff_id: &str) -> Reply<club::StaffReleased> {
        self.call(
            "staff_release",
            object([("staff_id", Some(json!(staff_id)))]),
        )
    }

    pub fn time_advance(&self) -> Reply<time::DayAdvanced> {
        self.call("time_advance", object([]))
    }

    pub fn time_check_blockers(&self) -> Reply<time::Blockers> {
        self.call("time_check_blockers", object([]))
    }

    pub fn time_skip_to_match_day(&self) -> Reply<time::SkipToMatchDay> {
        self.call("time_skip_to_match_day", object([]))
    }

    pub fn training_get(&self) -> Reply<training::TrainingSettings> {
        self.call("training_get", object([]))
    }

    pub fn training_set_focus_intensity(
        &self,
        focus: &str,
        intensity: &str,
    ) -> Reply<training::TrainingUpdated> {
        self.call(
            "training_set_focus_intensity",
            object([
                ("focus", Some(json!(focus))),
                ("intensity", Some(json!(intensity))),
            ]),
        )
    }

    pub fn training_set_groups(&self, groups_json: &str) -> Reply<training::TrainingGroupsUpdated> {
        self.call(
            "training_set_groups",
            object([("groups_json", Some(json!(groups_json)))]),
        )
    }

    pub fn training_set_player_focus(
        &self,
        player_id: &str,
        focus: Option<&str>,
    ) -> Reply<training::PlayerTrainingFocusUpdated> {
        self.call(
            "training_set_player_focus",
            object([
                ("player_id", Some(json!(player_id))),
                ("focus", focus.map(|v| json!(v))),
            ]),
        )
    }

    pub fn training_set_schedule(
        &self,
        schedule: &str,
    ) -> Reply<training::TrainingScheduleUpdated> {
        self.call(
            "training_set_schedule",
            object([("schedule", Some(json!(schedule)))]),
        )
    }

    pub fn transfer_counter_offer(
        &self,
        offer_id: &str,
        player_id: &str,
        requested_fee: i64,
    ) -> Reply<transfers::CounterOffered> {
        self.call(
            "transfer_counter_offer",
            object([
                ("offer_id", Some(json!(offer_id))),
                ("player_id", Some(json!(player_id))),
                ("requested_fee", Some(json!(requested_fee))),
            ]),
        )
    }

    pub fn transfer_free_agent_offer(
        &self,
        contract_years: i64,
        player_id: &str,
        weekly_wage: i64,
    ) -> Reply<transfers::FreeAgentOffered> {
        self.call(
            "transfer_free_agent_offer",
            object([
                ("contract_years", Some(json!(contract_years))),
                ("player_id", Some(json!(player_id))),
                ("weekly_wage", Some(json!(weekly_wage))),
            ]),
        )
    }

    pub fn transfer_free_agent_preview(
        &self,
        player_id: &str,
        weekly_wage: i64,
    ) -> Reply<transfers::FreeAgentPreview> {
        self.call(
            "transfer_free_agent_preview",
            object([
                ("player_id", Some(json!(player_id))),
                ("weekly_wage", Some(json!(weekly_wage))),
            ]),
        )
    }

    pub fn transfer_make_bid(&self, fee: i64, player_id: &str) -> Reply<transfers::BidMade> {
        self.call(
            "transfer_make_bid",
            object([
                ("fee", Some(json!(fee))),
                ("player_id", Some(json!(player_id))),
            ]),
        )
    }

    pub fn transfer_market_browse(
        &self,
        listed_only: Option<bool>,
        max_price: Option<i64>,
        position: Option<&str>,
    ) -> Reply<transfers::TransferMarket> {
        self.call(
            "transfer_market_browse",
            object([
                ("listed_only", listed_only.map(|v| json!(v))),
                ("max_price", max_price.map(|v| json!(v))),
                ("position", position.map(|v| json!(v))),
            ]),
        )
    }

    pub fn transfer_preview_bid(&self, fee: i64, player_id: &str) -> Reply<transfers::BidPreview> {
        self.call(
            "transfer_preview_bid",
            object([
                ("fee", Some(json!(fee))),
                ("player_id", Some(json!(player_id))),
            ]),
        )
    }

    pub fn transfer_respond_to_offer(
        &self,
        accept: bool,
        offer_id: &str,
        player_id: &str,
    ) -> Reply<transfers::OfferAnswered> {
        self.call(
            "transfer_respond_to_offer",
            object([
                ("accept", Some(json!(accept))),
                ("offer_id", Some(json!(offer_id))),
                ("player_id", Some(json!(player_id))),
            ]),
        )
    }

    pub fn transfer_toggle_listed(
        &self,
        player_id: &str,
    ) -> Reply<transfers::TransferListingToggled> {
        self.call(
            "transfer_toggle_listed",
            object([("player_id", Some(json!(player_id)))]),
        )
    }

    pub fn transfer_toggle_loan(&self, player_id: &str) -> Reply<transfers::LoanListingToggled> {
        self.call(
            "transfer_toggle_loan",
            object([("player_id", Some(json!(player_id)))]),
        )
    }
}
