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

    /// A player-OVR-style overall rating (≈30–95) summarising the manager's
    /// standing, track record, and experience. Drives AI squad-management quality
    /// (rotation aggressiveness) and can be surfaced in the UI like a player's OVR.
    ///
    /// Blend: 50% reputation, 30% track record (win rate + trophies), 20%
    /// experience (matches managed). A fresh mid-reputation manager sits near 50;
    /// a decorated, experienced one approaches the high 80s.
    ///
    /// Reputation is normalised against the 300–900 club-reputation domain (the
    /// same scale `management_quality` uses) because AI managers inherit their
    /// club's reputation; a narrower domain would saturate every elite club at
    /// the top and flatten the rotation gradient.
    pub fn rating(&self) -> u8 {
        let reputation = ((f64::from(self.reputation) - 300.0) / 600.0).clamp(0.0, 1.0);
        let experience =
            (f64::from(self.career_stats.matches_managed) / 250.0).clamp(0.0, 1.0);
        let win_rate = (f64::from(self.win_rate()) / 100.0).clamp(0.0, 1.0);
        let trophies = (f64::from(self.career_stats.trophies) / 10.0).clamp(0.0, 1.0);
        let track_record = 0.7 * win_rate + 0.3 * trophies;
        let score = 0.50 * reputation + 0.30 * track_record + 0.20 * experience;
        (30.0 + 65.0 * score).round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> Manager {
        Manager::new(
            "m1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        )
    }

    #[test]
    fn rating_for_fresh_mid_reputation_manager_is_mid_range() {
        // Mid of the 300–900 reputation domain, no career → sits around the middle.
        let mut m = manager();
        m.reputation = 600;
        let r = m.rating();
        assert!((45..=55).contains(&r), "expected mid-range rating, got {r}");
    }

    #[test]
    fn rating_rises_with_reputation_experience_and_success() {
        let mut elite = manager();
        elite.reputation = 800;
        elite.career_stats.matches_managed = 250;
        elite.career_stats.wins = 150; // 60% win rate
        elite.career_stats.trophies = 8;

        let mut journeyman = manager();
        journeyman.reputation = 250;
        journeyman.career_stats.matches_managed = 20;
        journeyman.career_stats.wins = 4; // 20% win rate

        let elite_rating = elite.rating();
        let journeyman_rating = journeyman.rating();

        assert!(
            elite_rating > journeyman_rating,
            "elite ({elite_rating}) should outrate journeyman ({journeyman_rating})"
        );
        assert!(elite_rating >= 80, "decorated manager should be high, got {elite_rating}");
        assert!(
            (30..=99).contains(&elite_rating) && (30..=99).contains(&journeyman_rating),
            "ratings stay in the OVR-like band"
        );
    }
}
