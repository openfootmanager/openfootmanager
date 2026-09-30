use super::super::domestic::{
    apply_domestic_berth_promotion_relegation, resolve_domestic_berth_fields,
};
use super::super::test_support::*;
use super::*;
use domain::league::{
    Berth, BerthRule, CompetitionFormat, CompetitionScope, CompetitionType, Fixture,
    FixtureCompetition, FixtureStatus, KnockoutRoundState, League, MatchResult,
};
use std::collections::HashSet;

#[test]
fn apply_pyramid_excludes_berth_targets_and_leaves_plain_leagues_alone() {
    // Country BR: Central (berth-fed) sits above a regional feeder and a
    // tier below that feeder. Without exclusion the linear chain would
    // swap Central's bottom club with the feeder's champion.
    let mut north = division(
        "north",
        1,
        "BR",
        &[
            ("n1", 60),
            ("n2", 50),
            ("n3", 40),
            ("n4", 30),
            ("n5", 20),
            ("n6", 10),
        ],
    );
    north.berths = vec![position_berth("central", 1, 2)];
    let central = division(
        "central",
        0,
        "BR",
        &[
            ("c1", 60),
            ("c2", 50),
            ("c3", 40),
            ("c4", 30),
            ("c5", 20),
            ("c6", 10),
        ],
    );
    let interior = division(
        "interior",
        2,
        "BR",
        &[
            ("i1", 60),
            ("i2", 50),
            ("i3", 40),
            ("i4", 30),
            ("i5", 20),
            ("i6", 10),
        ],
    );
    // Country ENG: a plain two-tier pyramid with no incoming berths.
    let eng_top = division(
        "eng-1",
        0,
        "ENG",
        &[
            ("t1", 60),
            ("t2", 50),
            ("t3", 40),
            ("t4", 30),
            ("t5", 20),
            ("t6", 10),
        ],
    );
    let eng_second = division(
        "eng-2",
        1,
        "ENG",
        &[
            ("s1", 60),
            ("s2", 50),
            ("s3", 40),
            ("s4", 30),
            ("s5", 20),
            ("s6", 10),
        ],
    );

    let central_before = central.participant_ids.clone();
    let mut competitions = vec![central, north, interior, eng_top, eng_second];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);

    assert_eq!(
        by_id("central").participant_ids,
        central_before,
        "a berth-fed Central League must be excluded from linear P/R"
    );
    // Feeder vs the tier below it still swaps (6-club → one up / one down).
    let north_ids: HashSet<&String> = by_id("north").participant_ids.iter().collect();
    let interior_ids: HashSet<&String> = by_id("interior").participant_ids.iter().collect();
    assert!(
        north_ids.contains(&"i1".to_string()),
        "feeder still receives the lower-tier champion: {:?}",
        by_id("north").participant_ids
    );
    assert!(
        interior_ids.contains(&"n6".to_string()),
        "lower tier still receives the feeder's bottom club: {:?}",
        by_id("interior").participant_ids
    );
    assert!(!north_ids.contains(&"n6".to_string()));
    assert!(!interior_ids.contains(&"i1".to_string()));

    // No-regression: a country without incoming berths still promotes.
    let eng1: HashSet<&String> = by_id("eng-1").participant_ids.iter().collect();
    let eng2: HashSet<&String> = by_id("eng-2").participant_ids.iter().collect();
    assert!(
        eng1.contains(&"s1".to_string()),
        "plain top division promotes"
    );
    assert!(
        !eng1.contains(&"t6".to_string()),
        "plain top division relegates"
    );
    assert!(eng2.contains(&"t6".to_string()));
    assert!(!eng2.contains(&"s1".to_string()));
}

