use colored::Colorize;
use comfy_table::{Cell, ContentArrangement, Table};

use crate::stats::BenchStats;
use crate::targets::{Provenance, Verdict};

pub struct RunConfig<'a> {
    pub home_name: &'a str,
    pub away_name: &'a str,
    pub home_style: &'a str,
    pub away_style: &'a str,
    pub home_formation: &'a str,
    pub away_formation: &'a str,
    pub home_rating: u8,
    pub away_rating: u8,
    pub goal_conversion_base: f64,
    pub seed: Option<u64>,
}

pub fn print_report(stats: &BenchStats, cfg: &RunConfig, verdicts: &[Verdict]) {
    let sep = "═".repeat(64);

    println!("\n{}", sep.bright_cyan());
    println!(
        "{}",
        format!(
            "  OFM SIMULATION REPORT  ·  {:>6} games  ·  {:.2}s",
            stats.games, stats.total_time_secs
        )
        .bright_cyan()
        .bold()
    );
    if let Some(s) = cfg.seed {
        println!("{}", format!("  seed: {s}").dimmed());
    }
    println!("{}\n", sep.bright_cyan());

    if stats.games == 0 {
        println!("{}", "  No games simulated.".yellow());
        println!("{}\n", sep.bright_cyan());
        return;
    }

    // ── Setup ────────────────────────────────────────────────────────────────
    section("SETUP");
    println!(
        "  Home  {:<10}  OVR {:>2}  {}",
        cfg.home_style.cyan(),
        cfg.home_rating,
        cfg.home_formation
    );
    println!(
        "  Away  {:<10}  OVR {:>2}  {}",
        cfg.away_style.cyan(),
        cfg.away_rating,
        cfg.away_formation
    );
    println!();

    // ── Results ──────────────────────────────────────────────────────────────
    section("RESULTS");
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Outcome", "Count", "%", ""]);

    let hw = stats.home_win_pct();
    let dp = stats.draw_pct();
    let aw = stats.away_win_pct();

    table.add_row(vec![
        Cell::new(format!("Home Win  ({})", cfg.home_name)),
        Cell::new(stats.home_wins.to_string()),
        Cell::new(format!("{:.1}%", hw)),
        Cell::new(ascii_bar(hw, 100.0, 20)),
    ]);
    table.add_row(vec![
        Cell::new("Draw"),
        Cell::new(stats.draws.to_string()),
        Cell::new(format!("{:.1}%", dp)),
        Cell::new(ascii_bar(dp, 100.0, 20)),
    ]);
    table.add_row(vec![
        Cell::new(format!("Away Win  ({})", cfg.away_name)),
        Cell::new(stats.away_wins.to_string()),
        Cell::new(format!("{:.1}%", aw)),
        Cell::new(ascii_bar(aw, 100.0, 20)),
    ]);
    println!("{table}\n");

    // ── Goals ────────────────────────────────────────────────────────────────
    section("GOALS");
    let gpg = stats.gpg();
    let cs_h = stats.clean_sheet_home_pct();
    let cs_a = stats.clean_sheet_away_pct();
    let btts = stats.btts_pct();

    metric("  Avg goals/game   ", gpg, 2);
    metric("  Home goals/game  ", stats.home_gpg(), 2);
    metric("  Away goals/game  ", stats.away_gpg(), 2);
    metric("  Clean sheet Home ", cs_h, 1);
    metric("  Clean sheet Away ", cs_a, 1);
    metric("  BTTS             ", btts, 1);
    println!();

    // Goals-per-game distribution
    println!("  Goals distribution:");
    let max_hist = stats
        .goals_per_game_hist
        .values()
        .copied()
        .max()
        .unwrap_or(1);
    for goals in 0u8..=9 {
        let count = stats.goals_per_game_hist.get(&goals).copied().unwrap_or(0);
        let pct = pct_of_games(count, stats.games);
        let bar = ascii_bar(count as f64, max_hist as f64, 18);
        let label = if goals == 9 {
            "9+".to_string()
        } else {
            goals.to_string()
        };
        println!("  {:>2} goals  {}  {:>5.1}%", label, bar, pct);
    }
    println!();

    // ── Top scorelines ───────────────────────────────────────────────────────
    section("MOST COMMON SCORELINES");
    let scorelines = stats.top_scorelines(8);
    let max_sl = scorelines.first().map(|(_, c)| *c).unwrap_or(1);
    for ((hg, ag), count) in &scorelines {
        let pct = pct_of_games(*count, stats.games);
        let bar = ascii_bar(*count as f64, max_sl as f64, 18);
        let hg_label = if *hg >= 6 {
            "6+".to_string()
        } else {
            hg.to_string()
        };
        let ag_label = if *ag >= 6 {
            "6+".to_string()
        } else {
            ag.to_string()
        };
        println!("  {hg_label}-{ag_label}  {bar}  {:>5.1}%", pct);
    }
    println!();

    // ── Shooting ─────────────────────────────────────────────────────────────
    section("SHOOTING");
    let shots = stats.shots_pg();
    let acc = stats.shot_accuracy_pct();
    let conv = stats.goal_conversion_pct();
    let xg = stats.xg_proxy_pg(cfg.goal_conversion_base);

    metric("  Shots/game       ", shots, 1);
    metric("  Shots on target %", acc, 1);
    metric("  Goal conversion %", conv, 1);
    metric("  xG/game (proxy)  ", xg, 2);
    let diff = stats.gpg() - xg;
    let diff_label = if diff >= 0.0 {
        format!("{:+.2} (overperforming)", diff).green().to_string()
    } else {
        format!("{:+.2} (underperforming)", diff)
            .yellow()
            .to_string()
    };
    println!("  Goals vs xG           {diff_label}");
    println!();

    // ── Discipline ───────────────────────────────────────────────────────────
    section("DISCIPLINE");
    let y = stats.yellows_pg();
    let r = stats.reds_pg();
    let f = stats.fouls_pg();
    let p = stats.penalties_pg();
    let pc = stats.penalty_conversion_pct();
    let inj = stats.injuries_pg();

    metric("  Yellow cards/game", y, 2);
    metric("  Red cards/game   ", r, 3);
    metric("  Fouls/game       ", f, 1);
    metric("  Penalties/game   ", p, 2);
    metric("  Pen. conversion %", pc, 1);
    metric("  Injuries/game    ", inj, 2);
    println!();

    // ── Set pieces ───────────────────────────────────────────────────────────
    section("SET PIECES");
    let c = stats.corners_pg();
    let fk = stats.free_kicks_pg();
    let gk = stats.goal_kicks_pg();
    let cr = stats.crosses_pg();

    metric("  Corners/game     ", c, 1);
    metric("  Free kicks/game  ", fk, 1);
    metric("  Goal kicks/game  ", gk, 1);
    metric("  Crosses/game     ", cr, 1);
    println!();

    // ── Goal sources ─────────────────────────────────────────────────────────
    section("GOAL SOURCES");
    let op = stats.open_play_goal_pct();
    let co = stats.corner_goal_pct();
    let fkp = stats.free_kick_goal_pct();
    let pep = stats.penalty_goal_pct();

    metric("  Open play %      ", op, 1);
    metric("  Corners %        ", co, 1);
    metric("  Free kicks %     ", fkp, 1);
    metric("  Penalties %      ", pep, 1);
    println!();

    // ── Possession & passing ─────────────────────────────────────────────────
    section("POSSESSION & PASSING");
    let hp = stats.avg_home_possession();

    println!(
        "  Home possession avg  {:.1}%  |  Away {:.1}%",
        hp,
        100.0 - hp
    );
    let passes_pg = stats.passes_completed as f64 / stats.games as f64;
    metric("  Passes/game      ", passes_pg, 1);
    // Note: passes_intercepted only covers buildup zone; pass accuracy intentionally omitted
    println!();

    // ── Scoring timeline ─────────────────────────────────────────────────────
    section("SCORING TIMELINE");
    let bucket_labels = [
        "1–15 ", "16–30", "31–45", "46–60", "61–75", "76–90", "90+  ",
    ];
    let max_b = stats.goals_by_bucket.iter().copied().max().unwrap_or(1) as f64;
    for (i, &count) in stats.goals_by_bucket.iter().enumerate() {
        let pct = if stats.total_goals > 0 {
            count as f64 / stats.total_goals as f64 * 100.0
        } else {
            0.0
        };
        let bar = ascii_bar(count as f64, max_b, 20);
        println!("  {}  {}  {:>5.1}%", bucket_labels[i], bar, pct);
    }
    println!();

    // ── Performance ──────────────────────────────────────────────────────────
    section("PERFORMANCE");
    println!(
        "  {} games in {:.2}s  ·  {:.0} games/sec",
        stats.games,
        stats.total_time_secs,
        stats.games_per_sec()
    );
    println!();

    calibration(verdicts);
    println!("{}", sep.bright_cyan());
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn section(title: &str) {
    println!("{}", format!("  {title}").bold().white());
}

fn ascii_bar(value: f64, max_val: f64, width: usize) -> String {
    if max_val <= 0.0 {
        return "░".repeat(width);
    }
    let filled = ((value / max_val) * width as f64).round() as usize;
    let filled = filled.min(width);
    format!(
        "{}{}",
        "█".repeat(filled).bright_blue(),
        "░".repeat(width - filled).dimmed()
    )
}

fn pct_of_games(count: u32, games: u32) -> f64 {
    if games == 0 {
        return 0.0;
    }
    count as f64 / games as f64 * 100.0
}

fn metric(label: &str, value: f64, decimals: usize) {
    println!("{label}{:.prec$}", value, prec = decimals);
}

/// Every metric against its band, with the band's source. A tick or a cross is a claim about real
/// football, so it is printed only where the band is measured and the bench plays what it
/// measures; anything else gets a quiet dot (inside) or bang (outside).
fn calibration(verdicts: &[Verdict]) {
    section("CALIBRATION BANDS");
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec!["Metric", "Value", "Band", "Source", ""]);
    for verdict in verdicts {
        let (source, judged) = match &verdict.provenance {
            Provenance::Measured { seasons, .. } => (
                format!("measured, {seasons}"),
                verdict.not_comparable.is_none(),
            ),
            Provenance::Unsourced => ("unsourced".dimmed().to_string(), false),
        };
        let mark = match (judged, verdict.inside) {
            (true, true) => "✓".green().bold().to_string(),
            (true, false) => "✗".red().bold().to_string(),
            (false, true) => "·".dimmed().to_string(),
            (false, false) => "!".dimmed().to_string(),
        };
        table.add_row(vec![
            Cell::new(verdict.label),
            Cell::new(verdict.unit.value(verdict.value)),
            Cell::new(verdict.unit.band(verdict.low, verdict.high)),
            Cell::new(source),
            Cell::new(mark),
        ]);
    }
    println!("{table}");
    if let Some(Provenance::Measured {
        source,
        competitions,
        retrieved,
        ..
    }) = verdicts
        .iter()
        .map(|verdict| &verdict.provenance)
        .find(|provenance| matches!(provenance, Provenance::Measured { .. }))
    {
        println!(
            "  {} {source}: {competitions} (retrieved {retrieved})",
            "measured:".dimmed()
        );
    }
    let caveated: Vec<&str> = verdicts
        .iter()
        .filter(|verdict| verdict.not_comparable.is_some())
        .map(|verdict| verdict.label)
        .collect();
    if let Some(reason) = verdicts.iter().find_map(|verdict| verdict.not_comparable) {
        println!(
            "  {} {}: {}",
            "n/c".dimmed(),
            caveated.join(", "),
            reason.dimmed()
        );
    }
    println!();
}
