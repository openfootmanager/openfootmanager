use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use domain::message::InboxMessage;
use ofm_core::familiarity_projection::{familiarity_projection, FamiliarityProjection};
use ofm_core::manager_standing::{manager_standing, ManagerStanding};
use ofm_core::player_career::{player_career, PlayerCareer};
use ofm_core::player_knowledge::{
    players_knowledge, potential_opinion, visible_wage, youth_prospect_opinions, PlayerKnowledge,
    VisibleWage,
};
use ofm_core::position_ratings::{position_ratings, PositionRatings};
use ofm_core::potential_opinion::PotentialOpinion;
use ofm_core::slices::competitions::{query_competitions, CompetitionsQuery, CompetitionsView};
use ofm_core::slices::inbox::{query_messages, MessagesQuery};
use ofm_core::slices::managers::{query_managers_page, ManagersPage, ManagersStatus};
use ofm_core::slices::news::{query_news_feed, NewsFeed, NewsFeedQuery};
use ofm_core::slices::players::{query_page, PlayersPage, PlayersPageQuery};
use ofm_core::slices::schedule::{query_schedule, ScheduleQuery, ScheduleSlice};
use ofm_core::slices::scouted_report::{query_scouted_report, ScoutedReportView};
use ofm_core::slices::session::{project_session, SessionState, SessionStateQuery};
use ofm_core::slices::squad::{query_squad, SquadPlayer};
use ofm_core::slices::staff::{query_staff, StaffSlice};
use ofm_core::slices::team_selection::{query_team_selection, TeamSelectionRatings};
use ofm_core::slices::teams::{
    query_club_league_standing, query_directory, query_team_card, ClubLeagueStanding, TeamCard,
    TeamsDirectory, TeamsDirectoryQuery,
};
use ofm_core::state::StateManager;
use ofm_core::tactics_lineup::{tactics_lineup, TacticsLineup};

const NO_ACTIVE_GAME: &str = "be.error.noActiveGameSession";