/// A tier that leaves the ladder leaves a gap in it. A berth-fed division
/// exchanges clubs through its berths, so the tiers above and below it are
/// two divisions apart — closing that gap promoted a fourth-division
/// champion into the first.
#[test]
fn a_berth_fed_middle_tier_does_not_make_its_neighbours_adjacent() {
    let d1 = division(
        "d1",
        0,
        "XX",
        &[("a1", 40), ("a2", 30), ("a3", 20), ("a4", 10)],
    );
    let d2 = division(
        "d2",
        1,
        "XX",
        &[("b1", 40), ("b2", 30), ("b3", 20), ("b4", 10)],
    );
    let mut north = division("north", 2, "XX", &[("n1", 20), ("n2", 10)]);
    north.berths = vec![position_berth("d2", 1, 1)];
    let mut south = division("south", 3, "XX", &[("s1", 20), ("s2", 10)]);
    south.berths = vec![position_berth("d2", 1, 1)];
    let d4 = division(
        "d4",
        4,
        "XX",
        &[("e1", 40), ("e2", 30), ("e3", 20), ("e4", 10)],
    );
    let (before1, before4) = (d1.participant_ids.clone(), d4.participant_ids.clone());

    let mut competitions = vec![d1, d2, north, south, d4];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    assert_eq!(
        by_id("d1").participant_ids,
        before1,
        "the top flight must not reach past the berth-fed tier: {:?}",
        by_id("d1").participant_ids
    );
    assert_eq!(
        by_id("d4").participant_ids,
        before4,
        "the bottom tier must not reach past the berth-fed tier: {:?}",
        by_id("d4").participant_ids
    );
}

/// A single feeder into a middle tier is an ordinary ladder edge written as
/// data, but the tier below it is still two divisions from the top.
#[test]
fn a_sole_feeder_berth_into_a_middle_tier_keeps_the_top_flight_off_the_third() {
    let d1 = division(
        "d1",
        0,
        "XX",
        &[("a1", 40), ("a2", 30), ("a3", 20), ("a4", 10)],
    );
    let d2 = division(
        "d2",
        1,
        "XX",
        &[("b1", 40), ("b2", 30), ("b3", 20), ("b4", 10)],
    );
    let mut d3 = division(
        "d3",
        2,
        "XX",
        &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
    );
    d3.berths = vec![position_berth("d2", 1, 1)];
    let before1 = d1.participant_ids.clone();

    let mut competitions = vec![d1, d2, d3];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    assert!(
        !by_id("d1").participant_ids.contains(&"c1".to_string()),
        "a third-division club cannot reach the first: {:?}",
        by_id("d1").participant_ids
    );
    assert_eq!(by_id("d1").participant_ids, before1);
}

/// Only a country's own league competition is a rung. A regional table and
/// a cup scored as a table both belong to a country and are both league
/// tables, but the ladder must never promote into or out of either.
#[test]
fn a_regional_table_and_a_cup_played_as_a_table_are_not_rungs() {
    let d1 = division(
        "d1",
        0,
        "XX",
        &[("a1", 40), ("a2", 30), ("a3", 20), ("a4", 10)],
    );
    let mut regional = division("regional", 1, "XX", &[("r1", 40), ("r2", 30)]);
    regional.scope = CompetitionScope::Regional;
    let mut cup_table = division("cup-table", 2, "XX", &[("k1", 40), ("k2", 30)]);
    cup_table.kind = CompetitionType::Cup;
    let d2 = division(
        "d2",
        3,
        "XX",
        &[("b1", 40), ("b2", 30), ("b3", 20), ("b4", 10)],
    );
    let before: Vec<(String, Vec<String>)> = [&d1, &regional, &cup_table, &d2]
        .iter()
        .map(|c| (c.id.clone(), c.participant_ids.clone()))
        .collect();

    let mut competitions = vec![d1, regional, cup_table, d2];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    for (id, expected) in &before {
        if id == "d1" || id == "d2" {
            continue; // these two are a real pyramid and do exchange
        }
        assert_eq!(
            &by_id(id).participant_ids,
            expected,
            "{id} is not a rung and must not exchange clubs"
        );
    }
    // And the two real divisions still swap across the non-rungs between
    // them, because a non-rung is not a gap in the ladder — it was never on it.
    assert!(
        by_id("d1").participant_ids.contains(&"b1".to_string()),
        "the second division's champion still goes up: {:?}",
        by_id("d1").participant_ids
    );
}

