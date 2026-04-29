use crate::clock::GameClock;
use domain::league::League;
use domain::manager::Manager;
use domain::message::InboxMessage;
use domain::news::NewsArticle;
use domain::player::Player;
use domain::season::SeasonContext;
use domain::staff::Staff;
use domain::team::Team;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveType {
    LeaguePosition,
    Wins,
    GoalsScored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardObjective {
    pub id: String,
    pub description: String,
    pub target: u32,
    pub objective_type: ObjectiveType,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutingAssignment {
    pub id: String,
    pub scout_id: String,
    pub player_id: String,
    pub days_remaining: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub clock: GameClock,
    pub manager: Manager,
    pub teams: Vec<Team>,
    pub players: Vec<Player>,
    pub staff: Vec<Staff>,
    pub messages: Vec<InboxMessage>,
    #[serde(default)]
    pub news: Vec<NewsArticle>,
    pub league: Option<League>,
    #[serde(default)]
    pub leagues: Vec<League>,
    #[serde(default)]
    pub scouting_assignments: Vec<ScoutingAssignment>,
    #[serde(default)]
    pub board_objectives: Vec<BoardObjective>,
    #[serde(default)]
    pub season_context: SeasonContext,
    #[serde(default)]
    pub days_since_last_job_offer: Option<u32>,
}

impl Game {
    pub fn new(
        clock: GameClock,
        manager: Manager,
        teams: Vec<Team>,
        players: Vec<Player>,
        staff: Vec<Staff>,
        messages: Vec<InboxMessage>,
    ) -> Self {
        let mut game = Self {
            clock,
            manager,
            teams,
            players,
            staff,
            messages,
            news: vec![],
            league: None,
            leagues: vec![],
            scouting_assignments: vec![],
            board_objectives: vec![],
            season_context: SeasonContext::default(),
            days_since_last_job_offer: None,
        };
        crate::football_identity::upgrade_game_football_identities(&mut game);
        crate::season_context::refresh_game_context(&mut game);
        game
    }
}
