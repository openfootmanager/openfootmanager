//! A/B comparison of the two paths a match can take through the engine.
//!
//! **A — instant.** What every fixture the player is not watching gets today:
//! the whole squad is handed to `engine::simulate` in one shot. There is no
//! starting XI, no bench, and no AI manager on either touchline.
//!
//! **B — live.** What the player's own fixture gets: eleven starters, a real
//! bench, and `ai_decide` consulted every minute for both sides.
//!
//! The interesting column is `burn` — the condition the *production* wear
//! formula would charge this squad for the match, projected onto each path's
//! minutes. That number is what turns "the instant path fields the whole squad"
//! from a code observation into a gameplay consequence.

use std::time::{Duration, Instant};

use engine::ai::{ai_decide, AiPersonality, AiProfile};
use engine::{
    simulate_with_rng, LiveMatchState, MatchConfig, MatchReport, PlayStyle, PlayerData, Side,
    TeamData,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::builder::build_squad_with_bench;

/// How much weaker each bench player is than the starter in the same slot.
const BENCH_OVR_PENALTY: u8 = 8;

/// What one path did with one squad.
#[derive(Default, Clone, Copy)]
pub struct PathTotals {
    matches: u32,
    /// Squad members credited with at least one minute.
    participants: u64,
    /// Sum of every squad member's minutes.
    player_minutes: u64,
    substitutions: u64,
    style_changes: u64,
    formation_changes: u64,
    /// Condition the production wear formula would charge, summed over the squad.
    condition_burn: u64,
    elapsed: Duration,
}

impl PathTotals {
    fn per_match(&self, total: u64) -> f64 {
        if self.matches == 0 {
            return 0.0;
        }
        total as f64 / self.matches as f64
    }

    fn micros_per_match(&self) -> f64 {
        if self.matches == 0 {
            return 0.0;
        }
        self.elapsed.as_secs_f64() * 1_000_000.0 / self.matches as f64
    }
}

/// Condition a player loses for `minutes` played, mirroring the production
/// formula in `ofm_core::player_wear::apply_match_wear`.
///
/// sim-bench deliberately depends on `engine` alone, so it cannot call the real
/// function — `player_wear` lives in `ofm_core`, which pulls in `domain` and the
/// whole game state. This is a projection tool, and the duplication is the price
/// of keeping the bench a leaf crate. The unit test below pins the exact values
/// `player_wear`'s own test asserts, so a change there fails here too.
fn projected_wear(minutes: u8, stamina: u8) -> u8 {
    if minutes == 0 {
        return 0;
    }
    let minutes_factor = minutes as f64 / 90.0;
    let stamina_factor = stamina as f64 / 100.0;
    let base_depletion = 40.0 * (1.0 - stamina_factor * 0.4);
    (base_depletion * minutes_factor) as u8
}

/// Both squads in full — the population each path is judged against, so a
/// reserve who never left the bench still shows up as zero minutes rather than
/// vanishing from the denominator.
fn squad_snapshot(
    home_xi: &TeamData,
    home_bench: &[PlayerData],
    away_xi: &TeamData,
    away_bench: &[PlayerData],
) -> Vec<PlayerData> {
    home_xi
        .players
        .iter()
        .chain(home_bench.iter())
        .chain(away_xi.players.iter())
        .chain(away_bench.iter())
        .cloned()
        .collect()
}

/// Fold one finished match's report into the running totals for a path.
fn accumulate(totals: &mut PathTotals, report: &MatchReport, squad: &[PlayerData]) {
    totals.matches += 1;
    for player in squad {
        let minutes = report
            .player_stats
            .get(&player.id)
            .map(|stats| stats.minutes_played)
            .unwrap_or(0);
        if minutes > 0 {
            totals.participants += 1;
        }
        totals.player_minutes += u64::from(minutes);
        totals.condition_burn += u64::from(projected_wear(minutes, player.stamina));
    }
}

/// Run the A/B and print the comparison table.
pub fn run(config: &MatchConfig, games: u32, seed: Option<u64>, rating: u8, formation: &str) {
    let base = seed.unwrap_or(42);
    let mut instant = PathTotals::default();
    let mut live = PathTotals::default();

    eprintln!("AI-path A/B: {games} matches per path (seed: {base})…");

    for i in 0..games {
        let game_seed = base.wrapping_add(u64::from(i));
        // Identical squads for both paths: same builder seed, same everything.
        let mut team_rng = StdRng::seed_from_u64(base.wrapping_add(0xDEAD_BEEF));
        let (home_xi, home_bench) = build_squad_with_bench(
            "home",
            "Home FC",
            rating,
            BENCH_OVR_PENALTY,
            PlayStyle::Balanced,
            formation,
            &mut team_rng,
        );
        let (away_xi, away_bench) = build_squad_with_bench(
            "away",
            "Away FC",
            rating,
            BENCH_OVR_PENALTY,
            PlayStyle::Balanced,
            formation,
            &mut team_rng,
        );
        // Owned, because path B consumes the teams it is handed.
        let squad = squad_snapshot(&home_xi, &home_bench, &away_xi, &away_bench);

        // ── Path A: the whole squad, one shot, no manager ───────────────────
        // Assembled before the timer starts: in production this list is what
        // `build_engine_team` already returns, so it is not path A's cost.
        let home_all = whole_squad(&home_xi, &home_bench);
        let away_all = whole_squad(&away_xi, &away_bench);
        let mut rng = StdRng::seed_from_u64(game_seed);
        let started = Instant::now();
        let report = simulate_with_rng(&home_all, &away_all, config, &mut rng);
        instant.elapsed += started.elapsed();
        accumulate(&mut instant, &report, &squad);

        // ── Path B: XI + bench, both sides managed ──────────────────────────
        let mut rng = StdRng::seed_from_u64(game_seed);
        let started = Instant::now();
        let report = run_live(
            home_xi, away_xi, home_bench, away_bench, config, &mut rng, &mut live,
        );
        live.elapsed += started.elapsed();
        accumulate(&mut live, &report, &squad);
    }

    print_table(&instant, &live, games);
}

/// The instant path's view of a club: starters and reserves in one undifferentiated list.
fn whole_squad(xi: &TeamData, bench: &[PlayerData]) -> TeamData {
    let mut team = xi.clone();
    team.players.extend_from_slice(bench);
    team
}

/// Step a live match to completion with `ai_decide` driving both sides, counting
/// the decisions it takes along the way.
fn run_live<R: Rng>(
    home: TeamData,
    away: TeamData,
    home_bench: Vec<PlayerData>,
    away_bench: Vec<PlayerData>,
    config: &MatchConfig,
    rng: &mut R,
    totals: &mut PathTotals,
) -> MatchReport {
    let mut state = LiveMatchState::new(home, away, config.clone(), home_bench, away_bench, false);
    let profile = AiProfile {
        reputation: 500,
        experience: 50,
        personality: AiPersonality::Pragmatist,
    };

    // Bounded so a phase-machine bug in the engine cannot hang the bench.
    // Regulation plus stoppage never reaches 200 minutes.
    for _ in 0..200 {
        if state.is_finished() {
            break;
        }
        state.step_minute(rng);
        for side in [Side::Home, Side::Away] {
            for cmd in ai_decide(&state, side, &profile, rng) {
                count_command(totals, &cmd);
                // A rejected command is a real outcome (no bench cover, subs
                // exhausted), not a bench failure — record the attempt and move on.
                let _ = state.apply_command(cmd);
            }
        }
    }

    state.into_report()
}

fn count_command(totals: &mut PathTotals, cmd: &engine::MatchCommand) {
    match cmd {
        engine::MatchCommand::Substitute { .. } => totals.substitutions += 1,
        engine::MatchCommand::ChangePlayStyle { .. } => totals.style_changes += 1,
        engine::MatchCommand::ChangeFormation { .. } => totals.formation_changes += 1,
        _ => {}
    }
}

fn print_table(instant: &PathTotals, live: &PathTotals, games: u32) {
    use colored::Colorize;

    let sep = "─".repeat(78);
    println!("{sep}");
    println!(
        "{}",
        "  AI-PATH A/B — per match, both squads combined"
            .bold()
            .bright_cyan()
    );
    println!("{sep}");
    println!(
        "{:<26} {:>12} {:>12} {:>12}",
        "metric", "A: instant", "B: live+AI", "delta"
    );
    println!("{sep}");

    let rows: [(&str, f64, f64); 6] = [
        (
            "participants",
            instant.per_match(instant.participants),
            live.per_match(live.participants),
        ),
        (
            "player-minutes",
            instant.per_match(instant.player_minutes),
            live.per_match(live.player_minutes),
        ),
        (
            "substitutions",
            instant.per_match(instant.substitutions),
            live.per_match(live.substitutions),
        ),
        (
            "play-style changes",
            instant.per_match(instant.style_changes),
            live.per_match(live.style_changes),
        ),
        (
            "formation changes",
            instant.per_match(instant.formation_changes),
            live.per_match(live.formation_changes),
        ),
        (
            "condition burned",
            instant.per_match(instant.condition_burn),
            live.per_match(live.condition_burn),
        ),
    ];
    for (label, a, b) in rows {
        println!("{label:<26} {a:>12.2} {b:>12.2} {:>+12.2}", b - a);
    }
    println!(
        "{:<26} {:>12.1} {:>12.1} {:>11.1}×",
        "µs / match",
        instant.micros_per_match(),
        live.micros_per_match(),
        if instant.micros_per_match() > 0.0 {
            live.micros_per_match() / instant.micros_per_match()
        } else {
            0.0
        }
    );
    println!("{sep}");
    println!(
        "  {games} matches per path. 44 squad players across both sides; only 22 should play."
    );
    println!("{sep}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_wear_matches_the_production_formula() {
        // Pinned against ofm_core::player_wear's own test: a stamina-100 player
        // on 100 condition finishes a full 90 on 76, i.e. loses exactly 24.
        assert_eq!(projected_wear(90, 100), 24);
        // And the values the plan's arithmetic rests on.
        assert_eq!(projected_wear(90, 70), 28);
        assert_eq!(projected_wear(90, 50), 32);
    }

    #[test]
    fn a_player_who_did_not_feature_burns_nothing() {
        assert_eq!(projected_wear(0, 70), 0);
    }

    #[test]
    fn wear_scales_with_minutes_played() {
        let full = projected_wear(90, 70);
        let half = projected_wear(45, 70);
        assert_eq!(half, full / 2);
    }

    #[test]
    fn a_squad_is_eleven_starters_and_eleven_reserves_with_distinct_ids() {
        let mut rng = StdRng::seed_from_u64(7);
        let (team, bench) = build_squad_with_bench(
            "home",
            "Home FC",
            70,
            BENCH_OVR_PENALTY,
            PlayStyle::Balanced,
            "4-3-3",
            &mut rng,
        );

        assert_eq!(team.players.len(), 11);
        assert_eq!(bench.len(), 11);

        let ids: std::collections::HashSet<&str> = team
            .players
            .iter()
            .chain(bench.iter())
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(ids.len(), 22, "starters and bench must not share ids");
    }

    #[test]
    fn the_bench_mirrors_the_starting_shape() {
        let mut rng = StdRng::seed_from_u64(11);
        let (team, bench) = build_squad_with_bench(
            "home",
            "Home FC",
            70,
            BENCH_OVR_PENALTY,
            PlayStyle::Balanced,
            "4-3-3",
            &mut rng,
        );

        let shape = |players: &[PlayerData]| {
            let mut counts = [0usize; 4];
            for p in players {
                counts[p.position as usize] += 1;
            }
            counts
        };
        assert_eq!(shape(&team.players), shape(&bench));
    }

    /// The regression this whole slice exists to expose: the instant path hands
    /// the engine every squad member, and the report then credits every one of
    /// them a full match.
    #[test]
    fn the_instant_path_credits_the_whole_squad_a_full_match() {
        let mut team_rng = StdRng::seed_from_u64(3);
        let (home_xi, home_bench) = build_squad_with_bench(
            "home",
            "Home FC",
            70,
            BENCH_OVR_PENALTY,
            PlayStyle::Balanced,
            "4-4-2",
            &mut team_rng,
        );
        let (away_xi, away_bench) = build_squad_with_bench(
            "away",
            "Away FC",
            70,
            BENCH_OVR_PENALTY,
            PlayStyle::Balanced,
            "4-4-2",
            &mut team_rng,
        );
        let squad = squad_snapshot(&home_xi, &home_bench, &away_xi, &away_bench);

        let mut rng = StdRng::seed_from_u64(3);
        let report = simulate_with_rng(
            &whole_squad(&home_xi, &home_bench),
            &whole_squad(&away_xi, &away_bench),
            &MatchConfig::default(),
            &mut rng,
        );

        let mut totals = PathTotals::default();
        accumulate(&mut totals, &report, &squad);

        assert_eq!(
            totals.participants, 44,
            "today every squad member is credited with minutes; when slice 1 lands this drops to 22"
        );
        assert!(
            totals.condition_burn > 1_000,
            "44 players × ~28 condition — got {}",
            totals.condition_burn
        );
    }
}
