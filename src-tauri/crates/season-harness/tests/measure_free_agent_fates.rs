//! What becomes of the players who are free agents on a new world's opening day (#869).
//!
//! Measurement only, `#[ignore]`d and run in the cloud in release mode:
//!
//! `cargo test --release -p season-harness --test measure_free_agent_fates -- --ignored --nocapture`
//!
//! The opening free agents (the unattached pool and the national-team top-ups) are followed by id.
//! After seasons 1, 5 and 10 the line says how many have signed with a club, retired or are still
//! free, how many of them are S (rating 81 or more) or A (77 to 80) players, and what share of the
//! world's S and A players they are.

use std::collections::HashSet;

use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::WorldSpec;

const S_FROM: u8 = 81;
const A_FROM: u8 = 77;

struct Cohort {
    ids: HashSet<String>,
}

impl SeasonObserver for Cohort {
    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, season: u32) {
        if ![1, 5, 10].contains(&season) {
            return;
        }
        let (mut signed, mut retired, mut free, mut gone) = (0, 0, 0, 0);
        let (mut cohort_s, mut cohort_a, mut world_s, mut world_a) = (0, 0, 0, 0);
        for player in game.players.iter().filter(|player| !player.retired) {
            let (is_s, is_a) = (player.ovr >= S_FROM, (A_FROM..S_FROM).contains(&player.ovr));
            world_s += usize::from(is_s);
            world_a += usize::from(is_a);
            if self.ids.contains(&player.id) {
                cohort_s += usize::from(is_s);
                cohort_a += usize::from(is_a);
            }
        }
        for id in &self.ids {
            match game.players.iter().find(|player| &player.id == id) {
                None => gone += 1,
                Some(player) if player.retired => retired += 1,
                Some(player) if player.team_id.is_some() => signed += 1,
                Some(_) => free += 1,
            }
        }
        eprintln!(
            "after season {season}: opening free agents {} -> signed {signed}, still free {free}, retired {retired}, removed {gone}; they hold S {cohort_s} of {world_s} ({:.1}%) and A {cohort_a} of {world_a} ({:.1}%) of the world's",
            self.ids.len(),
            100.0 * cohort_s as f64 / world_s.max(1) as f64,
            100.0 * cohort_a as f64 / world_a.max(1) as f64,
        );
    }
}

#[test]
#[ignore = "measurement: prints, asserts nothing"]
fn measure_what_the_opening_free_agents_become() {
    let mut game = WorldSpec::shipped(1).build().expect("the world builds");
    let ids: HashSet<String> = game
        .players
        .iter()
        .filter(|player| player.team_id.is_none() && !player.retired)
        .map(|player| player.id.clone())
        .collect();
    let (s, a) = ids.iter().fold((0, 0), |(s, a), id| {
        let player = game.players.iter().find(|player| &player.id == id);
        let ovr = player.map_or(0, |player| player.ovr);
        (
            s + usize::from(ovr >= S_FROM),
            a + usize::from((A_FROM..S_FROM).contains(&ovr)),
        )
    });
    eprintln!("opening: free agents {} (S {s}, A {a})", ids.len());
    let mut cohort = Cohort { ids };
    run_seasons(&mut game, RunOptions::seasons(10), &mut cohort).expect("the run completes");
}