#[test]
fn apply_pyramid_does_not_swap_sibling_position_range_feeders() {
    // Two same-priority regional groups that both send PositionRange
    // berths to the same Central League must not linearly swap — that
    // would put a promoted club on two tables after the berth merge.
    //
    // The Central League itself is in the slice, as it is in production:
    // a target is only a promotion destination if it is a domestic league
    // table this pass can see, so a berth pointing at nothing leaves its
    // feeders on the ladder rather than silently detaching them.
    let central = division(
        "central",
        0,
        "BR",
        &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
    );
    // Distinct priorities on purpose. Declared at the same rank the two
    // groups are peers, and the distinct-rank guard would refuse to swap
    // them whether or not the sibling rule existed — so this test would
    // pass with the sibling rule deleted and prove nothing.
    let mut north = division(
        "north",
        1,
        "BR",
        &[("n1", 40), ("n2", 30), ("n3", 20), ("n4", 10)],
    );
    north.berths = vec![position_berth("central", 1, 2)];
    let mut south = division(
        "south",
        2,
        "BR",
        &[("s1", 40), ("s2", 30), ("s3", 20), ("s4", 10)],
    );
    south.berths = vec![position_berth("central", 1, 2)];
    let north_before = north.participant_ids.clone();
    let south_before = south.participant_ids.clone();

    let mut competitions = vec![central, north, south];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    assert_eq!(
        by_id("north").participant_ids,
        north_before,
        "sibling feeders must not linearly swap: {:?}",
        by_id("north").participant_ids
    );
    assert_eq!(
        by_id("south").participant_ids,
        south_before,
        "sibling feeders must not linearly swap: {:?}",
        by_id("south").participant_ids
    );
}

/// A league with a matchday still to come.
fn still_playing(league: &mut League) {
    league.fixtures = vec![Fixture {
        id: format!("{}-unplayed", league.id),
        status: domain::league::FixtureStatus::Scheduled,
        competition: domain::league::FixtureCompetition::League,
        ..Default::default()
    }];
}

/// A tier that is still playing has no final table, and used to be dropped
/// from the chain rather than stopping it — which left the divisions on
/// either side of it adjacent. A third division's champion went straight
/// into the first, and the club it displaced fell two divisions at once.
#[test]
fn a_mid_season_tier_breaks_the_chain_rather_than_bridging_it() {
    let d1 = division(
        "d1",
        0,
        "XX",
        &[("a1", 40), ("a2", 30), ("a3", 20), ("a4", 10)],
    );
    let mut d2 = division(
        "d2",
        1,
        "XX",
        &[("b1", 40), ("b2", 30), ("b3", 20), ("b4", 10)],
    );
    still_playing(&mut d2);
    let d3 = division(
        "d3",
        2,
        "XX",
        &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
    );
    let (before1, before3) = (d1.participant_ids.clone(), d3.participant_ids.clone());

    let mut competitions = vec![d1, d2, d3];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    assert_eq!(
        by_id("d1").participant_ids,
        before1,
        "d1 must not reach past d2"
    );
    assert_eq!(
        by_id("d3").participant_ids,
        before3,
        "d3 must not reach past d2"
    );
}

/// `priority` is what ranks one division above another, so two leagues
/// sharing one are peers and the order between them is arbitrary. This is
/// also the case a berth pointing at a target that does not exist falls
/// into: its feeders stay on the ladder, and staying on it must not mean
/// trading clubs with each other.
#[test]
fn leagues_at_the_same_priority_are_peers_not_tiers() {
    let north = division("north", 1, "XX", &[("n1", 40), ("n2", 30), ("n3", 20)]);
    let south = division("south", 1, "XX", &[("s1", 40), ("s2", 30), ("s3", 20)]);
    let (bn, bs) = (north.participant_ids.clone(), south.participant_ids.clone());

    let mut competitions = vec![north, south];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    assert_eq!(by_id("north").participant_ids, bn);
    assert_eq!(by_id("south").participant_ids, bs);
}

