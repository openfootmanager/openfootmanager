//! Measurement probe: how often a board turns its youth intake away on wages, on
//! a real pyramid world played season after season.
//!
//! Run in release; it plays whole seasons of a three-nation world:
//!
//! ```text
//! cargo test --release -p ofm_core --test youth_intake_wages_probe -- --ignored --nocapture
//! ```
//!
//! What it reads at each rollover, per club: how many youngsters the intake
//! planned, how many joined, and so how many the board refused; whether the
//! club's wage bill was over its budget and its cash below zero. Then how long
//! the runs of seasons were in which a club took nobody.

use std::collections::{HashMap, HashSet};

use domain::manager::Manager;
use domain::player::SquadRole;
use domain::stats::StatsState;
use ofm_core::career::{CareerScope, begin_career};
use ofm_core::clock::GameClock;
use ofm_core::end_of_season;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, default_opening_year, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::world::start_date_for_year;

const NATIONS: &[&str] = &["ENG", "PT", "AR"];
const SEEDS: [u64; 3] = [7, 19, 42];
const SEASONS: u32 = 5;

fn pyramid_world(seed: u64) -> Game {
    let sources = DefinitionSources::embedded_only();
    let mut config = WorldGenConfig::standard_from(&sources);
    config
        .nations
        .retain(|nation| NATIONS.contains(&nation.code.as_str()));
    let world = generate_world_data_seeded_with(seed, &config, &sources);
    // A generated world opens in the calendar year it is made in, so the
    // career opens there too, with every contract dated against it.
    let start = start_date_for_year(default_opening_year() as i32).expect("a valid start year");
    let manager = Manager::new(
        "probe-manager".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut game = Game::new(
        GameClock::new(start),
        manager,
        world.teams,
        world.players,
        world.staff,
        vec![],
    );
    game.seed = seed;
    game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());
    repair_opening_youth_academies(&mut game);
    let club = game
        .teams
        .iter()
        .filter(|team| team.football_nation == "ENG")
        .max_by(|a, b| {
            a.reputation
                .cmp(&b.reputation)
                .then_with(|| b.id.cmp(&a.id))
        })
        .map(|team| team.id.clone())
        .expect("an English club");
    begin_career(
        &mut game,
        &club,
        CareerScope::default(),
        StatsState::default(),
    )
    .expect("the career begins");
    game
}

/// One club's intake at one rollover.
#[derive(Debug, Clone)]
struct Taken {
    planned: usize,
    joined: usize,
    over_budget: bool,
    in_debt: bool,
}

fn rollover(game: &mut Game, season: u32) -> HashMap<String, Taken> {
    let before: HashSet<String> = game.players.iter().map(|p| p.id.clone()).collect();
    let mut days = 0;
    while !end_of_season::is_season_complete(game) {
        days += 1;
        assert!(days < 600, "season {season} never finished");
        // Keep the player in a job, as the season harness does: a sacking is not
        // what this measures.
        game.manager.satisfaction = game.manager.satisfaction.max(60);
        game.manager.warning_stage = 0;
        ofm_core::turn::process_day(game);
    }
    let bills: HashMap<String, (bool, bool)> = game
        .teams
        .iter()
        .map(|team| {
            let bill = ofm_core::finances::calc_wages(game, &team.id);
            (team.id.clone(), (bill > team.wage_budget, team.finance < 0))
        })
        .collect();
    print_margins(game, season);
    game.manager.satisfaction = game.manager.satisfaction.max(60);
    game.manager.warning_stage = 0;
    end_of_season::advance_to_next_season(game).expect("the season rolls over");
    let rollover_day = game.clock.current_date.format("%Y-%m-%d").to_string();

    game.teams
        .iter()
        .map(|team| {
            let academy: Vec<_> = game
                .players
                .iter()
                .filter(|p| {
                    p.squad_role == SquadRole::Youth
                        && p.contract_club_id() == Some(team.id.as_str())
                })
                .collect();
            let is_newcomer = |p: &&domain::player::Player| {
                !before.contains(&p.id) && p.contract_start() == Some(rollover_day.as_str())
            };
            let joined = academy.iter().filter(|p| is_newcomer(p)).count();
            // The intake is the last squad step of the rollover, so the academy it
            // planned for is the one there now, less those who joined.
            let planned = ofm_core::youth_intake::plan_for(
                academy.iter().copied().filter(|p| !is_newcomer(p)),
            )
            .groups
            .len();
            let (over_budget, in_debt) = bills.get(&team.id).copied().unwrap_or_default();
            (
                team.id.clone(),
                Taken {
                    planned,
                    joined,
                    over_budget,
                    in_debt,
                },
            )
        })
        .collect()
}

