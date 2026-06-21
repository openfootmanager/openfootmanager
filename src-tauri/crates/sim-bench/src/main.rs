mod builder;
mod html;
mod stats;
mod terminal;

use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use colored::Colorize;
use engine::{MatchConfig, PlayStyle, simulate_with_rng};
use rand::SeedableRng;
use rand::rngs::StdRng;

use builder::build_team;
use stats::BenchStats;

#[derive(Parser)]
#[command(
    name = "ofm-sim-bench",
    about = "OpenFoot Manager — match simulation benchmarking & analysis tool"
)]
struct Cli {
    /// Number of games to simulate
    #[arg(short = 'n', long, default_value_t = 1000)]
    games: u32,

    /// RNG seed for reproducible runs (each game gets seed+i)
    #[arg(long)]
    seed: Option<u64>,

    /// Home team play style
    #[arg(long, value_enum, default_value_t = StyleArg::Balanced)]
    home_style: StyleArg,

    /// Away team play style
    #[arg(long, value_enum, default_value_t = StyleArg::Balanced)]
    away_style: StyleArg,

    /// Home team formation (e.g. 4-3-3)
    #[arg(long, default_value = "4-4-2")]
    home_formation: String,

    /// Away team formation
    #[arg(long, default_value = "4-4-2")]
    away_formation: String,

    /// Home team average overall rating (10–99)
    #[arg(long, default_value_t = 70)]
    home_rating: u8,

    /// Away team average overall rating (10–99)
    #[arg(long, default_value_t = 70)]
    away_rating: u8,

    /// Print a rich colour terminal report (default: JSON to stdout)
    #[arg(long)]
    verbose: bool,

    /// Write a self-contained HTML report to this path
    #[arg(long)]
    html: Option<PathBuf>,

    /// Write JSON output to this file (only useful with --verbose)
    #[arg(long)]
    out: Option<PathBuf>,

    /// Benchmark mode: time the engine, skip stat collection
    #[arg(long)]
    bench: bool,

    // ── MatchConfig overrides ────────────────────────────────────────────────
    #[arg(long, help = "Home advantage multiplier (default 1.08)")]
    home_advantage: Option<f64>,

    #[arg(long, help = "Base shot-on-target probability (default 0.45)")]
    shot_accuracy_base: Option<f64>,

    #[arg(long, help = "Base goal conversion probability (default 0.30)")]
    goal_conversion_base: Option<f64>,

    #[arg(long, help = "Per-action foul probability (default 0.12)")]
    foul_probability: Option<f64>,

    #[arg(long, help = "Yellow card probability per foul (default 0.30)")]
    yellow_card_probability: Option<f64>,

    #[arg(long, help = "Direct red / escalation probability (default 0.04)")]
    red_card_probability: Option<f64>,

    #[arg(long, help = "Penalty probability for box foul (default 0.08)")]
    penalty_probability: Option<f64>,

    #[arg(long, help = "Injury probability per foul (default 0.03)")]
    injury_probability: Option<f64>,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
enum StyleArg {
    Balanced,
    Attacking,
    Defensive,
    Possession,
    Counter,
    #[value(name = "high-press")]
    HighPress,
}

impl StyleArg {
    fn to_play_style(self) -> PlayStyle {
        match self {
            StyleArg::Balanced => PlayStyle::Balanced,
            StyleArg::Attacking => PlayStyle::Attacking,
            StyleArg::Defensive => PlayStyle::Defensive,
            StyleArg::Possession => PlayStyle::Possession,
            StyleArg::Counter => PlayStyle::Counter,
            StyleArg::HighPress => PlayStyle::HighPress,
        }
    }

    fn label(self) -> &'static str {
        match self {
            StyleArg::Balanced => "Balanced",
            StyleArg::Attacking => "Attacking",
            StyleArg::Defensive => "Defensive",
            StyleArg::Possession => "Possession",
            StyleArg::Counter => "Counter",
            StyleArg::HighPress => "High Press",
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.games == 0 {
        eprintln!("error: --games must be at least 1");
        std::process::exit(1);
    }

    let mut config = MatchConfig::default();
    if let Some(v) = cli.home_advantage {
        config.home_advantage = v;
    }
    if let Some(v) = cli.shot_accuracy_base {
        config.shot_accuracy_base = v;
    }
    if let Some(v) = cli.goal_conversion_base {
        config.goal_conversion_base = v;
    }
    if let Some(v) = cli.foul_probability {
        config.foul_probability = v;
    }
    if let Some(v) = cli.yellow_card_probability {
        config.yellow_card_probability = v;
    }
    if let Some(v) = cli.red_card_probability {
        config.red_card_probability = v;
    }
    if let Some(v) = cli.penalty_probability {
        config.penalty_probability = v;
    }
    if let Some(v) = cli.injury_probability {
        config.injury_probability = v;
    }

    if cli.bench {
        run_bench(&config, cli.games, cli.seed);
        return;
    }

    // Derive a base seed: explicit or from system clock
    let base_seed = cli.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    });

