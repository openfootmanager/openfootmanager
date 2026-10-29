//! What an answer does beyond the player's own trust: how much of a morale gain an unresolved
//! issue lets through, how a player's own personality colours a hard word, the dressing room's
//! view of a refused renewal, and the new manager's first contract briefing.

use super::*;
use domain::player::{PlayerIssue, PlayerIssueCategory};

fn morale_of(game: &Game, id: &str) -> u8 {
    game.players.iter().find(|p| p.id == id).unwrap().morale
}

/// p_fwd0's morale after promising him a chance in answer to a bench complaint, with the
/// world seed `seed` and an unresolved playing-time issue of `severity` (none at 0).
fn morale_after_promised_chance(seed: u64, severity: u8) -> u8 {
    let mut game = make_game();
    game.seed = seed;
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 40;
    if severity > 0 {
        player.morale_core.unresolved_issue = Some(PlayerIssue {
            category: PlayerIssueCategory::PlayingTime,
            severity,
        });
    }
    inject_player_message(&mut game, "bench_complaint_p_fwd0", "p_fwd0", "respond");
    player_events::apply_player_response(
        &mut game,
        "bench_complaint_p_fwd0",
        "respond",
        "promise_chance",
    )
    .expect("the answer is applied");
    morale_of(&game, "p_fwd0")
}

/// Given a benched player with an unresolved issue of severity 60, and the same player with one of
///       severity 40, over ten seeds
/// When the manager promises him a chance (a gain of 8 to 14 without the issue)
/// Then the middling issue halves the gain, rounding up, and the milder one lets it all through
#[test]
fn an_unresolved_issue_of_middling_severity_halves_a_morale_gain_rounding_up() {
    let mut saw_an_odd_gain = false;
    for seed in 0..10 {
        let gain = morale_after_promised_chance(seed, 0) - 40;
        saw_an_odd_gain |= gain % 2 == 1;

        assert_eq!(
            morale_after_promised_chance(seed, 60) - 40,
            (gain + 1) / 2,
            "seed {seed}"
        );
        assert_eq!(
            morale_after_promised_chance(seed, 40) - 40,
            gain,
            "seed {seed}"
        );
    }
    assert!(saw_an_odd_gain, "an odd gain is where rounding up shows");
}

/// p_fwd0's morale after "higher expectations" in answer to his happiness, with the world seed
/// `seed` and the given composure, leadership and aggression.
fn morale_after_higher_expectations(
    seed: u64,
    composure: u8,
    leadership: u8,
    aggression: u8,
) -> i16 {
    let mut game = make_game();
    game.seed = seed;
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.morale = 50;
    player.attributes.composure = composure;
    player.attributes.leadership = leadership;
    player.attributes.aggression = aggression;
    inject_player_message(&mut game, "happy_player_p_fwd0", "p_fwd0", "respond");
    player_events::apply_player_response(
        &mut game,
        "happy_player_p_fwd0",
        "respond",
        "higher_expectations",
    )
    .expect("the answer is applied");
    i16::from(morale_of(&game, "p_fwd0"))
}

/// Given the same happy player, once as a composed leader (personality +20) and once volatile
///       (−9), over twenty seeds
/// When the manager raises his expectations
/// Then the leader always ends 9 morale above the volatile one: a third of the 29 points between
///      their personalities, his own and no one else's
#[test]
fn higher_expectations_lands_better_on_a_composed_player_by_the_personality_margin() {
    for seed in 0..20 {
        let composed = morale_after_higher_expectations(seed, 95, 95, 20);
        let volatile = morale_after_higher_expectations(seed, 20, 20, 95);

        assert_eq!(composed - volatile, 9, "seed {seed}");
    }
}

fn refuse_renewal(game: &mut Game) -> player_events::PlayerResponseEffect {
    inject_player_message(game, "contract_concern_p_fwd0", "p_fwd0", "respond");
    player_events::apply_player_response(game, "contract_concern_p_fwd0", "respond", "no_renewal")
        .expect("the answer is applied")
}

/// Given the user's squad of eleven at morale 70, and a team2 player at 70
/// When the manager tells p_fwd0 there will be no renewal
/// Then each of his ten teammates drops by 2 to 5, and the other club's player does not move
#[test]
fn refusing_a_renewal_costs_each_teammate_two_to_five_morale_and_nobody_else() {
    let mut game = make_game();
    game.players.push(make_player(
        "elsewhere",
        "Elsewhere",
        "team2",
        Position::Midfielder,
    ));

    refuse_renewal(&mut game);

    for player in game.players.iter().filter(|p| p.id != "p_fwd0") {
        if player.id == "elsewhere" {
            assert_eq!(player.morale, 70, "another club's player");
        } else {
            assert!(
                (65..=68).contains(&player.morale),
                "{}: {}",
                player.id,
                player.morale
            );
        }
    }
}

/// Given the user's squad of eleven
/// When the manager refuses p_fwd0 a renewal
/// Then the answer reports the dressing room, and the ten teammates it upset
#[test]
fn refusing_a_renewal_reports_the_teammates_it_upset() {
    let mut game = make_game();

    let effect = refuse_renewal(&mut game);

    assert_eq!(
        (
            effect.i18n_key.as_str(),
            effect.i18n_params.get("affected").map(String::as_str)
        ),
        (
            "be.msg.playerEvent.effects.contractConcern.noRenewalWithDressingRoom",
            Some("10")
        )
    );
}

/// Given a squad of one, p_fwd0
/// When the manager refuses him a renewal
/// Then the answer reports no dressing room: there is nobody to upset
#[test]
fn refusing_a_lone_players_renewal_reports_no_dressing_room() {
    let mut game = make_game();
    game.players.retain(|p| p.id == "p_fwd0");

    let effect = refuse_renewal(&mut game);

    assert_eq!(
        (effect.i18n_key.as_str(), effect.i18n_params.get("affected")),
        ("be.msg.playerEvent.effects.contractConcern.noRenewal", None)
    );
}

/// Given a new manager whose squad has contracts ending in 20 days, 150 days and 330 days
/// When the takeover contract review is written
/// Then it counts three expiring this season, two of them urgent, one in its final weeks, and
///      leads with the final-month body
#[test]
fn takeover_contract_review_counts_urgent_and_final_week_contracts() {
    let mut game = make_game();
    for (id, end) in [
        ("p_fwd0", "2025-07-05"),
        ("p_fwd1", "2025-11-12"),
        ("p_mid0", "2026-05-11"),
    ] {
        game.players
            .iter_mut()
            .find(|p| p.id == id)
            .unwrap()
            .stage_contract_end(Some(end.to_string()));
    }

    player_events::generate_takeover_contract_review_message(&mut game);

    let review = game
        .messages
        .iter()
        .find(|m| m.id.starts_with("contract_review_takeover_"))
        .expect("a review");
    let param = |key: &str| review.i18n_params.get(key).map(String::as_str);
    assert_eq!(
        (
            review.body_key.as_deref(),
            param("totalExpiring"),
            param("urgentContracts"),
            param("finalMonthContracts")
        ),
        (
            Some("be.msg.takeoverContractReview.bodyFinalMonth"),
            Some("3"),
            Some("2"),
            Some("1")
        )
    );
}