/// How much more a week each club's board would let its wage bill grow, just
/// before the rollover: the margin a recruit's wage has to fit in. Found by
/// asking the board's own rule, a step at a time.
fn print_margins(game: &Game, season: u32) {
    let mut margins: Vec<i64> = game
        .teams
        .iter()
        .map(|team| {
            let bill = ofm_core::finances::calc_wages(game, &team.id);
            let allows = |extra: i64| {
                ofm_core::contract_wage_policy::wage_policy_allows_projection(
                    team,
                    bill,
                    bill + extra,
                )
            };
            let (mut low, mut high) = (0i64, 1i64);
            while allows(high) && high < 1 << 40 {
                low = high;
                high *= 2;
            }
            while high - low > 1 {
                let mid = (low + high) / 2;
                if allows(mid) { low = mid } else { high = mid }
            }
            low
        })
        .collect();
    margins.sort_unstable();
    let budgets = game
        .teams
        .iter()
        .map(|team| team.wage_budget)
        .min()
        .unwrap_or(0);
    let under_three_minimums = margins.iter().filter(|&&margin| margin < 1_500).count();
    println!(
        "  season {season} margins/week: min={} p10={} median={} | smallest wage budget={budgets} |          clubs with room for fewer than three minimum recruits={under_three_minimums}",
        margins[0],
        margins[margins.len() / 10],
        margins[margins.len() / 2]
    );
}

#[test]
#[ignore = "measurement probe: run in release with --ignored --nocapture"]
fn probe_how_often_boards_turn_the_intake_away() {
    for seed in SEEDS {
        let mut game = pyramid_world(seed);
        let clubs = game.teams.len();
        let started = std::time::Instant::now();
        let mut zero_runs: HashMap<String, u32> = HashMap::new();
        let mut longest: HashMap<String, u32> = HashMap::new();
        for season in 1..=SEASONS {
            let taken = rollover(&mut game, season);
            let planned: usize = taken.values().map(|t| t.planned).sum();
            let joined: usize = taken.values().map(|t| t.joined).sum();
            let refusing = taken.values().filter(|t| t.joined < t.planned).count();
            let empty: Vec<&Taken> = taken.values().filter(|t| t.joined == 0).collect();
            let empty_over_budget = empty.iter().filter(|t| t.over_budget).count();
            let empty_in_debt = empty.iter().filter(|t| t.in_debt).count();
            for (club, t) in &taken {
                let run = zero_runs.entry(club.clone()).or_default();
                *run = if t.joined == 0 { *run + 1 } else { 0 };
                let best = longest.entry(club.clone()).or_default();
                *best = (*best).max(*run);
            }
            println!(
                "seed {seed} season {season}: clubs={clubs} planned={planned} joined={joined} \
                 refused={} ({:.1}%) clubs_refusing={refusing} clubs_taking_none={} \
                 (over_budget={empty_over_budget} in_debt={empty_in_debt}) t={:?}",
                planned - joined,
                100.0 * (planned - joined) as f64 / planned.max(1) as f64,
                empty.len(),
                started.elapsed()
            );
        }
        let mut streaks: Vec<u32> = longest.values().copied().collect();
        streaks.sort_unstable();
        let histogram: Vec<(u32, usize)> = (0..=SEASONS)
            .map(|n| (n, streaks.iter().filter(|&&s| s == n).count()))
            .filter(|(_, count)| *count > 0)
            .collect();
        println!(
            "seed {seed}: longest run of seasons taking nobody, clubs per length: {histogram:?}"
        );
    }
}
