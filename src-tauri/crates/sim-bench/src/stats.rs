use std::collections::HashMap;

use engine::{EventType, MatchReport};
use serde::Serialize;

/// Aggregated statistics across N simulated matches.
#[derive(Default)]
pub struct BenchStats {
    pub games: u32,
    pub home_wins: u32,
    pub draws: u32,
    pub away_wins: u32,

    // Goals
    pub total_goals: u32,
    pub home_goals: u32,
    pub away_goals: u32,
    pub clean_sheets_home: u32,
    pub clean_sheets_away: u32,
    pub btts: u32,

    // Scoreline heatmap: (home_goals, away_goals) → game count, capped at 6 per side
    pub scorelines: HashMap<(u8, u8), u32>,

    // Goals per 15-minute bucket [1-15, 16-30, 31-45, 46-60, 61-75, 76-90, 90+]
    pub goals_by_bucket: [u32; 7],

    // Shooting
    pub total_shots: u64,
    pub shots_on_target: u64,
    pub shots_off_target: u64,
    pub shots_blocked: u64,

    // Penalties
    pub penalties_awarded: u64,
    pub penalty_goals: u64,

    // Passing
    pub passes_completed: u64,
    pub passes_intercepted: u64,

    // Discipline
    pub yellow_cards: u64,
    pub red_cards: u64,
    pub fouls: u64,
    pub injuries: u64,

    // Set pieces
    pub corners: u64,
    pub free_kicks: u64,

    // Tackles & interceptions
    pub tackles: u64,
    pub interceptions: u64,

    // Possession (sum of home % for averaging)
    pub home_possession_sum: f64,

    // Goals-per-game frequency histogram: total_goals_in_game → count_of_games
    pub goals_per_game_hist: HashMap<u8, u32>,

    pub total_time_secs: f64,
}

impl BenchStats {
    pub fn add(&mut self, report: &MatchReport) {
        self.games += 1;

        let hg = report.home_goals;
        let ag = report.away_goals;
        let total_this_game = hg as u32 + ag as u32;

        match hg.cmp(&ag) {
            std::cmp::Ordering::Greater => self.home_wins += 1,
            std::cmp::Ordering::Less => self.away_wins += 1,
            std::cmp::Ordering::Equal => self.draws += 1,
        }

        self.total_goals += total_this_game;
        self.home_goals += hg as u32;
        self.away_goals += ag as u32;

        if ag == 0 {
            self.clean_sheets_home += 1;
        }
        if hg == 0 {
            self.clean_sheets_away += 1;
        }
        if hg > 0 && ag > 0 {
            self.btts += 1;
        }

        *self.scorelines.entry((hg.min(6), ag.min(6))).or_default() += 1;
        *self
            .goals_per_game_hist
            .entry(total_this_game.min(9) as u8)
            .or_default() += 1;

        let hs = &report.home_stats;
        let aw = &report.away_stats;

        self.total_shots += (hs.shots + aw.shots) as u64;
        self.shots_on_target += (hs.shots_on_target + aw.shots_on_target) as u64;
        self.shots_off_target += (hs.shots_off_target + aw.shots_off_target) as u64;
        self.shots_blocked += (hs.shots_blocked + aw.shots_blocked) as u64;
        self.penalties_awarded += (hs.penalties + aw.penalties) as u64;
        self.penalty_goals += report.goals.iter().filter(|g| g.is_penalty).count() as u64;
        self.passes_completed += (hs.passes_completed + aw.passes_completed) as u64;
        self.passes_intercepted += (hs.passes_intercepted + aw.passes_intercepted) as u64;
        self.yellow_cards += (hs.yellow_cards + aw.yellow_cards) as u64;
        self.red_cards += (hs.red_cards + aw.red_cards) as u64;
        self.fouls += (hs.fouls + aw.fouls) as u64;
        self.corners += (hs.corners + aw.corners) as u64;
        self.free_kicks += (hs.free_kicks + aw.free_kicks) as u64;
        self.tackles += (hs.tackles + aw.tackles) as u64;
        self.interceptions += (hs.interceptions + aw.interceptions) as u64;
        self.home_possession_sum += report.home_possession;

        for event in &report.events {
            if event.is_goal() {
                self.goals_by_bucket[goal_bucket(event.minute)] += 1;
            }
            if matches!(event.event_type, EventType::Injury) {
                self.injuries += 1;
            }
        }
    }

