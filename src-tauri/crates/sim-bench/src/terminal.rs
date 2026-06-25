use colored::Colorize;
use comfy_table::{Cell, ContentArrangement, Table};

use crate::stats::BenchStats;

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

pub fn print_report(stats: &BenchStats, cfg: &RunConfig) {
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

    metric(
        "  Avg goals/game   ",
        gpg,
        2,
        "2.3–3.0",
        check(gpg, 2.3, 3.0),
    );
    metric("  Home goals/game  ", stats.home_gpg(), 2, "", true);
    metric("  Away goals/game  ", stats.away_gpg(), 2, "", true);
    metric(
        "  Clean sheet Home ",
        cs_h,
        1,
        "22–35%",
        check(cs_h, 22.0, 35.0),
    );
    metric(
        "  Clean sheet Away ",
        cs_a,
        1,
        "22–35%",
        check(cs_a, 22.0, 35.0),
    );
    metric(
        "  BTTS             ",
        btts,
        1,
        "50–55%",
        check(btts, 50.0, 55.0),
    );
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

    metric(
        "  Shots/game       ",
        shots,
        1,
        "18–32",
        check(shots, 18.0, 32.0),
    );
    metric(
        "  Shots on target %",
        acc,
        1,
        "32–45%",
        check(acc, 32.0, 45.0),
    );
    metric(
        "  Goal conversion %",
        conv,
        1,
        "20–40%",
        check(conv, 20.0, 40.0),
    );
    metric("  xG/game (proxy)  ", xg, 2, "", true);
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

    metric("  Yellow cards/game", y, 2, "2.0–4.0", check(y, 2.0, 4.0));
    metric(
        "  Red cards/game   ",
        r,
        3,
        "0.05–0.15",
        check(r, 0.05, 0.15),
    );
    metric("  Fouls/game       ", f, 1, "18–28", check(f, 18.0, 28.0));
    metric(
        "  Penalties/game   ",
        p,
        2,
        "0.20–0.50",
        check(p, 0.20, 0.50),
    );
    metric(
        "  Pen. conversion %",
        pc,
        1,
        "65–85%",
        check(pc, 65.0, 85.0),
    );
    metric("  Injuries/game    ", inj, 2, "", true);
    println!();

    // ── Set pieces ───────────────────────────────────────────────────────────
    section("SET PIECES");
    let c = stats.corners_pg();
    let fk = stats.free_kicks_pg();
    let gk = stats.goal_kicks_pg();
    let cr = stats.crosses_pg();

    metric("  Corners/game     ", c, 1, "8–14", check(c, 8.0, 14.0));
    metric("  Free kicks/game  ", fk, 1, "", true);
    metric("  Goal kicks/game  ", gk, 1, "8–14", check(gk, 8.0, 14.0));
    metric("  Crosses/game     ", cr, 1, "15–30", check(cr, 15.0, 30.0));
    println!();

    // ── Goal sources ─────────────────────────────────────────────────────────
    section("GOAL SOURCES");
    let op = stats.open_play_goal_pct();
    let co = stats.corner_goal_pct();
    let fkp = stats.free_kick_goal_pct();
    let pep = stats.penalty_goal_pct();

    metric("  Open play %      ", op, 1, "60–75%", check(op, 60.0, 75.0));
    metric("  Corners %        ", co, 1, "10–20%", check(co, 10.0, 20.0));
    metric("  Free kicks %     ", fkp, 1, "5–15%", check(fkp, 5.0, 15.0));
    metric("  Penalties %      ", pep, 1, "5–15%", check(pep, 5.0, 15.0));
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
    metric("  Passes/game      ", passes_pg, 1, "", true);
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
    println!("\n{}", sep.bright_cyan());
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

fn check(value: f64, lo: f64, hi: f64) -> bool {
    value >= lo && value <= hi
}

fn metric(label: &str, value: f64, decimals: usize, target: &str, ok: bool) {
    let val_str = format!("{:.prec$}", value, prec = decimals);
    if target.is_empty() {
        println!("{label}{val_str}");
    } else {
        let indicator = if ok {
            "✓".green().bold()
        } else {
            "✗".red().bold()
        };
        let target_str = format!("[{target}]").dimmed();
        println!("{label}{:<10}  {target_str}  {indicator}", val_str);
    }
}