/// The overlap guard only ever saw the tiers that had finished, so a
/// rollover landing between the Apertura's last matchday and the Clausura's
/// slipped past it: the two Aperturas swapped, and the promoted club ended
/// up registered with the first division's Apertura and the second
/// division's Clausura at the same time.
#[test]
fn split_season_halves_stay_aligned_when_only_one_half_has_ended() {
    let d1a = division(
        "d1-ap",
        0,
        "XX",
        &[("a1", 40), ("a2", 30), ("a3", 20), ("a4", 10)],
    );
    let d2a = division(
        "d2-ap",
        2,
        "XX",
        &[("b1", 40), ("b2", 30), ("b3", 20), ("b4", 10)],
    );
    let mut d1c = division(
        "d1-cl",
        1,
        "XX",
        &[("a1", 40), ("a2", 30), ("a3", 20), ("a4", 10)],
    );
    let mut d2c = division(
        "d2-cl",
        3,
        "XX",
        &[("b1", 40), ("b2", 30), ("b3", 20), ("b4", 10)],
    );
    still_playing(&mut d1c);
    still_playing(&mut d2c);
    let before = d1a.participant_ids.clone();

    let mut competitions = vec![d1a, d2a, d1c, d2c];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    assert_eq!(
        by_id("d1-ap").participant_ids,
        before,
        "the two halves must stay aligned"
    );
}

/// Repeated phases of one pyramid, separated by an excluded rung. The
/// overlap check runs per chained group, so splitting at the excluded rung
/// put the two phases in different groups and neither ever saw the other:
/// both chained, and the phases stopped describing the same division.
#[test]
fn repeated_phases_either_side_of_an_excluded_rung_stay_aligned() {
    let d1_open = division("d1-open", 0, "XX", &[("a1", 40), ("a2", 10)]);
    let d2_open = division("d2-open", 1, "XX", &[("b1", 40), ("b2", 10)]);
    let central = division("central", 2, "XX", &[("c1", 40), ("c2", 10)]);
    // Reversed finishing order in the closing phase, so a swap is visible.
    let d1_close = division("d1-close", 3, "XX", &[("a2", 40), ("a1", 10)]);
    let d2_close = division("d2-close", 4, "XX", &[("b2", 40), ("b1", 10)]);
    let mut feeder = division("feeder", 0, "YY", &[("f1", 40), ("f2", 10)]);
    feeder.berths = vec![position_berth("central", 1, 1)];

    let before: Vec<(String, Vec<String>)> = [&d1_open, &d2_open, &d1_close, &d2_close]
        .iter()
        .map(|c| (c.id.clone(), c.participant_ids.clone()))
        .collect();

    let mut competitions = vec![d1_open, d2_open, central, d1_close, d2_close, feeder];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    for (id, expected) in &before {
        assert_eq!(
            &by_id(id).participant_ids,
            expected,
            "{id} moved clubs although the country's phases overlap"
        );
    }
}

#[test]
fn apertura_and_clausura_are_not_adjacent_tiers() {
    // A split-season country runs both halves over the *same* clubs, as two
    // competitions at consecutive priorities (`create_game`). They are one
    // division played twice, not a two-tier pyramid, so the ladder must
    // leave them alone — swapping between them would put a club on both
    // tables and duplicate it in the regenerated schedule.
    let apertura = division(
        "ar-d1-apertura",
        0,
        "AR",
        &[("a1", 40), ("a2", 30), ("a3", 20), ("a4", 10)],
    );
    let clausura = division(
        "ar-d1-clausura",
        1,
        "AR",
        // A different winner from the Apertura's bottom club, so a swap
        // between the two halves lands a club on a table it is already on.
        &[("a2", 40), ("a1", 30), ("a3", 20), ("a4", 10)],
    );
    let before = apertura.participant_ids.clone();

    let mut competitions = vec![apertura, clausura];
    apply_pyramid_promotion_relegation(&mut competitions);

    for competition in &competitions {
        let unique: HashSet<&String> = competition.participant_ids.iter().collect();
        assert_eq!(
            unique.len(),
            competition.participant_ids.len(),
            "{} must not list a club twice: {:?}",
            competition.id,
            competition.participant_ids
        );
        let clubs: HashSet<&String> = before.iter().collect();
        assert_eq!(
            unique, clubs,
            "{} runs over the same clubs both halves: {:?}",
            competition.id, competition.participant_ids
        );
    }
}