    // Build teams with a fixed team-builder seed (separate from per-game seeds)
    let mut team_rng = StdRng::seed_from_u64(base_seed.wrapping_add(0xDEAD_BEEF));
    let home = build_team(
        "home",
        "Home FC",
        cli.home_rating,
        cli.home_style.to_play_style(),
        &cli.home_formation,
        &mut team_rng,
    );
    let away = build_team(
        "away",
        "Away FC",
        cli.away_rating,
        cli.away_style.to_play_style(),
        &cli.away_formation,
        &mut team_rng,
    );

    eprintln!(
        "Simulating {} games (seed: {})…",
        cli.games,
        if cli.seed.is_some() {
            base_seed.to_string()
        } else {
            "random".to_string()
        }
    );

    let start = Instant::now();
    let mut bench_stats = BenchStats::default();

    for i in 0..cli.games {
        let game_seed = base_seed.wrapping_add(i as u64);
        let mut rng = StdRng::seed_from_u64(game_seed);
        let report = simulate_with_rng(&home, &away, &config, &mut rng);
        bench_stats.add(&report);
    }

    bench_stats.total_time_secs = start.elapsed().as_secs_f64();

    // ── Terminal output ────────────────────────────────────────────────────────
    if cli.verbose {
        let run_cfg = terminal::RunConfig {
            home_name: "Home FC",
            away_name: "Away FC",
            home_style: cli.home_style.label(),
            away_style: cli.away_style.label(),
            home_formation: &cli.home_formation,
            away_formation: &cli.away_formation,
            home_rating: cli.home_rating,
            away_rating: cli.away_rating,
            goal_conversion_base: config.goal_conversion_base,
            seed: cli.seed,
        };
        terminal::print_report(&bench_stats, &run_cfg);
    }

    // ── HTML output ───────────────────────────────────────────────────────────
    if let Some(ref html_path) = cli.html {
        let run_cfg = html::RunConfig {
            home_name: "Home FC",
            away_name: "Away FC",
            home_style: cli.home_style.label(),
            away_style: cli.away_style.label(),
            home_formation: &cli.home_formation,
            away_formation: &cli.away_formation,
            home_rating: cli.home_rating,
            away_rating: cli.away_rating,
            goal_conversion_base: config.goal_conversion_base,
            seed: cli.seed,
        };
        let content = html::generate_html(&bench_stats, &run_cfg);
        std::fs::write(html_path, content).expect("Failed to write HTML report");
        eprintln!("HTML report → {}", html_path.display());
    }

    // ── JSON output ───────────────────────────────────────────────────────────
    let json_summary = bench_stats.to_json(config.goal_conversion_base);
    let json = serde_json::to_string_pretty(&json_summary).expect("JSON serialization failed");

    if let Some(ref out_path) = cli.out {
        std::fs::write(out_path, &json).expect("Failed to write JSON");
        eprintln!("JSON summary → {}", out_path.display());
    } else if !cli.verbose {
        // Default mode: JSON to stdout
        println!("{json}");
    }
}

fn run_bench(config: &MatchConfig, games: u32, seed: Option<u64>) {
    let base = seed.unwrap_or(42);
    let mut team_rng = StdRng::seed_from_u64(base.wrapping_add(0xDEAD_BEEF));
    let home = build_team("home", "Home FC", 70, PlayStyle::Balanced, "4-4-2", &mut team_rng);
    let away = build_team("away", "Away FC", 70, PlayStyle::Balanced, "4-4-2", &mut team_rng);

    eprintln!("Bench mode: {} games…", games);

    let mut times: Vec<std::time::Duration> = Vec::with_capacity(games as usize);
    for i in 0..games {
        let game_seed = base.wrapping_add(i as u64);
        let mut rng = StdRng::seed_from_u64(game_seed);
        let t = Instant::now();
        let _ = simulate_with_rng(&home, &away, config, &mut rng);
        times.push(t.elapsed());
    }

    let total: std::time::Duration = times.iter().sum();
    let total_secs = total.as_secs_f64();
    let gps = games as f64 / total_secs;

    times.sort();
    let p50 = times[games as usize / 2];
    let p95 = times[(games as f64 * 0.95) as usize];
    let p99 = times[(games as f64 * 0.99) as usize];

    let sep = "═".repeat(50);
    println!("\n{}", sep.bright_cyan());
    println!("{}", "  BENCHMARK RESULTS".bold().bright_cyan());
    println!("{}", sep.bright_cyan());
    println!("  Games simulated : {games}");
    println!("  Total time      : {total_secs:.3}s");
    println!("  Throughput      : {gps:.0} games/sec");
    println!("  Latency p50     : {}µs", p50.as_micros());
    println!("  Latency p95     : {}µs", p95.as_micros());
    println!("  Latency p99     : {}µs", p99.as_micros());
    println!("{}", sep.bright_cyan());
}
