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

pub fn generate_html(stats: &BenchStats, cfg: &RunConfig) -> String {
    let seed_line = cfg.seed.map(|s| format!(" · seed {s}")).unwrap_or_default();

    let xg = stats.xg_proxy_pg(cfg.goal_conversion_base);
    let goals_vs_xg = stats.gpg() - xg;
    let hp = stats.avg_home_possession();

    // Sections
    let setup_html = setup_section(cfg);
    let results_html = results_section(stats, cfg);
    let goals_hist_html = goals_histogram(stats);
    let scoreline_html = scoreline_heatmap(stats);
    let shooting_html = shooting_section(stats, xg, goals_vs_xg);
    let timeline_html = timeline_section(stats);
    let discipline_html = discipline_section(stats);
    let possession_html = possession_section(stats, hp);
    let goal_sources_html = goal_sources_section(stats);
    let benchmark_html = benchmark_section(stats);
    let targets_html = targets_table(stats, xg);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OFM Simulation Report — {games} games{seed_line}</title>
<style>
  :root {{
    --bg: #0f1117;
    --surface: #1a1d27;
    --surface2: #222536;
    --border: #2d3149;
    --accent: #4f8ef7;
    --accent2: #7c6cf7;
    --green: #34d399;
    --yellow: #fbbf24;
    --red: #f87171;
    --text: #e2e8f0;
    --muted: #64748b;
    --home: #4f8ef7;
    --draw: #94a3b8;
    --away: #f472b6;
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
    line-height: 1.5;
  }}
  .header {{
    background: linear-gradient(135deg, #1a1d27 0%, #0f1117 100%);
    border-bottom: 1px solid var(--border);
    padding: 24px 32px;
  }}
  .header h1 {{
    font-size: 22px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: 0.5px;
  }}
  .header .sub {{
    color: var(--muted);
    font-size: 13px;
    margin-top: 4px;
  }}
  .container {{ max-width: 1200px; margin: 0 auto; padding: 24px 32px; }}
  .grid-2 {{ display: grid; grid-template-columns: 1fr 1fr; gap: 20px; }}
  .grid-3 {{ display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 20px; }}
  .card {{
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 20px;
  }}
  .card h2 {{
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--muted);
    margin-bottom: 16px;
  }}
  .stat-row {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
  }}
  .stat-row:last-child {{ border-bottom: none; }}
  .stat-label {{ color: var(--muted); }}
  .stat-value {{ font-weight: 600; font-variant-numeric: tabular-nums; }}
  .ok {{ color: var(--green); }}
  .warn {{ color: var(--yellow); }}
  .bad {{ color: var(--red); }}
  .tag {{
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
  }}
  .tag-ok {{ background: rgba(52,211,153,0.15); color: var(--green); }}
  .tag-bad {{ background: rgba(248,113,113,0.15); color: var(--red); }}
  /* Bar charts */
  .bar-container {{
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
  }}
  .bar-row {{
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .bar-label {{ width: 60px; font-size: 12px; color: var(--muted); text-align: right; }}
  .bar-label-left {{ width: 60px; font-size: 12px; color: var(--muted); }}
  .bar-track {{
    flex: 1;
    height: 18px;
    background: var(--surface2);
    border-radius: 3px;
    overflow: hidden;
    position: relative;
  }}
  .bar-fill {{
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s;
  }}
  .bar-pct {{
    font-size: 12px;
    color: var(--muted);
    width: 45px;
    text-align: right;
  }}
  /* Histogram */
  .histogram {{
    display: flex;
    align-items: flex-end;
    gap: 6px;
    height: 120px;
    margin-top: 8px;
  }}
  .hist-col {{
    display: flex;
    flex-direction: column;
    align-items: center;
    flex: 1;
    gap: 4px;
  }}
  .hist-bar {{
    width: 100%;
    background: var(--accent);
    border-radius: 3px 3px 0 0;
    min-height: 2px;
  }}
  .hist-label {{
    font-size: 11px;
    color: var(--muted);
  }}
  /* Heatmap */
  .heatmap-wrap {{ overflow-x: auto; margin-top: 8px; }}
  .heatmap {{
    display: grid;
    gap: 3px;
  }}
  .heatmap-cell {{
    width: 52px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
  }}
  .heatmap-header {{ background: var(--surface2); color: var(--muted); font-size: 11px; }}
  /* Funnel */
  .funnel {{ margin-top: 8px; }}
  .funnel-step {{
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }}
  .funnel-step:last-child {{ border-bottom: none; }}
  .funnel-label {{ flex: 1; color: var(--muted); }}
  .funnel-value {{ font-weight: 700; font-size: 18px; color: var(--text); width: 70px; text-align: right; }}
  .funnel-sub {{ font-size: 11px; color: var(--muted); width: 60px; text-align: right; }}
  /* Setup table */
  .setup-table {{ width: 100%; border-collapse: collapse; margin-top: 8px; }}
  .setup-table td {{ padding: 6px 12px; }}
  .setup-table .side {{ font-weight: 700; color: var(--text); }}
  .setup-table .style {{ color: var(--accent); }}
  .setup-table .rating {{
    background: var(--surface2);
    border-radius: 4px;
    padding: 2px 8px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }}
  /* SVG donut */
  .donut-wrap {{
    display: flex;
    align-items: center;
    gap: 24px;
  }}
  .donut-legend {{ display: flex; flex-direction: column; gap: 10px; }}
  .legend-item {{ display: flex; align-items: center; gap: 8px; font-size: 13px; }}
  .legend-dot {{ width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }}
  .legend-pct {{ color: var(--muted); font-size: 12px; }}
  /* Timeline SVG */
  .section-title {{
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    margin-bottom: 4px;
  }}
  @media print {{
    body {{ background: #fff; color: #000; }}
    .card {{ border: 1px solid #ccc; background: #fff; }}
    .bar-fill, .hist-bar {{ print-color-adjust: exact; -webkit-print-color-adjust: exact; }}
  }}
</style>
</head>
<body>
<div class="header">
  <h1>OFM Simulation Report</h1>
  <div class="sub">{games} games{seed_line}</div>
</div>
<div class="container">

  {setup_html}

  <div style="height:20px"></div>

  <div class="grid-2">
    {results_html}
    {goals_hist_html}
  </div>

  <div style="height:20px"></div>

  {scoreline_html}

  <div style="height:20px"></div>

  <div class="grid-2">
    {shooting_html}
    {timeline_html}
  </div>

  <div style="height:20px"></div>

  <div class="grid-3">
    {discipline_html}
    {possession_html}
    {benchmark_html}
  </div>

  <div style="height:20px"></div>

  {goal_sources_html}

  <div style="height:20px"></div>

  {targets_html}

  <div style="height:32px"></div>
</div>
</body>
</html>"#,
        games = stats.games,
        seed_line = seed_line,
        setup_html = setup_html,
        results_html = results_html,
        goals_hist_html = goals_hist_html,
        scoreline_html = scoreline_html,
        shooting_html = shooting_html,
        timeline_html = timeline_html,
        discipline_html = discipline_html,
        possession_html = possession_html,
        goal_sources_html = goal_sources_html,
        benchmark_html = benchmark_html,
        targets_html = targets_html,
    )
}

// ── Section generators ────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn setup_section(cfg: &RunConfig) -> String {
    format!(
        r#"<div class="card">
  <h2>Match Setup</h2>
  <table class="setup-table">
    <tr>
      <td class="side">Home</td>
      <td class="style">{home_style}</td>
      <td>{home_formation}</td>
      <td><span class="rating">OVR {home_rating}</span></td>
      <td style="color:var(--muted)">({home_name})</td>
    </tr>
    <tr>
      <td class="side">Away</td>
      <td class="style">{away_style}</td>
      <td>{away_formation}</td>
      <td><span class="rating">OVR {away_rating}</span></td>
      <td style="color:var(--muted)">({away_name})</td>
    </tr>
  </table>
</div>"#,
        home_style = esc(cfg.home_style),
        home_formation = esc(cfg.home_formation),
        home_rating = cfg.home_rating,
        home_name = esc(cfg.home_name),
        away_style = esc(cfg.away_style),
        away_formation = esc(cfg.away_formation),
        away_rating = cfg.away_rating,
        away_name = esc(cfg.away_name),
    )
}

fn results_section(stats: &BenchStats, _cfg: &RunConfig) -> String {
    let hw = stats.home_win_pct();
    let dp = stats.draw_pct();
    let aw = stats.away_win_pct();

    let donut = donut_svg(hw, dp, aw);

    let legend = format!(
        r#"<div class="donut-legend">
  <div class="legend-item">
    <div class="legend-dot" style="background:var(--home)"></div>
    <div>
      <div>Home Win</div>
      <div class="legend-pct">{} &nbsp;{:.1}%</div>
    </div>
  </div>
  <div class="legend-item">
    <div class="legend-dot" style="background:var(--draw)"></div>
    <div>
      <div>Draw</div>
      <div class="legend-pct">{} &nbsp;{:.1}%</div>
    </div>
  </div>
  <div class="legend-item">
    <div class="legend-dot" style="background:var(--away)"></div>
    <div>
      <div>Away Win</div>
      <div class="legend-pct">{} &nbsp;{:.1}%</div>
    </div>
  </div>
</div>"#,
        stats.home_wins, hw, stats.draws, dp, stats.away_wins, aw
    );

    // Key numbers
    let gpg = stats.gpg();
    let cs_h = stats.clean_sheet_home_pct();
    let btts = stats.btts_pct();

    format!(
        r#"<div class="card">
  <h2>Match Outcomes</h2>
  <div class="donut-wrap">
    {donut}
    {legend}
  </div>
  <div style="height:16px"></div>
  <div class="stat-row">
    <span class="stat-label">Avg goals/game</span>
    <span class="stat-value {}">{:.2}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Clean sheet (home)</span>
    <span class="stat-value">{:.1}%</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Both teams scored</span>
    <span class="stat-value">{:.1}%</span>
  </div>
</div>"#,
        range_class(gpg, 2.3, 3.0),
        gpg,
        cs_h,
        btts,
    )
}

fn goals_histogram(stats: &BenchStats) -> String {
    let max = stats
        .goals_per_game_hist
        .values()
        .copied()
        .max()
        .unwrap_or(1) as f64;

    let mut bars = String::new();
    for goals in 0u8..=9 {
        let count = stats.goals_per_game_hist.get(&goals).copied().unwrap_or(0);
        let pct = count as f64 / stats.games as f64 * 100.0;
        let height = ((count as f64 / max) * 100.0).round() as u32;
        let label = if goals == 9 {
            "9+".to_string()
        } else {
            goals.to_string()
        };
        bars.push_str(&format!(
            r#"<div class="hist-col">
  <div class="hist-bar" style="height:{height}px" title="{pct:.1}%"></div>
  <div class="hist-label">{label}</div>
</div>"#
        ));
    }

    format!(
        r#"<div class="card">
  <h2>Goals per Game Distribution</h2>
  <div class="histogram">{bars}</div>
  <div style="margin-top:8px; display:flex; justify-content:space-between; font-size:11px; color:var(--muted)">
    <span>0 goals</span>
    <span>← goals per game →</span>
    <span>9+ goals</span>
  </div>
</div>"#
    )
}

fn scoreline_heatmap(stats: &BenchStats) -> String {
    let max_count = stats.scorelines.values().copied().max().unwrap_or(1) as f64;

    // Build 7×7 grid (0–6 per side), first row/col are headers
    // columns = away goals 0..6, rows = home goals 0..6
    let cols = 8; // label col + 7 data cols
    let mut cells = String::new();

    // Header row
    cells.push_str(r#"<div class="heatmap-cell heatmap-header"></div>"#);
    for ag in 0u8..=6 {
        let label = if ag == 6 {
            "6+".to_string()
        } else {
            ag.to_string()
        };
        cells.push_str(&format!(
            r#"<div class="heatmap-cell heatmap-header">{label}</div>"#
        ));
    }

    // Data rows
    for hg in 0u8..=6 {
        let label = if hg == 6 {
            "6+".to_string()
        } else {
            hg.to_string()
        };
        cells.push_str(&format!(
            r#"<div class="heatmap-cell heatmap-header">{label}</div>"#
        ));
        for ag in 0u8..=6 {
            let count = stats.scorelines.get(&(hg, ag)).copied().unwrap_or(0);
            let pct = count as f64 / stats.games as f64 * 100.0;
            let opacity = if count == 0 {
                0.0
            } else {
                0.1 + (count as f64 / max_count) * 0.85
            };
            let text_color = if opacity > 0.55 {
                "#fff"
            } else {
                "var(--muted)"
            };
            cells.push_str(&format!(
                r#"<div class="heatmap-cell" style="background:rgba(79,142,247,{opacity:.2});color:{text_color}" title="{hg}-{ag}: {count} games ({pct:.1}%)">{pct_text}</div>"#,
                pct_text = if count == 0 {
                    "".to_string()
                } else {
                    format!("{pct:.0}%")
                }
            ));
        }
    }

    format!(
        r#"<div class="card">
  <h2>Scoreline Heatmap  <span style="font-size:11px;font-weight:400;text-transform:none;letter-spacing:0">— rows = home goals, columns = away goals</span></h2>
  <div class="heatmap-wrap">
    <div class="heatmap" style="grid-template-columns:repeat({cols},52px)">{cells}</div>
  </div>
</div>"#
    )
}

fn shooting_section(stats: &BenchStats, xg: f64, goals_vs_xg: f64) -> String {
    let shots = stats.shots_pg();
    let sot = stats.shots_on_target as f64 / stats.games as f64;
    let goals_pg = stats.gpg();
    let acc = stats.shot_accuracy_pct();
    let conv = stats.goal_conversion_pct();

    let diff_class = if goals_vs_xg >= 0.0 { "ok" } else { "warn" };
    let diff_sign = if goals_vs_xg >= 0.0 { "+" } else { "" };

    format!(
        r#"<div class="card">
  <h2>Shooting</h2>
  <div class="funnel">
    <div class="funnel-step">
      <div class="funnel-label">Shots (total)</div>
      <div class="funnel-value">{shots:.1}</div>
      <div class="funnel-sub">per game</div>
    </div>
    <div class="funnel-step">
      <div class="funnel-label">Shots on Target</div>
      <div class="funnel-value">{sot:.1}</div>
      <div class="funnel-sub">{acc:.1}% acc.</div>
    </div>
    <div class="funnel-step">
      <div class="funnel-label">Goals</div>
      <div class="funnel-value">{goals_pg:.2}</div>
      <div class="funnel-sub">{conv:.1}% conv.</div>
    </div>
  </div>
  <div style="height:12px"></div>
  <div class="stat-row">
    <span class="stat-label">xG/game (proxy)</span>
    <span class="stat-value">{xg:.2}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Goals vs xG</span>
    <span class="stat-value {diff_class}">{diff_sign}{goals_vs_xg:.2}</span>
  </div>
</div>"#
    )
}

fn timeline_section(stats: &BenchStats) -> String {
    let labels = ["1–15", "16–30", "31–45", "46–60", "61–75", "76–90", "90+"];
    let max = stats.goals_by_bucket.iter().copied().max().unwrap_or(1) as f64;
    let total = stats.total_goals as f64;

    let mut rows = String::new();
    for (i, &count) in stats.goals_by_bucket.iter().enumerate() {
        let pct = if total > 0.0 {
            count as f64 / total * 100.0
        } else {
            0.0
        };
        let width = ((count as f64 / max) * 100.0).round() as u32;
        rows.push_str(&format!(
            r#"<div class="bar-row">
  <div class="bar-label">{label}</div>
  <div class="bar-track">
    <div class="bar-fill" style="width:{width}%;background:var(--accent)"></div>
  </div>
  <div class="bar-pct">{pct:.1}%</div>
</div>"#,
            label = labels[i]
        ));
    }

    format!(
        r#"<div class="card">
  <h2>Scoring Timeline</h2>
  <p style="font-size:11px;color:var(--muted);margin-bottom:12px">Goals distribution by 15-minute interval</p>
  <div class="bar-container">{rows}</div>
</div>"#
    )
}

fn discipline_section(stats: &BenchStats) -> String {
    let y = stats.yellows_pg();
    let r = stats.reds_pg();
    let f = stats.fouls_pg();
    let p = stats.penalties_pg();
    let pc = stats.penalty_conversion_pct();
    let inj = stats.injuries_pg();

    format!(
        r#"<div class="card">
  <h2>Discipline</h2>
  <div class="stat-row">
    <span class="stat-label">Yellow cards/game</span>
    <span class="stat-value {}">{:.2}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Red cards/game</span>
    <span class="stat-value {}">{:.3}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Fouls/game</span>
    <span class="stat-value {}">{:.1}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Penalties/game</span>
    <span class="stat-value {}">{:.2}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Pen. conversion</span>
    <span class="stat-value {}">{:.1}%</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Injuries/game</span>
    <span class="stat-value">{:.2}</span>
  </div>
</div>"#,
        range_class(y, 2.0, 4.0),
        y,
        range_class(r, 0.05, 0.15),
        r,
        range_class(f, 18.0, 28.0),
        f,
        range_class(p, 0.20, 0.50),
        p,
        range_class(pc, 65.0, 85.0),
        pc,
        inj,
    )
}

fn possession_section(stats: &BenchStats, hp: f64) -> String {
    let ap = 100.0 - hp;
    let corners = stats.corners_pg();
    let fk = stats.free_kicks_pg();
    let gk = stats.goal_kicks_pg();
    let cr = stats.crosses_pg();
    let passes_pg = stats.passes_completed as f64 / stats.games as f64;

    let home_w = hp.round() as u32;
    let away_w = ap.round() as u32;

    format!(
        r#"<div class="card">
  <h2>Possession &amp; Set Pieces</h2>
  <p style="font-size:11px;color:var(--muted);margin-bottom:8px">Average possession split</p>
  <div style="display:flex;gap:2px;height:24px;border-radius:4px;overflow:hidden;margin-bottom:12px">
    <div style="width:{home_w}%;background:var(--home);display:flex;align-items:center;justify-content:center;font-size:12px;font-weight:700;color:#fff">{hp:.1}%</div>
    <div style="width:{away_w}%;background:var(--away);display:flex;align-items:center;justify-content:center;font-size:12px;font-weight:700;color:#fff">{ap:.1}%</div>
  </div>
  <div class="stat-row">
    <span class="stat-label">Passes/game</span>
    <span class="stat-value">{passes_pg:.0}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Corners/game</span>
    <span class="stat-value {corner_cls}">{corners:.1}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Free kicks/game</span>
    <span class="stat-value">{fk:.1}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Goal kicks/game</span>
    <span class="stat-value {gk_cls}">{gk:.1}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Crosses/game</span>
    <span class="stat-value {cr_cls}">{cr:.1}</span>
  </div>
</div>"#,
        passes_pg = passes_pg,
        corner_cls = range_class(corners, 8.0, 14.0),
        corners = corners,
        fk = fk,
        gk_cls = range_class(gk, 8.0, 14.0),
        gk = gk,
        cr_cls = range_class(cr, 15.0, 30.0),
        cr = cr,
        hp = hp,
        ap = ap,
        home_w = home_w,
        away_w = away_w,
    )
}

fn goal_sources_section(stats: &BenchStats) -> String {
    let op = stats.open_play_goal_pct();
    let co = stats.corner_goal_pct();
    let fk = stats.free_kick_goal_pct();
    let pe = stats.penalty_goal_pct();

    format!(
        r#"<div class="card">
  <h2>Goal Sources</h2>
  <div class="stat-row">
    <span class="stat-label">Open play %</span>
    <span class="stat-value {op_cls}">{op:.1}%</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Corners %</span>
    <span class="stat-value {co_cls}">{co:.1}%</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Free kicks %</span>
    <span class="stat-value {fk_cls}">{fk:.1}%</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Penalties %</span>
    <span class="stat-value {pe_cls}">{pe:.1}%</span>
  </div>
</div>"#,
        op_cls = range_class(op, 60.0, 75.0),
        op = op,
        co_cls = range_class(co, 10.0, 20.0),
        co = co,
        fk_cls = range_class(fk, 5.0, 15.0),
        fk = fk,
        pe_cls = range_class(pe, 5.0, 15.0),
        pe = pe,
    )
}

fn benchmark_section(stats: &BenchStats) -> String {
    let gps = stats.games_per_sec();

    format!(
        r#"<div class="card">
  <h2>Performance</h2>
  <div style="text-align:center;padding:16px 0">
    <div style="font-size:36px;font-weight:800;color:var(--accent)">{gps:.0}</div>
    <div style="font-size:12px;color:var(--muted);margin-top:4px">games / second</div>
  </div>
  <div class="stat-row">
    <span class="stat-label">Games</span>
    <span class="stat-value">{games}</span>
  </div>
  <div class="stat-row">
    <span class="stat-label">Total time</span>
    <span class="stat-value">{time:.3}s</span>
  </div>
</div>"#,
        gps = gps,
        games = stats.games,
        time = stats.total_time_secs,
    )
}

fn targets_table(stats: &BenchStats, xg: f64) -> String {
    let rows = [
        ("Goals/game", stats.gpg(), 2.3, 3.0, "f64"),
        ("Shots/game", stats.shots_pg(), 18.0, 32.0, "f64"),
        (
            "Shots on target %",
            stats.shot_accuracy_pct(),
            32.0,
            45.0,
            "pct",
        ),
        (
            "Goal conversion %",
            stats.goal_conversion_pct(),
            20.0,
            40.0,
            "pct",
        ),
        ("Yellow cards/game", stats.yellows_pg(), 2.0, 4.0, "f64"),
        ("Red cards/game", stats.reds_pg(), 0.05, 0.15, "f64"),
        ("Fouls/game", stats.fouls_pg(), 18.0, 28.0, "f64"),
        ("Corners/game", stats.corners_pg(), 8.0, 14.0, "f64"),
        ("Goal kicks/game", stats.goal_kicks_pg(), 8.0, 14.0, "f64"),
        ("Crosses/game", stats.crosses_pg(), 15.0, 30.0, "f64"),
        ("Home win %", stats.home_win_pct(), 40.0, 52.0, "pct"),
        (
            "Clean sheet %",
            stats.clean_sheet_home_pct(),
            22.0,
            35.0,
            "pct",
        ),
        ("Penalties/game", stats.penalties_pg(), 0.20, 0.50, "f64"),
        (
            "Pen. conversion %",
            stats.penalty_conversion_pct(),
            65.0,
            85.0,
            "pct",
        ),
        ("BTTS %", stats.btts_pct(), 50.0, 55.0, "pct"),
        ("Open play goals %", stats.open_play_goal_pct(), 60.0, 75.0, "pct"),
        ("Corner goals %", stats.corner_goal_pct(), 10.0, 20.0, "pct"),
        ("Free kick goals %", stats.free_kick_goal_pct(), 5.0, 15.0, "pct"),
        ("Penalty goals %", stats.penalty_goal_pct(), 5.0, 15.0, "pct"),
        ("xG/game (proxy)", xg, 0.0, 9999.0, "f64"),
    ];

    let mut body = String::new();
    for (label, value, lo, hi, fmt) in &rows {
        let ok = *value >= *lo && *value <= *hi;
        let value_str = match *fmt {
            "pct" => format!("{value:.1}%"),
            _ => format!("{value:.2}"),
        };
        let target_str = if *hi >= 9000.0 {
            "—".to_string()
        } else if *fmt == "pct" {
            format!("{lo}–{hi}%")
        } else {
            format!("{lo}–{hi}")
        };
        let badge = if *hi >= 9000.0 {
            String::new()
        } else if ok {
            r#"<span class="tag tag-ok">✓ OK</span>"#.to_string()
        } else {
            r#"<span class="tag tag-bad">✗ Off target</span>"#.to_string()
        };
        body.push_str(&format!(
            r#"<tr>
  <td style="padding:8px 12px;color:var(--muted)">{label}</td>
  <td style="padding:8px 12px;font-weight:700;font-variant-numeric:tabular-nums">{value_str}</td>
  <td style="padding:8px 12px;color:var(--muted)">{target_str}</td>
  <td style="padding:8px 12px">{badge}</td>
</tr>"#
        ));
    }

    format!(
        r#"<div class="card">
  <h2>Real-Football Benchmark Comparison</h2>
  <table style="width:100%;border-collapse:collapse">
    <thead>
      <tr style="border-bottom:1px solid var(--border)">
        <th style="padding:8px 12px;text-align:left;color:var(--muted);font-size:11px;font-weight:600;text-transform:uppercase">Metric</th>
        <th style="padding:8px 12px;text-align:left;color:var(--muted);font-size:11px;font-weight:600;text-transform:uppercase">Simulated</th>
        <th style="padding:8px 12px;text-align:left;color:var(--muted);font-size:11px;font-weight:600;text-transform:uppercase">Real-World Target</th>
        <th style="padding:8px 12px;text-align:left;color:var(--muted);font-size:11px;font-weight:600;text-transform:uppercase">Status</th>
      </tr>
    </thead>
    <tbody>{body}</tbody>
  </table>
</div>"#
    )
}

// ── SVG Helpers ───────────────────────────────────────────────────────────────

fn donut_svg(home_pct: f64, draw_pct: f64, _away_pct: f64) -> String {
    // SVG donut using stroke-dasharray on a circle with r=40, circumference≈251
    let circ = 2.0 * std::f64::consts::PI * 40.0;
    let home_dash = home_pct / 100.0 * circ;
    let draw_dash = draw_pct / 100.0 * circ;
    let away_dash = circ - home_dash - draw_dash;

    // Segments start from top (rotate -90deg)
    let home_offset = 0.0;
    let draw_offset = circ - home_dash;
    let away_offset = circ - home_dash - draw_dash;

    format!(
        r#"<svg width="120" height="120" viewBox="0 0 120 120">
  <circle cx="60" cy="60" r="40" fill="none" stroke="var(--surface2)" stroke-width="18"/>
  <circle cx="60" cy="60" r="40" fill="none" stroke="var(--home)" stroke-width="18"
    stroke-dasharray="{home_dash:.1} {rest1:.1}"
    stroke-dashoffset="{home_ofs:.1}"
    transform="rotate(-90 60 60)"/>
  <circle cx="60" cy="60" r="40" fill="none" stroke="var(--draw)" stroke-width="18"
    stroke-dasharray="{draw_dash:.1} {rest2:.1}"
    stroke-dashoffset="{draw_ofs:.1}"
    transform="rotate(-90 60 60)"/>
  <circle cx="60" cy="60" r="40" fill="none" stroke="var(--away)" stroke-width="18"
    stroke-dasharray="{away_dash:.1} {rest3:.1}"
    stroke-dashoffset="{away_ofs:.1}"
    transform="rotate(-90 60 60)"/>
</svg>"#,
        home_dash = home_dash,
        rest1 = circ - home_dash,
        home_ofs = circ - home_offset,
        draw_dash = draw_dash,
        rest2 = circ - draw_dash,
        draw_ofs = circ - draw_offset,
        away_dash = away_dash,
        rest3 = circ - away_dash,
        away_ofs = circ - away_offset,
    )
}

fn range_class(value: f64, lo: f64, hi: f64) -> &'static str {
    if value >= lo && value <= hi {
        "ok"
    } else {
        "bad"
    }
}
