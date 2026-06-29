use crate::clock::GameClock;
use domain::league::{CompetitionType, FixtureStatus, League};
use domain::manager::Manager;
use domain::message::InboxMessage;
use domain::national_team::NationalTeam;
use domain::news::NewsArticle;
use domain::player::{Player, Position};
use domain::season::SeasonContext;
use domain::staff::Staff;
use domain::team::Team;
use domain::world_history::WorldHistoryArchive;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveType {
    LeaguePosition,
    Wins,
    GoalsScored,
    FinancialStability,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum YouthScoutingRegion {
    #[default]
    Domestic,
    International,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum YouthScoutingObjective {
    #[default]
    Balanced,
    HighPotential,
    ReadySoon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouthScoutingAssignment {
    pub id: String,
    pub scout_id: String,
    #[serde(default)]
    pub region: YouthScoutingRegion,
    #[serde(default)]
    pub objective: YouthScoutingObjective,
    pub target_position: Option<Position>,
    pub days_remaining: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub clock: GameClock,
    pub manager: Manager,
    #[serde(default)]
    pub manager_id: String,
    #[serde(default)]
    pub managers: Vec<Manager>,
    pub teams: Vec<Team>,
    pub players: Vec<Player>,
    pub staff: Vec<Staff>,
    pub messages: Vec<InboxMessage>,
    #[serde(default)]
    pub news: Vec<NewsArticle>,
    #[serde(default)]
    pub competitions: Vec<League>,
    #[serde(default)]
    pub national_teams: Vec<NationalTeam>,
    #[serde(default)]
    pub active_region_ids: Vec<String>,
    #[serde(default)]
    pub active_competition_ids: Vec<String>,
    /// DEPRECATED (legacy, back-compat only). Superseded by `competitions`, which
    /// is the single source of truth. Retained for two reasons: loading saves
    /// written before the multi-competition system (see
    /// [`Game::promote_legacy_league`], run on load to populate `competitions`
    /// from it), and as the turn loop's working buffer (`sync_legacy_league`
    /// mirrors the user's competition here). Do not add new readers; it will be
    /// removed once the save-format gate has migrated all saves off it.
    #[serde(default)]
    pub league: Option<League>,
    #[serde(default)]
    pub scouting_assignments: Vec<ScoutingAssignment>,
    #[serde(default)]
    pub youth_scouting_assignments: Vec<YouthScoutingAssignment>,
    #[serde(default)]
    pub board_objectives: Vec<BoardObjective>,
    #[serde(default)]
    pub season_context: SeasonContext,
    #[serde(default)]
    pub days_since_last_job_offer: Option<u32>,
    #[serde(default)]
    pub available_staff_market_last_activity_date: Option<String>,
    #[serde(default)]
    pub vacant_team_days: HashMap<String, u32>,
    #[serde(default)]
    pub world_history: WorldHistoryArchive,
    /// Per-locale translation bundles from the world package (if any), keyed by
    /// locale code. The frontend merges these into the active i18n namespace so
    /// custom competition `name_key` values resolve to package-supplied strings.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub extra_translations: std::collections::HashMap<String, serde_json::Value>,
    /// Records which `.ofm` packages were used to build this save.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_lockfile: Vec<crate::generator::PackageLock>,
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
        let manager_id = manager.id.clone();
        let managers = vec![manager.clone()];
        let mut game = Self {
            clock,
            manager,
            manager_id,
            managers,
            teams,
            players,
            staff,
            messages,
            news: vec![],
            competitions: vec![],
            national_teams: vec![],
            active_region_ids: vec![],
            active_competition_ids: vec![],
            league: None,
            scouting_assignments: vec![],
            youth_scouting_assignments: vec![],
            board_objectives: vec![],
            season_context: SeasonContext::default(),
            days_since_last_job_offer: None,
            available_staff_market_last_activity_date: None,
            vacant_team_days: HashMap::new(),
            world_history: WorldHistoryArchive::default(),
            extra_translations: std::collections::HashMap::new(),
            package_lockfile: vec![],
        };
        game.promote_legacy_league();
        crate::football_identity::upgrade_game_football_identities(&mut game);
        crate::season_context::refresh_game_context(&mut game);
        game
    }

    pub fn sync_user_manager_record(&mut self) {
        let user_manager_id = self.manager_id.clone();
        if let Some(existing) = self
            .managers
            .iter_mut()
            .find(|manager| manager.id == user_manager_id)
        {
            *existing = self.manager.clone();
        } else {
            self.managers.push(self.manager.clone());
        }
    }

    pub fn promote_legacy_league(&mut self) {
        if self.competitions.is_empty() && let Some(league) = self.league.clone() {
            self.competitions.push(league);
        }
        self.sync_legacy_league();
    }

    pub fn sync_legacy_league(&mut self) {
        // The legacy `league` field backs the home dashboard (next match, league
        // position, etc.), so it must mirror the competition the user's club
        // actually plays in — not just the first competition in the world.
        self.league = self
            .user_competition_index()
            .map(|index| self.competitions[index].clone())
            .or_else(|| self.competitions.first().cloned());
    }

    /// Whether the user's club has a scheduled fixture on `date` in ANY of its
    /// competitions (league or cup). This is the source of truth for "is today a
    /// match day" — the legacy `league` mirror misses cups and isn't reliable
    /// while the turn loop swaps competitions through it.
    pub fn user_has_scheduled_match_on(&self, date: &str) -> bool {
        let Some(team_id) = self.manager.team_id.as_deref() else {
            return false;
        };
        self.competitions.iter().any(|competition| {
            competition.fixtures.iter().any(|fixture| {
                fixture.date == date
                    && fixture.status == FixtureStatus::Scheduled
                    && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
            })
        })
    }

    /// Index of the competition the user's club plays in, preferring its
    /// domestic league. `None` when unemployed or no competition lists the club.
    fn user_competition_index(&self) -> Option<usize> {
        let team_id = self.manager.team_id.as_deref()?;
        let contains = |competition: &League| {
            competition
                .standings
                .iter()
                .any(|entry| entry.team_id == team_id)
                || competition.participant_ids.iter().any(|id| id == team_id)
        };
        self.competitions
            .iter()
            .position(|competition| {
                competition.kind == CompetitionType::League && contains(competition)
            })
            .or_else(|| self.competitions.iter().position(contains))
    }

    pub fn primary_competition(&self) -> Option<&League> {
        self.competitions.first().or(self.league.as_ref())
    }

    /// Confederation/region id for a country code. Prefers the world's own
    /// data — a domestic competition's declared region — so data-defined
    /// confederations are respected at runtime, falling back to the built-in
    /// nation catalog for countries the world doesn't place itself.
    pub fn region_for_country(&self, country_code: &str) -> String {
        self.competitions
            .iter()
            .filter(|competition| competition.country_id.as_deref() == Some(country_code))
            .find_map(|competition| competition.region_id.clone())
            .unwrap_or_else(|| crate::nations::region_for_code(country_code).to_string())
    }

    /// Whether a competition falls within the player's active simulation scope.
    /// Empty scope sets mean "everything is active" (the legacy, unscoped game),
    /// so this stays backward compatible with worlds that never set a scope.
    pub fn competition_in_active_scope(&self, competition: &League) -> bool {
        let competition_selected = self.active_competition_ids.is_empty()
            || self.active_competition_ids.contains(&competition.id);
        let region_selected = self.active_region_ids.is_empty()
            || competition
                .region_id
                .as_ref()
                .is_none_or(|region_id| self.active_region_ids.contains(region_id));
        competition_selected && region_selected
    }

    /// The set of team ids participating in an actively-simulated competition,
    /// or `None` when no scope is configured (every team is simulated in full).
    ///
    /// Used to keep expensive daily subsystems (e.g. the transfer market) to the
    /// teams the player actually follows; dormant clubs are handled by lighter,
    /// periodic approximations rather than the full daily pass.
    pub fn active_team_ids(&self) -> Option<HashSet<String>> {
        if self.active_competition_ids.is_empty() && self.active_region_ids.is_empty() {
            return None;
        }
        let mut ids = HashSet::new();
        for competition in &self.competitions {
            if self.competition_in_active_scope(competition) {
                for entry in &competition.standings {
                    ids.insert(entry.team_id.clone());
                }
            }
        }
        // The player's own club is always simulated in full.
        if let Some(team_id) = &self.manager.team_id {
            ids.insert(team_id.clone());
        }
        Some(ids)
    }

    pub fn primary_competition_mut(&mut self) -> Option<&mut League> {
        if self.competitions.is_empty() && let Some(league) = self.league.clone() {
            self.competitions.push(league);
        }
        self.competitions.first_mut()
    }

    pub fn competition_by_id(&self, competition_id: &str) -> Option<&League> {
        self.competitions
            .iter()
            .find(|competition| competition.id == competition_id)
    }

    pub fn competition_by_id_mut(&mut self, competition_id: &str) -> Option<&mut League> {
        self.competitions
            .iter_mut()
            .find(|competition| competition.id == competition_id)
    }
}
