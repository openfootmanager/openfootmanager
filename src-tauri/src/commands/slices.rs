use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use domain::message::InboxMessage;
use ofm_core::player_knowledge::{
    players_knowledge, potential_opinion, visible_wage, PlayerKnowledge, VisibleWage,
};
use ofm_core::potential_opinion::PotentialOpinion;
use ofm_core::slices::competitions::{query_competitions, CompetitionsQuery, CompetitionsView};
use ofm_core::slices::inbox::{query_messages, MessagesQuery};
use ofm_core::slices::news::{query_news_feed, NewsFeed, NewsFeedQuery};
use ofm_core::slices::players::{query_page, PlayersPage, PlayersPageQuery};
use ofm_core::slices::schedule::{query_schedule, ScheduleQuery, ScheduleSlice};
use ofm_core::slices::scouted_report::{query_scouted_report, ScoutedReportView};
use ofm_core::slices::session::{project_session, SessionState, SessionStateQuery};
use ofm_core::slices::squad::{query_squad, SquadPlayer};
use ofm_core::slices::staff::{query_staff, StaffSlice};
use ofm_core::slices::teams::{
    query_club_league_standing, query_directory, query_team_card, ClubLeagueStanding, TeamCard,
    TeamsDirectory, TeamsDirectoryQuery,
};
use ofm_core::state::StateManager;

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
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
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
            Manager::new(
                "m".into(),
                "M".into(),
                "Gr".into(),
                "1980-01-01".into(),
                "GB".into(),
            ),
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
        assert!(get_squad_internal(&state, "other-club").unwrap().is_empty());
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
    /// Then the first comes as the shape the profile reads (the range, the band, the certainty and
    ///      a tagged source naming the scout and the date), and the other two as null
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
                }
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
}