#[test]
fn apply_pyramid_takes_every_sibling_feeder_off_the_ladder() {
    // Three groups feed one Central League. Splitting the chain pair by
    // pair leaves the last group attached to the tier below it, so which
    // group swaps with `lower` would depend on competition order alone.
    let mut competitions = vec![
        division("lower", 4, "BR", &[("d1", 20), ("d2", 10)]),
        division(
            "central",
            0,
            "BR",
            &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
        ),
    ];
    // Ranked 1, 2, 3 rather than all at 1, and `lower` below them at 4.
    // Equal ranks would be refused by the distinct-rank guard on their own,
    // masking whether the sibling rule does anything; ranked apart, the
    // four of them would chain happily if it were removed.
    for (priority, (id, first, second)) in [
        ("north", "n1", "n2"),
        ("south", "s1", "s2"),
        ("east", "e1", "e2"),
    ]
    .into_iter()
    .enumerate()
    .map(|(offset, group)| (offset as u32 + 1, group))
    {
        let mut group = division(id, priority, "BR", &[(first, 20), (second, 10)]);
        group.berths = vec![position_berth("central", 1, 1)];
        competitions.push(group);
    }
    let before: Vec<(String, Vec<String>)> = competitions
        .iter()
        .map(|c| (c.id.clone(), c.participant_ids.clone()))
        .collect();

    apply_pyramid_promotion_relegation(&mut competitions);

    for (competition, (id, expected)) in competitions.iter().zip(&before) {
        assert_eq!(
            &competition.participant_ids, expected,
            "sibling feeders and their target stay off the linear ladder, whatever \
                 order they are declared in: {id} became {:?}",
            competition.participant_ids
        );
    }
}

#[test]
fn apply_pyramid_still_swaps_when_top_flight_is_only_a_non_position_berth_target() {
    // CupWinner, PlayoffWinner, and fallback_to must not pull a league out
    // of the linear pyramid — only a primary PositionRange target does.
    let eng_top = division(
        "eng-1",
        0,
        "ENG",
        &[
            ("t1", 60),
            ("t2", 50),
            ("t3", 40),
            ("t4", 30),
            ("t5", 20),
            ("t6", 10),
        ],
    );
    let mut eng_second = division(
        "eng-2",
        1,
        "ENG",
        &[
            ("s1", 60),
            ("s2", 50),
            ("s3", 40),
            ("s4", 30),
            ("s5", 20),
            ("s6", 10),
        ],
    );
    eng_second.berths = vec![Berth {
        target: "eng-1".to_string(),
        rule: BerthRule::PlayoffWinner { from: 3, to: 6 },
        fallback_to: None,
    }];
    let mut cup = League::new(
        "eng-cup".to_string(),
        "Cup".to_string(),
        2026,
        &["t1".to_string(), "s1".to_string()],
    );
    cup.kind = CompetitionType::Cup;
    cup.country_id = Some("ENG".to_string());
    cup.rules.format = CompetitionFormat::Knockout;
    cup.berths = vec![Berth {
        target: "eng-1".to_string(),
        rule: BerthRule::CupWinner,
        fallback_to: None,
    }];
    let mut other = division("other", 0, "WAL", &[("w1", 20), ("w2", 10)]);
    other.berths = vec![Berth {
        target: "missing-continental".to_string(),
        rule: BerthRule::PositionRange { from: 1, to: 1 },
        fallback_to: Some("eng-1".to_string()),
    }];

    let mut competitions = vec![eng_top, eng_second, cup, other];
    apply_pyramid_promotion_relegation(&mut competitions);

    let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
    let eng1: HashSet<&String> = by_id("eng-1").participant_ids.iter().collect();
    let eng2: HashSet<&String> = by_id("eng-2").participant_ids.iter().collect();
    assert!(
        eng1.contains(&"s1".to_string()),
        "CupWinner / PlayoffWinner / fallback_to must not block linear promotion: {:?}",
        by_id("eng-1").participant_ids
    );
    assert!(!eng1.contains(&"t6".to_string()));
    assert!(eng2.contains(&"t6".to_string()));
    assert!(!eng2.contains(&"s1".to_string()));
}