    // --- Computed metrics ---

    pub fn gpg(&self) -> f64 {
        self.total_goals as f64 / self.games as f64
    }
    pub fn home_gpg(&self) -> f64 {
        self.home_goals as f64 / self.games as f64
    }
    pub fn away_gpg(&self) -> f64 {
        self.away_goals as f64 / self.games as f64
    }
    pub fn home_win_pct(&self) -> f64 {
        self.home_wins as f64 / self.games as f64 * 100.0
    }
    pub fn draw_pct(&self) -> f64 {
        self.draws as f64 / self.games as f64 * 100.0
    }
    pub fn away_win_pct(&self) -> f64 {
        self.away_wins as f64 / self.games as f64 * 100.0
    }
    pub fn clean_sheet_home_pct(&self) -> f64 {
        self.clean_sheets_home as f64 / self.games as f64 * 100.0
    }
    pub fn clean_sheet_away_pct(&self) -> f64 {
        self.clean_sheets_away as f64 / self.games as f64 * 100.0
    }
    pub fn btts_pct(&self) -> f64 {
        self.btts as f64 / self.games as f64 * 100.0
    }
    pub fn shots_pg(&self) -> f64 {
        self.total_shots as f64 / self.games as f64
    }
    pub fn shot_accuracy_pct(&self) -> f64 {
        if self.total_shots == 0 {
            return 0.0;
        }
        self.shots_on_target as f64 / self.total_shots as f64 * 100.0
    }
    pub fn goal_conversion_pct(&self) -> f64 {
        if self.shots_on_target == 0 {
            return 0.0;
        }
        self.total_goals as f64 / self.shots_on_target as f64 * 100.0
    }
    pub fn xg_proxy_pg(&self, conversion_base: f64) -> f64 {
        (self.shots_on_target as f64 / self.games as f64) * conversion_base
    }
    pub fn yellows_pg(&self) -> f64 {
        self.yellow_cards as f64 / self.games as f64
    }
    pub fn reds_pg(&self) -> f64 {
        self.red_cards as f64 / self.games as f64
    }
    pub fn fouls_pg(&self) -> f64 {
        self.fouls as f64 / self.games as f64
    }
    pub fn corners_pg(&self) -> f64 {
        self.corners as f64 / self.games as f64
    }
    pub fn free_kicks_pg(&self) -> f64 {
        self.free_kicks as f64 / self.games as f64
    }
    pub fn penalties_pg(&self) -> f64 {
        self.penalties_awarded as f64 / self.games as f64
    }
    pub fn penalty_conversion_pct(&self) -> f64 {
        if self.penalties_awarded == 0 {
            return 0.0;
        }
        self.penalty_goals as f64 / self.penalties_awarded as f64 * 100.0
    }
    pub fn injuries_pg(&self) -> f64 {
        self.injuries as f64 / self.games as f64
    }
    pub fn avg_home_possession(&self) -> f64 {
        self.home_possession_sum / self.games as f64
    }
    pub fn pass_accuracy_pct(&self) -> f64 {
        let total = self.passes_completed + self.passes_intercepted;
        if total == 0 {
            return 0.0;
        }
        self.passes_completed as f64 / total as f64 * 100.0
    }
    pub fn games_per_sec(&self) -> f64 {
        self.games as f64 / self.total_time_secs
    }

    /// Top N scorelines sorted by frequency descending.
    pub fn top_scorelines(&self, n: usize) -> Vec<((u8, u8), u32)> {
        let mut list: Vec<_> = self
            .scorelines
            .iter()
            .map(|(&k, &v)| (k, v))
            .collect();
        list.sort_by(|a, b| b.1.cmp(&a.1));
        list.truncate(n);
        list
    }

