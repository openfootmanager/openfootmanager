use std::collections::HashMap;
use std::time::Instant;

use engine::{
    simulate_with_rng, EventType, GoalSource, MatchConfig, MatchReport, PlayStyle, PlayerData,
    PlayerRole, Position, TeamData,
};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public DTOs (Tauri serialises these to JSON for the frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SimBatchConfig {
    pub games: u32,
    pub seed: Option<u64>,

    pub home_style: PlayStyleDto,
    pub away_style: PlayStyleDto,
    pub home_formation: String,
    pub away_formation: String,
    pub home_rating: u8,
    pub away_rating: u8,

    // MatchConfig overrides (all optional)
    pub home_advantage: Option<f64>,
    pub shot_accuracy_base: Option<f64>,
    pub goal_conversion_base: Option<f64>,
    pub foul_probability: Option<f64>,
    pub yellow_card_probability: Option<f64>,
    pub red_card_probability: Option<f64>,
    pub penalty_probability: Option<f64>,
    pub injury_probability: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayStyleDto {
    Balanced,
    Attacking,
    Defensive,
    Possession,
    Counter,
    HighPress,
}

impl From<PlayStyleDto> for PlayStyle {
    fn from(dto: PlayStyleDto) -> Self {
        match dto {
            PlayStyleDto::Balanced => PlayStyle::Balanced,
            PlayStyleDto::Attacking => PlayStyle::Attacking,
            PlayStyleDto::Defensive => PlayStyle::Defensive,
            PlayStyleDto::Possession => PlayStyle::Possession,
            PlayStyleDto::Counter => PlayStyle::Counter,
            PlayStyleDto::HighPress => PlayStyle::HighPress,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SimBatchResults {
    pub games: u32,

    // Outcomes
    pub home_wins: u32,
    pub draws: u32,
    pub away_wins: u32,
    pub home_win_pct: f64,
    pub draw_pct: f64,
    pub away_win_pct: f64,

    // Goals
    pub goals_per_game: f64,
    pub home_goals_per_game: f64,
    pub away_goals_per_game: f64,
    pub clean_sheet_home_pct: f64,
    pub clean_sheet_away_pct: f64,
    pub btts_pct: f64,

    // Shooting
    pub shots_per_game: f64,
    pub shots_on_target_pct: f64,
    pub goal_conversion_pct: f64,
    pub xg_proxy_per_game: f64,

    // Discipline
    pub yellow_cards_per_game: f64,
    pub red_cards_per_game: f64,
    pub fouls_per_game: f64,
    pub penalties_per_game: f64,
    pub penalty_conversion_pct: f64,
    pub injuries_per_game: f64,

    // Set pieces
    pub corners_per_game: f64,
    pub free_kicks_per_game: f64,

    // Possession
    pub home_possession_avg: f64,
    pub away_possession_avg: f64,
    pub passes_per_game: f64,

    // Distributions (for charts)
    /// goals_per_game_hist[n] = fraction of games with exactly n goals (n capped at 9)
    pub goals_per_game_hist: Vec<f64>,
    /// scoreline_heatmap[home_goals][away_goals] = fraction of games (6×6, goals capped at 5)
    pub scoreline_heatmap: Vec<Vec<f64>>,
    /// goals_by_bucket[0..6] = fraction of all goals per 15-min bucket
    pub goals_by_bucket: Vec<f64>,

    // Performance
    pub total_time_secs: f64,
    pub games_per_sec: f64,
}

#[derive(Debug, Serialize)]
pub struct SingleMatchResult {
    pub home_goals: u8,
    pub away_goals: u8,
    pub home_possession: f64,
    pub total_minutes: u8,
    pub home_shots: u16,
    pub away_shots: u16,
    pub home_yellow_cards: u8,
    pub away_yellow_cards: u8,
    pub home_red_cards: u8,
    pub away_red_cards: u8,
    pub home_corners: u16,
    pub away_corners: u16,
    pub home_fouls: u16,
    pub away_fouls: u16,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

fn validate_probability(name: &str, value: f64) -> Result<(), String> {
    if !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} must be between 0.0 and 1.0, got {value}"));
    }
    Ok(())
}

/// Run a batch of headless simulations and return aggregate statistics.
#[tauri::command]
pub fn run_sim_batch(config: SimBatchConfig) -> Result<SimBatchResults, String> {
    if config.games == 0 || config.games > 100_000 {
        return Err(format!(
            "games must be between 1 and 100000, got {}",
            config.games
        ));
    }
    if let Some(v) = config.shot_accuracy_base { validate_probability("shot_accuracy_base", v)?; }
    if let Some(v) = config.goal_conversion_base { validate_probability("goal_conversion_base", v)?; }
    if let Some(v) = config.foul_probability { validate_probability("foul_probability", v)?; }
    if let Some(v) = config.yellow_card_probability { validate_probability("yellow_card_probability", v)?; }
    if let Some(v) = config.red_card_probability { validate_probability("red_card_probability", v)?; }
    if let Some(v) = config.penalty_probability { validate_probability("penalty_probability", v)?; }
    if let Some(v) = config.injury_probability { validate_probability("injury_probability", v)?; }

    let match_config = build_match_config(&config);
    let base_seed = config.seed.unwrap_or_else(system_seed);

    // Build both teams with a deterministic seed (so the same config always produces the same teams)
    let mut team_rng = StdRng::seed_from_u64(base_seed.wrapping_add(0xDEAD_BEEF));
    let home = build_team(
        "home",
        config.home_rating,
        PlayStyle::from(config.home_style),
        &config.home_formation,
        &mut team_rng,
    );
    let away = build_team(
        "away",
        config.away_rating,
        PlayStyle::from(config.away_style),
        &config.away_formation,
        &mut team_rng,
    );

    let mut agg = Aggregator::default();
    let start = Instant::now();

    for i in 0..config.games {
        let game_seed = base_seed.wrapping_add(i as u64);
        let mut rng = StdRng::seed_from_u64(game_seed);
        let report = simulate_with_rng(&home, &away, &match_config, &mut rng);
        agg.add(&report);
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok(agg.into_results(config.games, elapsed, match_config.goal_conversion_base))
}

/// Run a single seeded match and return basic result stats.
#[tauri::command]
pub fn run_single_seeded_match(
    seed: u64,
    home_style: PlayStyleDto,
    away_style: PlayStyleDto,
    home_formation: String,
    away_formation: String,
    home_rating: u8,
    away_rating: u8,
) -> Result<SingleMatchResult, String> {
    let match_config = MatchConfig::default();
    let mut team_rng = StdRng::seed_from_u64(seed.wrapping_add(0xDEAD_BEEF));

    let home = build_team(
        "home",
        home_rating,
        PlayStyle::from(home_style),
        &home_formation,
        &mut team_rng,
    );
    let away = build_team(
        "away",
        away_rating,
        PlayStyle::from(away_style),
        &away_formation,
        &mut team_rng,
    );

    let mut rng = StdRng::seed_from_u64(seed);
    let r = simulate_with_rng(&home, &away, &match_config, &mut rng);

    Ok(SingleMatchResult {
        home_goals: r.home_goals,
        away_goals: r.away_goals,
        home_possession: r.home_possession,
        total_minutes: r.total_minutes,
        home_shots: r.home_stats.shots,
        away_shots: r.away_stats.shots,
        home_yellow_cards: r.home_stats.yellow_cards,
        away_yellow_cards: r.away_stats.yellow_cards,
        home_red_cards: r.home_stats.red_cards,
        away_red_cards: r.away_stats.red_cards,
        home_corners: r.home_stats.corners,
        away_corners: r.away_stats.corners,
        home_fouls: r.home_stats.fouls,
        away_fouls: r.away_stats.fouls,
    })
}

// ---------------------------------------------------------------------------
// Internal aggregation
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Aggregator {
    home_wins: u32,
    draws: u32,
    away_wins: u32,
    total_goals: u32,
    home_goals: u32,
    away_goals: u32,
    clean_sheets_home: u32,
    clean_sheets_away: u32,
    btts: u32,
    scorelines: HashMap<(u8, u8), u32>,
    goals_per_game_hist: HashMap<u8, u32>,
    goals_by_bucket: [u32; 7],
    total_shots: u64,
    shots_on_target: u64,
    penalties_awarded: u64,
    penalty_goals: u64,
    passes_completed: u64,
    yellow_cards: u64,
    red_cards: u64,
    fouls: u64,
    injuries: u64,
    corners: u64,
    free_kicks: u64,
    home_possession_sum: f64,
}

impl Aggregator {
    fn add(&mut self, r: &MatchReport) {
        let hg = r.home_goals;
        let ag = r.away_goals;
        let total = hg as u32 + ag as u32;

        match hg.cmp(&ag) {
            std::cmp::Ordering::Greater => self.home_wins += 1,
            std::cmp::Ordering::Less => self.away_wins += 1,
            std::cmp::Ordering::Equal => self.draws += 1,
        }
        self.total_goals += total;
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

        *self.scorelines.entry((hg.min(5), ag.min(5))).or_default() += 1;
        *self
            .goals_per_game_hist
            .entry(total.min(9) as u8)
            .or_default() += 1;

        let hs = &r.home_stats;
        let aw = &r.away_stats;
        self.total_shots += (hs.shots + aw.shots) as u64;
        self.shots_on_target += (hs.shots_on_target + aw.shots_on_target) as u64;
        self.penalties_awarded += r
            .events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::PenaltyAwarded))
            .count() as u64;
        self.penalty_goals += r
            .goals
            .iter()
            .filter(|g| g.goal_source == GoalSource::Penalty)
            .count() as u64;
        self.passes_completed += (hs.passes_completed + aw.passes_completed) as u64;
        self.yellow_cards += (hs.yellow_cards + aw.yellow_cards) as u64;
        self.red_cards += (hs.red_cards + aw.red_cards) as u64;
        self.fouls += (hs.fouls + aw.fouls) as u64;
        self.corners += (hs.corners + aw.corners) as u64;
        self.free_kicks += (hs.free_kicks + aw.free_kicks) as u64;
        self.home_possession_sum += r.home_possession;

        for event in &r.events {
            if event.is_goal() {
                self.goals_by_bucket[goal_bucket(event.minute)] += 1;
            }
            if matches!(event.event_type, EventType::Injury) {
                self.injuries += 1;
            }
        }
    }

    fn into_results(self, games: u32, elapsed: f64, conversion_base: f64) -> SimBatchResults {
        let n = games as f64;

        let gpg = self.total_goals as f64 / n;
        let shot_acc = if self.total_shots == 0 {
            0.0
        } else {
            self.shots_on_target as f64 / self.total_shots as f64 * 100.0
        };
        let conversion = if self.shots_on_target == 0 {
            0.0
        } else {
            self.total_goals as f64 / self.shots_on_target as f64 * 100.0
        };
        let xg_proxy = (self.shots_on_target as f64 / n) * conversion_base;
        let pen_conv = if self.penalties_awarded == 0 {
            0.0
        } else {
            self.penalty_goals as f64 / self.penalties_awarded as f64 * 100.0
        };

        // Histogram as fractions
        let hist: Vec<f64> = (0u8..=9)
            .map(|g| {
                self.goals_per_game_hist
                    .get(&g)
                    .copied()
                    .unwrap_or(0) as f64
                    / n
            })
            .collect();

        // Scoreline heatmap [0..=5][0..=5] as fractions
        let heatmap: Vec<Vec<f64>> = (0u8..=5)
            .map(|hg| {
                (0u8..=5)
                    .map(|ag| {
                        self.scorelines.get(&(hg, ag)).copied().unwrap_or(0) as f64 / n
                    })
                    .collect()
            })
            .collect();

        // Goals by bucket as fractions of total goals
        let bucket_fracs: Vec<f64> = self
            .goals_by_bucket
            .iter()
            .map(|&c| {
                if self.total_goals == 0 {
                    0.0
                } else {
                    c as f64 / self.total_goals as f64
                }
            })
            .collect();

        let home_poss = self.home_possession_sum / n;

        SimBatchResults {
            games,
            home_wins: self.home_wins,
            draws: self.draws,
            away_wins: self.away_wins,
            home_win_pct: self.home_wins as f64 / n * 100.0,
            draw_pct: self.draws as f64 / n * 100.0,
            away_win_pct: self.away_wins as f64 / n * 100.0,
            goals_per_game: gpg,
            home_goals_per_game: self.home_goals as f64 / n,
            away_goals_per_game: self.away_goals as f64 / n,
            clean_sheet_home_pct: self.clean_sheets_home as f64 / n * 100.0,
            clean_sheet_away_pct: self.clean_sheets_away as f64 / n * 100.0,
            btts_pct: self.btts as f64 / n * 100.0,
            shots_per_game: self.total_shots as f64 / n,
            shots_on_target_pct: shot_acc,
            goal_conversion_pct: conversion,
            xg_proxy_per_game: xg_proxy,
            yellow_cards_per_game: self.yellow_cards as f64 / n,
            red_cards_per_game: self.red_cards as f64 / n,
            fouls_per_game: self.fouls as f64 / n,
            penalties_per_game: self.penalties_awarded as f64 / n,
            penalty_conversion_pct: pen_conv,
            injuries_per_game: self.injuries as f64 / n,
            corners_per_game: self.corners as f64 / n,
            free_kicks_per_game: self.free_kicks as f64 / n,
            home_possession_avg: home_poss,
            away_possession_avg: 100.0 - home_poss,
            passes_per_game: self.passes_completed as f64 / n,
            goals_per_game_hist: hist,
            scoreline_heatmap: heatmap,
            goals_by_bucket: bucket_fracs,
            total_time_secs: elapsed,
            games_per_sec: games as f64 / elapsed,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_match_config(cfg: &SimBatchConfig) -> MatchConfig {
    let mut mc = MatchConfig::default();
    if let Some(v) = cfg.home_advantage {
        mc.home_advantage = v;
    }
    if let Some(v) = cfg.shot_accuracy_base {
        mc.shot_accuracy_base = v;
    }
    if let Some(v) = cfg.goal_conversion_base {
        mc.goal_conversion_base = v;
    }
    if let Some(v) = cfg.foul_probability {
        mc.foul_probability = v;
    }
    if let Some(v) = cfg.yellow_card_probability {
        mc.yellow_card_probability = v;
    }
    if let Some(v) = cfg.red_card_probability {
        mc.red_card_probability = v;
    }
    if let Some(v) = cfg.penalty_probability {
        mc.penalty_probability = v;
    }
    if let Some(v) = cfg.injury_probability {
        mc.injury_probability = v;
    }
    mc
}

fn system_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
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

fn build_team(id: &str, avg_ovr: u8, play_style: PlayStyle, formation: &str, rng: &mut StdRng) -> TeamData {
    let (n_def, n_mid, n_fwd) = parse_formation(formation);
    let mut players = Vec::with_capacity(11);

    players.push(make_player(id, "GK", 1, Position::Goalkeeper, avg_ovr, rng));
    for i in 1..=n_def {
        players.push(make_player(id, "DEF", i, Position::Defender, avg_ovr, rng));
    }
    for i in 1..=n_mid {
        players.push(make_player(id, "MID", i, Position::Midfielder, avg_ovr, rng));
    }
    for i in 1..=n_fwd {
        players.push(make_player(id, "FWD", i, Position::Forward, avg_ovr, rng));
    }

    TeamData {
        id: id.to_string(),
        name: format!("{} FC", id),
        formation: formation.to_string(),
        play_style,
        players,
        tactics: engine::TacticsConfig::default(),
    }
}

fn parse_formation(formation: &str) -> (u8, u8, u8) {
    let parts: Vec<u8> = formation
        .split('-')
        .filter_map(|s| s.parse::<u8>().ok())
        .collect();
    let result = match parts.len() {
        2 => (parts[0], 0, parts[1]),
        3 => (parts[0], parts[1], parts[2]),
        4 => (parts[0], parts[1] + parts[2], parts[3]),
        _ => (4, 4, 2),
    };
    // Ensure exactly 10 outfield players; fall back to 4-4-2 if not
    if result.0 + result.1 + result.2 != 10 {
        return (4, 4, 2);
    }
    result
}

fn make_player(
    team_id: &str,
    pos_label: &str,
    idx: u8,
    position: Position,
    avg_ovr: u8,
    rng: &mut StdRng,
) -> PlayerData {
    fn noise(base: f64, rng: &mut StdRng) -> u8 {
        (base + rng.random_range(-10.0f64..10.0f64)).clamp(10.0, 99.0) as u8
    }
    fn biased(base: f64, offset: f64, rng: &mut StdRng) -> u8 {
        (base + offset + rng.random_range(-8.0f64..8.0f64)).clamp(10.0, 99.0) as u8
    }

    let base = avg_ovr as f64;
    let (shoot_off, tackle_off, pass_off, defend_off, gk_off) = match position {
        Position::Goalkeeper => (-25.0, 0.0, 0.0, 10.0, 20.0),
        Position::Defender => (-18.0, 12.0, -5.0, 18.0, -15.0),
        Position::Midfielder => (-3.0, 5.0, 12.0, 0.0, -15.0),
        Position::Forward => (18.0, -12.0, 3.0, -18.0, -20.0),
    };

    PlayerData {
        id: format!("{team_id}_{pos_label}{idx}"),
        name: format!("{pos_label}{idx}"),
        position,
        ovr: avg_ovr,
        condition: rng.random_range(80u8..=100u8),
        fitness: rng.random_range(65u8..=90u8),
        pace: noise(base, rng),
        stamina: noise(base, rng),
        strength: noise(base, rng),
        agility: noise(base, rng),
        passing: biased(base, pass_off, rng),
        shooting: biased(base, shoot_off, rng),
        tackling: biased(base, tackle_off, rng),
        dribbling: noise(base, rng),
        defending: biased(base, defend_off, rng),
        positioning: noise(base, rng),
        vision: biased(base, pass_off / 2.0, rng),
        decisions: noise(base, rng),
        composure: noise(base, rng),
        aggression: noise(base, rng),
        teamwork: noise(base, rng),
        leadership: noise(base, rng),
        handling: biased(base, gk_off, rng),
        reflexes: biased(base, gk_off, rng),
        aerial: noise(base, rng),
        traits: vec![],
        role: PlayerRole::default(),
    }
}