#[test]
fn cup_winner_into_top_flight_does_not_block_linear_pr_or_mutate_the_cup() {
    // CupWinner → top flight must not become a domestic entrant or turn
    // the cup into a feeder. Linear P/R with the adjacent tier still runs.
    let eng_top = division(
        "eng-1",
        0,
        "ENG",
        &[
            ("t1", 60),
            ("t2", 50),
            ("t3", 40),
            ("t4", 30),
            ("t5", 20),
            ("t6", 10),
        ],
    );
    let eng_second = division(
        "eng-2",
        1,
        "ENG",
        &[
            ("s1", 60),
            ("s2", 50),
            ("s3", 40),
            ("s4", 30),
            ("s5", 20),
            ("s6", 10),
        ],
    );
    let final_id = "eng-cup-final";
    let mut cup = League::new(
        "eng-cup".to_string(),
        "Cup".to_string(),
        2026,
        &["t1".to_string(), "s1".to_string()],
    );
    cup.kind = CompetitionType::Cup;
    cup.country_id = Some("ENG".to_string());
    cup.rules.format = CompetitionFormat::Knockout;
    cup.berths = vec![Berth {
        target: "eng-1".to_string(),
        rule: BerthRule::CupWinner,
        fallback_to: None,
    }];
    cup.fixtures = vec![Fixture {
        id: final_id.to_string(),
        matchday: 1,
        date: "2026-05-01".to_string(),
        home_team_id: "t1".to_string(),
        away_team_id: "s1".to_string(),
        competition: FixtureCompetition::Cup,
        status: FixtureStatus::Completed,
        result: Some(MatchResult {
            home_goals: 2,
            away_goals: 1,
            ..Default::default()
        }),
        ..Default::default()
    }];
    cup.knockout_rounds = vec![KnockoutRoundState {
        id: "eng-cup-final-round".to_string(),
        name: "Final".to_string(),
        fixture_ids: vec![final_id.to_string()],
        bye_team_ids: vec![],
        completed: true,
    }];
    let cup_before = cup.participant_ids.clone();

    let mut game = empty_game();
    game.competitions = vec![eng_top, eng_second, cup];

    let fields = resolve_domestic_berth_fields(&game);
    apply_pyramid_promotion_relegation(&mut game.competitions);
    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    assert_eq!(
        by_id("eng-cup").participant_ids,
        cup_before,
        "CupWinner must not turn the cup into a feeder: {:?}",
        by_id("eng-cup").participant_ids
    );
    let eng1_ids: Vec<&str> = by_id("eng-1")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        eng1_ids.iter().filter(|id| **id == "t1").count(),
        1,
        "cup winner already in top flight must not be duplicated: {eng1_ids:?}"
    );
    let eng1: HashSet<&str> = eng1_ids.iter().copied().collect();
    let eng2: HashSet<&str> = by_id("eng-2")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert!(
        eng1.contains("s1"),
        "linear P/R must still promote the feeder champion: {:?}",
        by_id("eng-1").participant_ids
    );
    assert!(!eng1.contains("t6"));
    assert!(eng2.contains("t6"));
    assert!(!eng2.contains("s1"));
    assert_eq!(
        by_id("eng-1").participant_ids.len(),
        6,
        "top flight keeps its authored size: {:?}",
        by_id("eng-1").participant_ids
    );
}