#[tauri::command]
pub async fn get_players_page(
    state: State<'_, Arc<StateManager>>,
    query: PlayersPageQuery,
) -> Result<PlayersPage, String> {
    state
        .get_game(|game| query_page(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_teams_directory(
    state: State<'_, Arc<StateManager>>,
    query: TeamsDirectoryQuery,
) -> Result<TeamsDirectory, String> {
    state
        .get_game(|game| query_directory(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// One club's card, as the clubs directory shows it, for the team profile; null for an id no club
/// has. Its average rating is withheld for a club outside the manager's division.
#[tauri::command]
pub async fn get_team_card(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<Option<TeamCard>, String> {
    get_team_card_internal(&state, &team_id)
}

fn get_team_card_internal(state: &StateManager, team_id: &str) -> Result<Option<TeamCard>, String> {
    state
        .get_game(|game| query_team_card(game, team_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// Every club's squad as the team-selection screen rates it, before a career begins; no club once
/// one has (`slices::team_selection`).
#[tauri::command]
pub async fn get_team_selection_ratings(
    state: State<'_, Arc<StateManager>>,
) -> Result<TeamSelectionRatings, String> {
    get_team_selection_ratings_internal(&state)
}

fn get_team_selection_ratings_internal(
    state: &StateManager,
) -> Result<TeamSelectionRatings, String> {
    state
        .get_game(query_team_selection)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_schedule(
    state: State<'_, Arc<StateManager>>,
    query: ScheduleQuery,
) -> Result<ScheduleSlice, String> {
    state
        .get_game(|game| query_schedule(game, &query))
        .flatten()
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_news_feed(
    state: State<'_, Arc<StateManager>>,
    query: NewsFeedQuery,
) -> Result<NewsFeed, String> {
    state
        .get_game(|game| query_news_feed(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_messages_page(
    state: State<'_, Arc<StateManager>>,
    query: MessagesQuery,
) -> Result<Vec<InboxMessage>, String> {
    state
        .get_game(|game| query_messages(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_competitions_view(
    state: State<'_, Arc<StateManager>>,
    query: CompetitionsQuery,
) -> Result<CompetitionsView, String> {
    state
        .get_game(|game| query_competitions(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_session_state(
    state: State<'_, Arc<StateManager>>,
    _query: SessionStateQuery,
) -> Result<SessionState, String> {
    state
        .get_game(project_session)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_club_league_standing(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<Option<ClubLeagueStanding>, String> {
    state
        .get_game(|game| query_club_league_standing(game, &team_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_scouted_report(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Option<ScoutedReportView>, String> {
    state
        .get_game(|game| query_scouted_report(game, &player_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_player_knowledge(
    state: State<'_, Arc<StateManager>>,
    player_ids: Vec<String>,
) -> Result<HashMap<String, PlayerKnowledge>, String> {
    get_player_knowledge_internal(&state, &player_ids)
}

fn get_player_knowledge_internal(
    state: &StateManager,
    player_ids: &[String],
) -> Result<HashMap<String, PlayerKnowledge>, String> {
    state
        .get_game(|game| players_knowledge(game, player_ids))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// The club's opinion of one player's potential, for his profile: a range, or null when the club
/// holds none. Not a bulk command, on purpose: no list ever carries potential.
#[tauri::command]
pub async fn get_potential_opinion(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Option<PotentialOpinion>, String> {
    get_potential_opinion_internal(&state, &player_id)
}

/// One player's career for his profile: his seasons at each club newest first with every move
/// written where it happened, the same rows summed by club, and his totals; an archived retiree's
/// is his saved seasons; null for an id neither an active player nor a retiree has. The projection is `ofm_core::player_career`: the screen draws it as given.
#[tauri::command]
pub async fn get_player_career(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Option<PlayerCareer>, String> {
    get_player_career_internal(&state, &player_id)
}

fn get_player_career_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<Option<PlayerCareer>, String> {
    state
        .get_game(|game| {
            if let Some(player) = game.players.iter().find(|player| player.id == player_id) {
                return Some(player_career(game, player));
            }
            // An archived retiree keeps his saved seasons: his profile still shows them.
            ofm_core::retirees::find_retiree(game, player_id)
                .map(|retiree| player_career(game, &retiree))
        })
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// What one player is worth at each position he plays, for his profile: his rating, familiarity
/// and key attributes there, or null when the club knows nothing of his attributes.
#[tauri::command]
pub async fn get_position_ratings(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Option<PositionRatings>, String> {
    get_position_ratings_internal(&state, &player_id)
}

/// How each of the manager's club's players stands at every position (natural, can play, out of
/// position), for the squad and tactics screens.
#[tauri::command]
pub async fn get_familiarity(
    state: State<'_, Arc<StateManager>>,
) -> Result<FamiliarityProjection, String> {
    get_familiarity_internal(&state)
}

/// The side the manager's club will field: the eleven slot by slot with each one's role and how
/// he stands there, the bench, and everyone else. The match's own selection, for the Tactics screen.
#[tauri::command]
pub async fn get_tactics_lineup(
    state: State<'_, Arc<StateManager>>,
) -> Result<TacticsLineup, String> {
    get_tactics_lineup_internal(&state)
}

/// The managers directory: every active manager with his club, reputation band and record, and
/// every club with no manager, filtered by `status`.
#[tauri::command]
pub async fn get_managers_page(
    state: State<'_, Arc<StateManager>>,
    status: ManagersStatus,
) -> Result<ManagersPage, String> {
    get_managers_page_internal(&state, status)
}

fn get_managers_page_internal(
    state: &StateManager,
    status: ManagersStatus,
) -> Result<ManagersPage, String> {
    state
        .get_game(|game| query_managers_page(game, status))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

fn get_tactics_lineup_internal(state: &StateManager) -> Result<TacticsLineup, String> {
    state
        .get_game(tactics_lineup)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())?
}

/// How the board and the fans feel about the user's manager, in the words the Manager tab shows
/// (`ofm_core::manager_standing`).
#[tauri::command]
pub async fn get_manager_standing(
    state: State<'_, Arc<StateManager>>,
) -> Result<ManagerStanding, String> {
    get_manager_standing_internal(&state)
}

fn get_manager_standing_internal(state: &StateManager) -> Result<ManagerStanding, String> {
    state
        .get_game(|game| manager_standing(&game.manager))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

fn get_familiarity_internal(state: &StateManager) -> Result<FamiliarityProjection, String> {
    state
        .get_game(familiarity_projection)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())?
}

fn get_position_ratings_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<Option<PositionRatings>, String> {
    state
        .get_game(|game| {
            game.players
                .iter()
                .find(|player| player.id == player_id)
                .and_then(|player| position_ratings(game, player))
        })
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

fn get_potential_opinion_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<Option<PotentialOpinion>, String> {
    state
        .get_game(|game| {
            game.players
                .iter()
                .find(|player| player.id == player_id)
                .and_then(|player| potential_opinion(game, player))
        })
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// One player's weekly wage as the club may see it, for his profile. Lists never ask: it is not a
/// bulk command on purpose.
#[tauri::command]
pub async fn get_visible_wage(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Option<VisibleWage>, String> {
    get_visible_wage_internal(&state, &player_id)
}

fn get_visible_wage_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<Option<VisibleWage>, String> {
    state
        .get_game(|game| {
            game.players
                .iter()
                .find(|player| player.id == player_id)
                .map(|player| visible_wage(game, player))
        })
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_squad(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<Vec<SquadPlayer>, String> {
    get_squad_internal(&state, &team_id)
}

fn get_squad_internal(state: &StateManager, team_id: &str) -> Result<Vec<SquadPlayer>, String> {
    state
        .get_game(|game| query_squad(game, team_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())?
}

#[tauri::command]
pub async fn get_staff(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<StaffSlice, String> {
    state
        .get_game(|game| query_staff(game, &team_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// The reporting scout's opinion of each prospect in one youth scouting report, by prospect id,
/// for the report's screen. Empty for a report that gave none, or an id that names no such report.
#[tauri::command]
pub async fn get_youth_prospect_opinions(
    state: State<'_, Arc<StateManager>>,
    message_id: String,
) -> Result<HashMap<String, PotentialOpinion>, String> {
    get_youth_prospect_opinions_internal(&state, &message_id)
}

fn get_youth_prospect_opinions_internal(
    state: &StateManager,
    message_id: &str,
) -> Result<HashMap<String, PotentialOpinion>, String> {
    state
        .get_game(|game| youth_prospect_opinions(game, message_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given no active game, when the team card adapter reads it,
    /// then the translated session error is returned.
    #[test]
    fn the_team_card_adapter_reports_a_missing_session() {
        assert_eq!(
            get_team_card_internal(&StateManager::new(), "club").unwrap_err(),
            NO_ACTIVE_GAME
        );
    }

    /// Given no active game, when the career adapter reads it, then the translated session error
    /// is returned.
    #[test]
    fn the_career_adapter_reports_a_missing_session() {
        assert_eq!(
            get_player_career_internal(&StateManager::new(), "p").unwrap_err(),
            NO_ACTIVE_GAME
        );
    }

    /// A live session holding one player, "p", with one closed season of 30 appearances, and one
    /// archived retiree, "r", with one of 20.
    fn session_with_a_career_player() -> StateManager {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::player::{CareerEntry, Player, PlayerAttributes, Position};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let mut player = Player::new(
            "p".into(),
            "P".into(),
            "P".into(),
            "2000-01-01".into(),
            "GB".into(),
            Position::Midfielder,
            attrs,
        );
        player.career.push(CareerEntry {
            season: 2025,
            team_id: "north".into(),
            team_name: "North".into(),
            appearances: 30,
            goals: 4,
            assists: 2,
            ..Default::default()
        });
        let manager = Manager::new(
            "m".into(),
            "M".into(),
            "Gr".into(),
            "1980-01-01".into(),
            "GB".into(),
        );
        // An archived retiree, "r": not among the world's players, with a closed season of 20.
        let mut retiree = Player::new(
            "r".into(),
            "R".into(),
            "R".into(),
            "1988-01-01".into(),
            "GB".into(),
            Position::Midfielder,
            player.attributes.clone(),
        );
        retiree.career.push(CareerEntry {
            season: 2030,
            team_id: "porto".into(),
            team_name: "Porto".into(),
            appearances: 20,
            goals: 5,
            assists: 1,
            ..Default::default()
        });
        let state = StateManager::new();
        let mut game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
            ),
            manager,
            vec![],
            vec![player],
            vec![],
            vec![],
        );
        game.retired_players
            .push(domain::retired_player::RetiredPlayer::from_player(
                &retiree,
                "2035-06-30",
                2035,
            ));
        state.set_game(game);
        state
    }

    /// Given a live session with a player who has one closed season
    /// When the career adapter reads him
    /// Then his career carries that season in its totals
    #[test]
    fn the_career_adapter_reads_the_live_players_seasons() {
        let state = session_with_a_career_player();

        let career = get_player_career_internal(&state, "p").unwrap().unwrap();

        assert_eq!(career.totals.appearances, 30);
        assert_eq!(career.totals.goals, 4);
    }

    /// Given an archived retiree (no longer among the world's players) with a closed season
    /// When the career adapter reads him
    /// Then his career is his saved seasons, so a retiree's profile keeps its history, and no
    ///      season is in progress for him
    #[test]
    fn the_career_adapter_reads_an_archived_retirees_seasons() {
        let state = session_with_a_career_player();

        let career = get_player_career_internal(&state, "r").unwrap().unwrap();

        assert_eq!(career.totals.appearances, 20);
        assert_eq!(career.totals.goals, 5);
        assert_eq!(career.by_season.len(), 1);
    }

    /// Given a live session and an id no player has
    /// When the career adapter reads it
    /// Then there is no career (null), not an error
    #[test]
    fn the_career_adapter_knows_no_stranger() {
        let state = session_with_a_career_player();

        assert!(get_player_career_internal(&state, "nobody")
            .unwrap()
            .is_none());
    }

    /// Given no active game, when the tactics lineup adapter reads it,
    /// then the translated session error is returned.
    #[test]
    fn the_tactics_lineup_adapter_reports_a_missing_session() {
        assert_eq!(
            get_tactics_lineup_internal(&StateManager::new()).unwrap_err(),
            NO_ACTIVE_GAME
        );
    }

    /// Given the manager's club, another club he knows nothing about, and an id no club has
    /// When the team card adapter reads each from the live session
    /// Then his club's card carries its average rating as a number, the other's sends `null`
    ///      in its place (the shape the team profile reads), and the unknown id has no card
    #[test]
    fn the_team_card_adapter_withholds_an_unknown_clubs_average() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::player::{Player, PlayerAttributes, Position};
        use domain::team::Team;
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let player_at = |id: &str, club: &str| {
            let mut player = Player::new(
                id.into(),
                id.into(),
                id.into(),
                "2000-01-01".into(),
                "GB".into(),
                Position::Midfielder,
                attrs.clone(),
            );
            player.team_id = Some(club.into());
            player.ovr = 66;
            player
        };
        let team = |id: &str| {
            Team::new(
                id.into(),
                id.into(),
                id.into(),
                "ENG".into(),
                "City".into(),
                "Ground".into(),
                10_000,
            )
        };
        let mut manager = Manager::new(
            "m".into(),
            "M".into(),
            "Gr".into(),
            "1980-01-01".into(),
            "GB".into(),
        );
        manager.team_id = Some("mine".into());
        let state = StateManager::new();
        state.set_game(ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap(),
            ),
            manager,
            vec![team("mine"), team("stranger")],
            vec![player_at("a", "mine"), player_at("b", "stranger")],
            vec![],
            vec![],
        ));

        let card_of = |club: &str| {
            serde_json::to_value(get_team_card_internal(&state, club).unwrap()).unwrap()
        };

        assert_eq!(card_of("mine")["avg_ovr"], 66);
        assert!(card_of("stranger")["avg_ovr"].is_null());
        assert!(get_team_card_internal(&state, "nobody").unwrap().is_none());
    }

    /// Given no active game, when the roster adapter reads it,
    /// then the existing translated session error is returned.
    /// Given no active game, when team selection asks for the squads, then the translated session
    /// error is returned.
    #[test]
    fn the_team_selection_adapter_reports_a_missing_session() {
        assert_eq!(
            get_team_selection_ratings_internal(&StateManager::new()).unwrap_err(),
            NO_ACTIVE_GAME
        );
    }

    #[test]
    fn the_squad_adapter_reports_a_missing_session() {
        assert_eq!(
            get_squad_internal(&StateManager::new(), "club").unwrap_err(),
            NO_ACTIVE_GAME
        );
    }
    /// Given a thin club roster, when the IPC adapter reads the active session,
    /// then its flattened player data carries the backend eligibility verdict.
    #[test]
    fn the_squad_adapter_projects_call_ups_from_the_live_session() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::player::{Player, PlayerAttributes, Position, SquadRole};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let mut keeper = Player::new(
            "keeper".into(),
            "Keeper".into(),
            "Senior Keeper".into(),
            "2000-01-01".into(),
            "GB".into(),
            Position::Goalkeeper,
            attrs.clone(),
        );
        keeper.team_id = Some("club".into());
        let mut youth = Player::new(
            "youth".into(),
            "Youth".into(),
            "Called Youth".into(),
            "2008-01-01".into(),
            "GB".into(),
            Position::Forward,
            attrs,
        );
        youth.team_id = Some("club".into());
        youth.squad_role = SquadRole::Youth;
        let state = StateManager::new();
        state.set_game(ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap(),
            ),
            {
                let mut manager = Manager::new(
                    "m".into(),
                    "M".into(),
                    "Gr".into(),
                    "1980-01-01".into(),
                    "GB".into(),
                );
                manager.hire("club".into());
                manager
            },
            vec![],
            vec![keeper, youth],
            vec![],
            vec![],
        ));
        let value = serde_json::to_value(get_squad_internal(&state, "club").unwrap()).unwrap();
        let rows = value.as_array().unwrap();
        assert_eq!(rows.len(), 2);
        let youth = rows.iter().find(|p| p["id"] == "youth").unwrap();
        assert_eq!(youth["match_day_eligible"], true);
        assert_eq!(youth["squad_role"], "Youth");
        assert_eq!(
            get_squad_internal(&state, "other-club").unwrap_err(),
            ofm_core::slices::squad::ANOTHER_CLUBS_SQUAD,
            "another club's squad is refused, not answered"
        );
    }

    /// Given no active game, when the potential opinion adapter reads it,
    /// then the translated session error is returned.
    #[test]
    fn the_potential_opinion_adapter_reports_a_missing_session() {
        assert_eq!(
            get_potential_opinion_internal(&StateManager::new(), "p").unwrap_err(),
            NO_ACTIVE_GAME
        );
    }

    /// Given a player in the live session whose scout, Ann Scout, wrote 76-84 on 1 Sep, a player
    ///       nobody has judged, and an id no player has
    /// When the potential opinion adapter reads each
    /// Then the first comes as the shape the profile reads (the range, the band, the certainty, a
    ///      tagged source naming the scout and the date, and the wonderkid verdict), and the other
    ///      two as null
    #[test]
    fn the_potential_opinion_adapter_sends_the_opinion_or_null() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::player::{Player, PlayerAttributes, Position};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let rated = |id: &str| {
            let mut player = Player::new(
                id.into(),
                id.into(),
                id.into(),
                "2004-01-01".into(),
                "GB".into(),
                Position::Midfielder,
                attrs.clone(),
            );
            player.team_id = Some("faraway".into());
            player.potential = 80;
            player
        };
        let mut game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
            ),
            Manager::new(
                "m".into(),
                "M".into(),
                "Gr".into(),
                "1980-01-01".into(),
                "GB".into(),
            ),
            vec![],
            vec![rated("scouted"), rated("unjudged")],
            vec![],
            vec![],
        );
        game.scouted_players.push(ofm_core::game::ScoutedPlayer {
            player_id: "scouted".into(),
            scouted_on: "2026-09-01".into(),
            attributes: attrs.clone(),
            potential: Some(ofm_core::game::PotentialRange { low: 76, high: 84 }),
            scout_name: Some("Ann Scout".into()),
        });
        let state = StateManager::new();
        state.set_game(game);

        let opinion =
            serde_json::to_value(get_potential_opinion_internal(&state, "scouted").unwrap())
                .unwrap();

        assert_eq!(
            opinion,
            serde_json::json!({
                "low": 76,
                "high": 84,
                "band": "A",
                "certainty": "fairly_sure",
                "source": {
                    "kind": "scout",
                    "scout_name": "Ann Scout",
                    "scouted_on": "2026-09-01",
                    "current": true
                },
                "wonderkid": false
            })
        );
        assert!(get_potential_opinion_internal(&state, "unjudged")
            .unwrap()
            .is_none());
        assert!(get_potential_opinion_internal(&state, "nobody")
            .unwrap()
            .is_none());
    }

    /// Given no active game, when the knowledge adapter reads it,
    /// then the translated session error is returned.
    #[test]
    fn the_knowledge_adapter_reports_a_missing_session() {
        assert_eq!(
            get_player_knowledge_internal(&StateManager::new(), &["p".to_string()]).unwrap_err(),
            NO_ACTIVE_GAME
        );
    }

    /// Given the manager's own player, a free agent a scout reported on, and an id no player has,
    /// when the IPC adapter is asked about all three,
    /// then the screen receives the exact wire shape it reads: the own player tagged `own`, the
    /// scouted one with `scouted_on` and `current`, and nothing for the stray id.
    #[test]
    fn the_knowledge_adapter_answers_from_the_live_session() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::player::{Player, PlayerAttributes, Position};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let mut own = Player::new(
            "own".into(),
            "Own".into(),
            "Own Player".into(),
            "2000-01-01".into(),
            "GB".into(),
            Position::Midfielder,
            attrs.clone(),
        );
        own.team_id = Some("club".into());
        let free_agent = Player::new(
            "free".into(),
            "Free".into(),
            "Free Agent".into(),
            "2000-01-01".into(),
            "GB".into(),
            Position::Forward,
            attrs.clone(),
        );
        let mut manager = Manager::new(
            "m".into(),
            "M".into(),
            "Gr".into(),
            "1980-01-01".into(),
            "GB".into(),
        );
        manager.team_id = Some("club".into());
        let mut game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap(),
            ),
            manager,
            vec![],
            vec![own, free_agent],
            vec![],
            vec![],
        );
        game.scouted_players.push(ofm_core::game::ScoutedPlayer {
            player_id: "free".into(),
            scouted_on: "2026-07-20".into(),
            attributes: attrs,
            potential: None,
            scout_name: None,
        });
        let state = StateManager::new();
        state.set_game(game);

        let answer =
            get_player_knowledge_internal(&state, &["own".into(), "free".into(), "nobody".into()]);
        let value = serde_json::to_value(answer.unwrap()).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "own": { "kind": "own" },
                "free": { "kind": "scouted", "scouted_on": "2026-07-20", "current": true },
            })
        );
    }

    /// Given the manager's own player on 15,843 a week and a scouted free agent on the same
    /// When the profile asks for each one's wage, and for an id no player has
    /// Then it receives the exact figure, the rounded one, and nothing: the wire shape it reads
    #[test]
    fn the_wage_adapter_answers_exact_rounded_or_nothing() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::player::{Player, PlayerAttributes, Position};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let paid = |id: &str, team: Option<&str>| {
            let mut player = Player::new(
                id.into(),
                id.into(),
                id.into(),
                "2000-01-01".into(),
                "GB".into(),
                Position::Midfielder,
                attrs.clone(),
            );
            player.team_id = team.map(Into::into);
            player.stage_wage(15_843);
            player
        };
        let mut manager = Manager::new(
            "m".into(),
            "M".into(),
            "Gr".into(),
            "1980-01-01".into(),
            "GB".into(),
        );
        manager.team_id = Some("club".into());
        let mut game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap(),
            ),
            manager,
            vec![],
            vec![paid("own", Some("club")), paid("free", None)],
            vec![],
            vec![],
        );
        game.scouted_players.push(ofm_core::game::ScoutedPlayer {
            player_id: "free".into(),
            scouted_on: "2026-07-20".into(),
            attributes: attrs.clone(),
            potential: None,
            scout_name: None,
        });
        let state = StateManager::new();
        state.set_game(game);
        let wage_of = |id: &str| {
            serde_json::to_value(get_visible_wage_internal(&state, id).unwrap()).unwrap()
        };

        assert_eq!(
            wage_of("own"),
            serde_json::json!({ "kind": "exact", "weekly": 15_843 })
        );
        assert_eq!(
            wage_of("free"),
            serde_json::json!({ "kind": "approximate", "weekly": 16_000 })
        );
        assert_eq!(wage_of("nobody"), serde_json::Value::Null);
    }

    /// Given a youth report giving Ann Scout's opinion (76-84, on 1 Sep) of one of its two
    ///       prospects, and an id that names no message
    /// When the youth prospect opinions adapter reads each
    /// Then the report's comes keyed by the judged prospect, in the profile's opinion shape; the
    ///      other prospect and the unknown id give nothing
    #[test]
    fn the_youth_prospect_opinions_adapter_sends_each_opinion_by_prospect() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::message::{MessageContext, ProspectOpinion};
        use domain::player::{Player, PlayerAttributes, Position};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let prospect = |id: &str| {
            let mut player = Player::new(
                id.into(),
                id.into(),
                id.into(),
                "2009-01-01".into(),
                "GB".into(),
                Position::Midfielder,
                attrs.clone(),
            );
            player.potential = 80;
            player
        };
        let mut game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
            ),
            Manager::new(
                "m".into(),
                "M".into(),
                "Gr".into(),
                "1980-01-01".into(),
                "GB".into(),
            ),
            vec![],
            vec![],
            vec![],
            vec![],
        );
        game.messages.push(
            InboxMessage::new(
                "youth-scout-1".into(),
                String::new(),
                String::new(),
                "Ann Scout".into(),
                "2026-09-01".into(),
            )
            .with_context(MessageContext {
                youth_prospects: Some(vec![prospect("judged"), prospect("unjudged")]),
                youth_prospect_opinions: Some(vec![ProspectOpinion {
                    prospect_id: "judged".into(),
                    low: 76,
                    high: 84,
                    scout_name: "Ann Scout".into(),
                    scouted_on: "2026-09-01".into(),
                }]),
                ..MessageContext::default()
            }),
        );
        let state = StateManager::new();
        state.set_game(game);

        let opinions = serde_json::to_value(
            get_youth_prospect_opinions_internal(&state, "youth-scout-1").unwrap(),
        )
        .unwrap();

        assert_eq!(
            opinions,
            serde_json::json!({
                "judged": {
                    "low": 76,
                    "high": 84,
                    "band": "A",
                    "certainty": "fairly_sure",
                    "source": {
                        "kind": "scout",
                        "scout_name": "Ann Scout",
                        "scouted_on": "2026-09-01",
                        "current": true
                    },
                    "wonderkid": false
                }
            })
        );
        assert!(get_youth_prospect_opinions_internal(&state, "nothing")
            .unwrap()
            .is_empty());
    }

    /// Given an unknown id and no world knowledge of anyone
    /// When the position ratings adapter reads it
    /// Then the answer is null, not an error
    #[test]
    fn the_position_ratings_adapter_answers_null_for_a_player_it_cannot_find() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        let game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
            ),
            Manager::new(
                "m".into(),
                "M".into(),
                "Gr".into(),
                "1980-01-01".into(),
                "GB".into(),
            ),
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let state = StateManager::new();
        state.set_game(game);

        assert_eq!(get_position_ratings_internal(&state, "nobody"), Ok(None));
    }

    /// Given no game is loaded, and then a game with only the user's manager
    /// When the managers directory adapter reads it
    /// Then the first is the no-session key and the second lists the user
    #[test]
    fn the_managers_page_adapter_answers_a_key_for_no_session_and_lists_the_user() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        let state = StateManager::new();
        assert_eq!(
            get_managers_page_internal(&state, ManagersStatus::All),
            Err(NO_ACTIVE_GAME.to_string())
        );
        let game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
            ),
            Manager::new(
                "m".into(),
                "M".into(),
                "Gr".into(),
                "1980-01-01".into(),
                "GB".into(),
            ),
            vec![],
            vec![],
            vec![],
            vec![],
        );
        state.set_game(game);

        let page =
            get_managers_page_internal(&state, ManagersStatus::All).expect("a game is loaded");
        let ids: Vec<&str> = page
            .managers
            .iter()
            .map(|manager| manager.id.as_str())
            .collect();
        assert_eq!(ids, vec!["m"]);
    }

    /// Given no game is loaded, and then a game whose manager's board is on 27
    /// When the manager-standing adapter reads it
    /// Then the first is the no-session key, and the second reads the board as concerned (the
    ///      board only warns at 25)
    #[test]
    fn the_manager_standing_adapter_answers_a_key_for_no_session_and_reads_the_board() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use ofm_core::manager_standing::BoardMood;
        let state = StateManager::new();
        assert_eq!(
            get_manager_standing_internal(&state),
            Err(NO_ACTIVE_GAME.to_string())
        );
        let mut manager = Manager::new(
            "m".into(),
            "M".into(),
            "Gr".into(),
            "1980-01-01".into(),
            "GB".into(),
        );
        manager.satisfaction = 27;
        let game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
            ),
            manager,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        state.set_game(game);

        assert_eq!(
            get_manager_standing_internal(&state).map(|standing| standing.board),
            Ok(BoardMood::Concerned)
        );
    }

    /// Given no game is loaded and then a game whose manager has no club
    /// When the familiarity adapter reads it
    /// Then the first is the no-session key and the second the no-team key
    #[test]
    fn the_familiarity_adapter_answers_a_key_for_no_session_and_no_club() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        let state = StateManager::new();
        assert_eq!(
            get_familiarity_internal(&state),
            Err(NO_ACTIVE_GAME.to_string())
        );
        let game = ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap(),
            ),
            Manager::new(
                "m".into(),
                "M".into(),
                "Gr".into(),
                "1980-01-01".into(),
                "GB".into(),
            ),
            vec![],
            vec![],
            vec![],
            vec![],
        );
        state.set_game(game);

        assert_eq!(
            get_familiarity_internal(&state),
            Err("be.error.noTeamAssigned".to_string())
        );
    }
}