    /// Serialisable summary for JSON output.
    pub fn to_json(&self, goal_conversion_base: f64) -> JsonSummary {
        JsonSummary {
            games: self.games,
            outcomes: OutcomeJson {
                home_wins: self.home_wins,
                draws: self.draws,
                away_wins: self.away_wins,
                home_win_pct: self.home_win_pct(),
                draw_pct: self.draw_pct(),
                away_win_pct: self.away_win_pct(),
            },
            goals: GoalsJson {
                per_game: self.gpg(),
                home_per_game: self.home_gpg(),
                away_per_game: self.away_gpg(),
                clean_sheet_home_pct: self.clean_sheet_home_pct(),
                clean_sheet_away_pct: self.clean_sheet_away_pct(),
                btts_pct: self.btts_pct(),
            },
            shooting: ShootingJson {
                shots_per_game: self.shots_pg(),
                shots_on_target_pct: self.shot_accuracy_pct(),
                goal_conversion_pct: self.goal_conversion_pct(),
                xg_proxy_per_game: self.xg_proxy_pg(goal_conversion_base),
                goals_vs_xg: self.gpg() - self.xg_proxy_pg(goal_conversion_base),
            },
            discipline: DisciplineJson {
                yellow_cards_per_game: self.yellows_pg(),
                red_cards_per_game: self.reds_pg(),
                fouls_per_game: self.fouls_pg(),
                penalties_per_game: self.penalties_pg(),
                penalty_conversion_pct: self.penalty_conversion_pct(),
                injuries_per_game: self.injuries_pg(),
            },
            set_pieces: SetPiecesJson {
                corners_per_game: self.corners_pg(),
                free_kicks_per_game: self.free_kicks_pg(),
            },
            possession: PossessionJson {
                home_avg_pct: self.avg_home_possession(),
                away_avg_pct: 100.0 - self.avg_home_possession(),
                pass_accuracy_pct: self.pass_accuracy_pct(),
            },
            performance: PerfJson {
                total_time_secs: self.total_time_secs,
                games_per_sec: self.games_per_sec(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct JsonSummary {
    pub games: u32,
    pub outcomes: OutcomeJson,
    pub goals: GoalsJson,
    pub shooting: ShootingJson,
    pub discipline: DisciplineJson,
    pub set_pieces: SetPiecesJson,
    pub possession: PossessionJson,
    pub performance: PerfJson,
}

#[derive(Serialize)]
pub struct OutcomeJson {
    pub home_wins: u32,
    pub draws: u32,
    pub away_wins: u32,
    pub home_win_pct: f64,
    pub draw_pct: f64,
    pub away_win_pct: f64,
}

#[derive(Serialize)]
pub struct GoalsJson {
    pub per_game: f64,
    pub home_per_game: f64,
    pub away_per_game: f64,
    pub clean_sheet_home_pct: f64,
    pub clean_sheet_away_pct: f64,
    pub btts_pct: f64,
}

#[derive(Serialize)]
pub struct ShootingJson {
    pub shots_per_game: f64,
    pub shots_on_target_pct: f64,
    pub goal_conversion_pct: f64,
    pub xg_proxy_per_game: f64,
    pub goals_vs_xg: f64,
}

#[derive(Serialize)]
pub struct DisciplineJson {
    pub yellow_cards_per_game: f64,
    pub red_cards_per_game: f64,
    pub fouls_per_game: f64,
    pub penalties_per_game: f64,
    pub penalty_conversion_pct: f64,
    pub injuries_per_game: f64,
}

#[derive(Serialize)]
pub struct SetPiecesJson {
    pub corners_per_game: f64,
    pub free_kicks_per_game: f64,
}

#[derive(Serialize)]
pub struct PossessionJson {
    pub home_avg_pct: f64,
    pub away_avg_pct: f64,
    pub pass_accuracy_pct: f64,
}

#[derive(Serialize)]
pub struct PerfJson {
    pub total_time_secs: f64,
    pub games_per_sec: f64,
}

fn goal_bucket(minute: u8) -> usize {
    match minute {
        1..=15 => 0,
        16..=30 => 1,
        31..=45 => 2,
        46..=60 => 3,
        61..=75 => 4,
        76..=90 => 5,
        _ => 6,
    }
}
