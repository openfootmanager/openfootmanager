use serde::{Deserialize, Serialize};

fn default_fan_approval() -> u8 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manager {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub nationality: String,
    #[serde(default)]
    pub football_nation: String,
    #[serde(default)]
    pub birth_country: Option<String>,
    pub reputation: u32,
    pub satisfaction: u8, // 0 to 100
    #[serde(default = "default_fan_approval")]
    pub fan_approval: u8, // 0 to 100 — fan sentiment
    pub team_id: Option<String>,

    // Board warning stage at current club: 0 = none, 1 = warning, 2 = final warning.
    // Reset to 0 on hire so warnings don't carry over between clubs.
    #[serde(default)]
    pub warning_stage: u8,

    // Career stats (cumulative)
    pub career_stats: ManagerCareerStats,

    // Employment history
    pub career_history: Vec<ManagerCareerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManagerCareerStats {
    pub matches_managed: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub trophies: u32,
    pub best_finish: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerCareerEntry {
    pub team_id: String,
    pub team_name: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub matches: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub best_league_position: Option<u32>,
}

impl ManagerCareerEntry {
    pub fn open(team_id: String, team_name: String, start_date: String) -> Self {
        Self {
            team_id,
            team_name,
            start_date,
            end_date: None,
            matches: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            best_league_position: None,
        }
    }
}

impl Manager {
    pub fn new(
        id: String,
        first_name: String,
        last_name: String,
        date_of_birth: String,
        nationality: String,
    ) -> Self {
        let football_nation = crate::identity::normalize_football_nation_code(&nationality);
        let birth_country = crate::identity::derive_birth_country_code(&nationality);
        Self {
            id,
            first_name,
            last_name,
            date_of_birth,
            nationality,
            football_nation,
            birth_country,
            reputation: 500,
            satisfaction: 100,
            fan_approval: 50,
            team_id: None,
            warning_stage: 0,
            career_stats: ManagerCareerStats::default(),
            career_history: Vec::new(),
        }
    }

    pub fn hire(&mut self, team_id: String) {
        self.team_id = Some(team_id);
        self.warning_stage = 0;
    }

    pub fn fire(&mut self, date: &str) {
        if let Some(entry) = self
            .career_history
            .iter_mut()
            .find(|e| e.end_date.is_none())
        {
            entry.end_date = Some(date.to_string());
        }
        self.team_id = None;
        self.warning_stage = 0;
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn win_rate(&self) -> f32 {
        if self.career_stats.matches_managed == 0 {
            return 0.0;
        }
        self.career_stats.wins as f32 / self.career_stats.matches_managed as f32 * 100.0
    }
}
