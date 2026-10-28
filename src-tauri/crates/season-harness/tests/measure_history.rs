//! How varied a new world's generated past is: champions, their share of the titles, the spread
//! of the champion's points, how often the best-reputation club wins.
//!
//! Measurement only, `#[ignore]`d: `cargo test --release -p season-harness --test measure_history
//! -- --ignored --nocapture`

use std::collections::HashMap;

use domain::league::{CompetitionScope, CompetitionType};
use ofm_core::game::Game;
use ofm_core::history_generation::generate_past_world_history;
use season_harness::worlds::WorldSpec;

/// The app's default depth for a generated world (`DEFAULT_GENERATED_HISTORY_DEPTH_YEARS`).
const DEPTH: u32 = 12;
/// A longer past, to read the same bands over twenty seasons.
const LONG_DEPTH: u32 = 20;

struct LeagueRow {
    name: String,
    clubs: usize,
    distinct: usize,
    top_share: f64,
    top_reputation_wins: f64,
    champion_points: Vec<u32>,
}

fn rows(game: &Game, start_year: i32, depth: u32) -> Vec<LeagueRow> {
    let first = (start_year - depth as i32) as u32;
    let mut seen: std::collections::HashSet<Vec<String>> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for competition in &game.competitions {
        if competition.kind != CompetitionType::League
            || competition.scope != CompetitionScope::Domestic
        {
            continue;
        }
        let members: Vec<&domain::team::Team> = game
            .teams
            .iter()
            .filter(|team| competition.participant_ids.contains(&team.id))
            .collect();
        if members.len() < 10 {
            continue;
        }
        // A second competition over the same clubs (a Clausura) has no table of its own.
        let mut ids: Vec<String> = members.iter().map(|team| team.id.clone()).collect();
        ids.sort();
        if !seen.insert(ids) {
            continue;
        }
        let best_reputation = members
            .iter()
            .map(|team| team.reputation)
            .max()
            .unwrap_or(0);
        let mut wins_by_club: HashMap<&str, u32> = HashMap::new();
        let mut champion_points = Vec::new();
        let mut top_reputation_wins = 0u32;
        let mut seasons = 0u32;
        for season in first..start_year as u32 {
            let champion = members
                .iter()
                .filter_map(|team| {
                    team.history
                        .iter()
                        .find(|record| record.season == season)
                        .map(|record| (team, record))
                })
                .min_by_key(|(_, record)| record.league_position);
            let Some((team, record)) = champion else {
                continue;
            };
            seasons += 1;
            *wins_by_club.entry(team.id.as_str()).or_default() += 1;
            champion_points.push(record.won * 3 + record.drawn);
            if team.reputation == best_reputation {
                top_reputation_wins += 1;
            }
        }
        if seasons == 0 {
            continue;
        }
        out.push(LeagueRow {
            name: competition.name.clone(),
            clubs: members.len(),
            distinct: wins_by_club.len(),
            top_share: f64::from(*wins_by_club.values().max().unwrap()) / f64::from(seasons),
            top_reputation_wins: f64::from(top_reputation_wins) / f64::from(seasons),
            champion_points,
        });
    }
    out
}

fn spread(values: &[u32]) -> f64 {
    let mean = values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64;
    (values
        .iter()
        .map(|v| (f64::from(*v) - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64)
        .sqrt()
}

fn measure(name: &str, spec: WorldSpec, depth: u32, print_leagues: bool) {
    let mut game = spec.build().expect("the world builds");
    generate_past_world_history(&mut game, spec.start_year, depth);
    let rows = rows(&game, spec.start_year, depth);
    let n = rows.len() as f64;
    let mean = |f: &dyn Fn(&LeagueRow) -> f64| rows.iter().map(f).sum::<f64>() / n;
    eprintln!(
        "{name:>22} | {} leagues, {depth} seasons | distinct champions {:.1} | top club's share {:.0}% | best-reputation club wins {:.0}% | champion points sd {:.1}",
        rows.len(),
        mean(&|r| r.distinct as f64),
        mean(&|r| r.top_share) * 100.0,
        mean(&|r| r.top_reputation_wins) * 100.0,
        mean(&|r| spread(&r.champion_points)),
    );
    if print_leagues {
        for row in rows.iter().take(8) {
            eprintln!(
                "{:>22}   {} ({} clubs): {} champions, top {:.0}%, best-rep {:.0}%, points {:?}",
                "",
                row.name,
                row.clubs,
                row.distinct,
                row.top_share * 100.0,
                row.top_reputation_wins * 100.0,
                row.champion_points
            );
        }
    }
}

/// Given the gate and the shipped world on several seeds,
/// When their twelve past seasons are generated,
/// Then the variety of the champions is printed per world, for the app's twelve seasons and for twenty.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_variety_of_a_generated_past() {
    for depth in [DEPTH, LONG_DEPTH] {
        for seed in 1..=5 {
            measure(
                &format!("gate seed {seed}"),
                WorldSpec::gate(seed),
                depth,
                seed == 1,
            );
        }
        for seed in 1..=5 {
            measure(
                &format!("shipped seed {seed}"),
                WorldSpec::shipped(seed),
                depth,
                seed == 1,
            );
        }
    }
}
