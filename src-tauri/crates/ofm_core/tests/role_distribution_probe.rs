//! What roles does a real generated world actually hand out?
//!
//! ```text
//! cargo test -p ofm_core --test role_distribution_probe --release -- --ignored --nocapture
//! ```
//!
//! `ai_roles` decides by *shape* — a specialist role is taken only when a player
//! scores better at that role's three attributes than at his own average, by a
//! fixed margin. Every test of that logic uses hand-built players with one
//! attribute spiked, which proves the rule and says nothing at all about the
//! margin. Two ways it can be wrong against real squads, and both look like
//! success from inside a unit test:
//!
//! - **Too high.** Generated attributes are correlated enough that almost nobody
//!   clears it, squads come out all-`Standard`, and the feature is invisible —
//!   which is the complaint this work exists to answer.
//! - **Too low.** Everybody clears it, every squad is wall-to-wall specialists,
//!   and `Standard` stops meaning anything.
//!
//! It also reports the share per position group, because the baseline a player
//! is measured against is not identical across them: an outfielder's average
//! includes shooting, tackling, defending and aggression, and the generator does
//! not hand those out evenly to a forward and a centre-half. If one group is
//! wildly more specialised than another, that skew is the reason.

use ofm_core::generator;

/// A fixed seed, so the numbers below are the same numbers next time somebody
/// changes `SPECIALIST_MARGIN` and wants to know what it did.
fn seed() -> u64 {
    std::env::var("OFM_PROBE_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20260820)
}

#[test]
#[ignore = "generates whole clubs; run explicitly"]
fn report_how_many_players_get_a_job_title() {
    let (teams, players, _staff) = generator::generate_world_seeded(
        seed(),
        &generator::definitions::DefinitionSources::embedded_only(),
    );

    let mut squads = 0usize;
    let mut players_total = 0usize;
    let mut specialists = 0usize;
    let mut per_group: Vec<(&str, usize, usize)> = vec![
        ("Goalkeeper", 0, 0),
        ("Defender", 0, 0),
        ("Midfielder", 0, 0),
        ("Forward", 0, 0),
    ];
    let mut per_role: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut all_standard_squads = 0usize;

    for team in &teams {
        let squad: Vec<_> = players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team.id.as_str()))
            .collect();
        if squad.is_empty() {
            continue;
        }
        squads += 1;
        players_total += squad.len();
        let named = squad
            .iter()
            .filter(|p| team.player_roles.contains_key(&p.id))
            .count();
        specialists += named;
        if named == 0 {
            all_standard_squads += 1;
        }

        for player in &squad {
            let group = match player.position.to_group_position() {
                domain::player::Position::Goalkeeper => 0,
                domain::player::Position::Defender => 1,
                domain::player::Position::Midfielder => 2,
                _ => 3,
            };
            per_group[group].1 += 1;
            if let Some(role) = team.player_roles.get(&player.id) {
                per_group[group].2 += 1;
                *per_role.entry(format!("{role:?}")).or_default() += 1;
            }
        }
    }

    println!();
    println!("Roles handed out across {squads} generated squads, {players_total} players.");
    println!();
    println!(
        "specialists: {specialists} of {players_total} ({:.1}%)",
        100.0 * specialists as f64 / players_total.max(1) as f64
    );
    println!("squads with nobody named at all: {all_standard_squads}");
    println!();
    println!("{:<12} {:>8} {:>12}", "group", "players", "specialist%");
    println!("{}", "-".repeat(34));
    for (name, total, named) in &per_group {
        println!(
            "{:<12} {:>8} {:>11.1}%",
            name,
            total,
            100.0 * *named as f64 / (*total).max(1) as f64
        );
    }
    println!();
    println!("{:<22} {:>8}", "role", "count");
    println!("{}", "-".repeat(32));
    let mut rows: Vec<(&String, &usize)> = per_role.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (role, count) in rows {
        println!("{role:<22} {count:>8}");
    }
    println!();
    println!(
        "Neither extreme is the goal: a squad should read as a few players with a \
         job and a majority without one."
    );
    println!();
}
