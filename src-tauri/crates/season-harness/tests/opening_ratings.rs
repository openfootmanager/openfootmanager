//! A new world's players open at the rating the game computes for them (#869).
//!
//! A generated player is rated at his coarse slot position (Defender, Midfielder, Forward) and
//! only later given a specific natural position (RightBack, LeftWinger, ...). The daily rating
//! refresh rates him at that specific position, which is on average 1.4 points higher, so every
//! rating used to jump on the first day. These tests pin that the opening world is already what
//! the refresh would make it, and that it still has the tiers, mean and wonderkids the maintainer
//! set.

use domain::age::age_on_or_unknown;
use ofm_core::player_rating::natural_ovr;
use season_harness::distribution::distribution_report;
use season_harness::worlds::WorldSpec;

/// Given the shipped world on its opening day,
/// When each player's stored overall is compared with the rating formula's for his natural
///      position,
/// Then they are the same for every player.
#[test]
fn every_player_opens_at_the_rating_the_game_computes_for_him() {
    for seed in [1, 2] {
        let game = WorldSpec::shipped(seed).build().expect("the world builds");
        let on = game.clock.current_date.date_naive();
        let differing: Vec<String> = game
            .players
            .iter()
            .filter(|player| !player.retired)
            .filter(|player| natural_ovr(player).round() as u8 != player.ovr)
            .take(5)
            .map(|player| {
                format!(
                    "{} age {} stored {} formula {:.1}",
                    player.id,
                    age_on_or_unknown(on, &player.date_of_birth),
                    player.ovr,
                    natural_ovr(player)
                )
            })
            .collect();

        assert!(
            differing.is_empty(),
            "seed {seed}: stored ratings differ from the formula's, e.g. {differing:?}"
        );
    }
}

/// Given the shipped world on its opening day,
/// When the whole world is read,
/// Then it opens at the ratings the game really computes: senior S 1 to 3%, A 8 to 11% and a mean
///      of 69 to 70.2 (the figures a world showed on day 1 before, S 1.2%, A 6.5% and 68.2, were
///      1.4 points low), and the wonderkids no more common than the maintainer's rare rate
///      (develop opened at 0.34% and 0.29% on these seeds). The settled world, which is what the
///      tier targets are about, is unchanged: this only stops the opening from understating it.
#[test]
fn the_opening_world_keeps_its_tiers_mean_and_rare_wonderkids() {
    for seed in [1, 2] {
        let game = WorldSpec::shipped(seed).build().expect("the world builds");
        let world = distribution_report(&game);
        let message = format!(
            "seed {seed}: S {:.2}% A {:.2}% mean {:.2} wonderkids {:.2}%",
            world.senior_ovr_s_share * 100.0,
            world.senior_ovr_a_share * 100.0,
            world.mean_senior_ovr,
            world.wonderkid_share * 100.0
        );
        eprintln!("OPENING {message}");

        assert!(
            (0.01..=0.03).contains(&world.senior_ovr_s_share),
            "S share; {message}"
        );
        assert!(
            (0.08..=0.11).contains(&world.senior_ovr_a_share),
            "A share; {message}"
        );
        assert!(
            (69.0..=70.2).contains(&world.mean_senior_ovr),
            "mean rating; {message}"
        );
        assert!(
            (0.0015..=0.0035).contains(&world.wonderkid_share),
            "wonderkids; {message}"
        );
    }
}
