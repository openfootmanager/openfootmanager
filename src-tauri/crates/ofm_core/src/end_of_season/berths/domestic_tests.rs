use super::super::continental::resolve_continental_fields;
use super::super::pyramid::apply_pyramid_promotion_relegation;
use super::super::test_support::*;
use super::*;
use domain::league::{
    Berth, BerthRule, CompetitionScope, CompetitionType, Fixture, FixtureCompetition,
    FixtureStatus, League,
};
use std::collections::HashSet;

#[test]
fn resolve_domestic_berth_fields_collects_regional_place_getters() {
    let mut game = two_region_central_pyramid();
    for competition in &mut game.competitions {
        competition.region_id = Some("sa".to_string());
    }
    // High-reputation outsiders must not pad a domestic table: fill is a
    // continental-bracket concern only.
    game.teams.push(filler_club("rep-star", "BR", 99));
    // A continental target with incoming berths must not leak into the
    // domestic map — scope filtering is the whole point of the split.
    let mut continental = League::new(
        "ucl".to_string(),
        "UCL".to_string(),
        2026,
        &["seed-0".to_string(), "seed-1".to_string()],
    );
    continental.scope = CompetitionScope::Continental;
    continental.kind = CompetitionType::ContinentalClub;
    game.competitions[1]
        .berths
        .push(position_berth("ucl", 1, 1));
    game.competitions.push(continental);

    let domestic = resolve_domestic_berth_fields(&game);
    let continental_fields = resolve_continental_fields(&game);

    let central = domestic
        .get("central")
        .expect("Central League is berth-fed");
    let winners: HashSet<&str> = central.iter().map(String::as_str).collect();
    assert_eq!(winners, HashSet::from(["n1", "n2", "s1", "s2"]));
    assert!(
        !domestic.contains_key("ucl"),
        "continental targets stay out of the domestic map: {domestic:?}"
    );
    assert!(
        !domestic.contains_key("north") && !domestic.contains_key("south"),
        "feeders without incoming berths are absent: {domestic:?}"
    );
    assert!(
        continental_fields.contains_key("ucl"),
        "continental resolution is unchanged for berth-fed cups"
    );
    assert!(
        !continental_fields.contains_key("central"),
        "a domestic league must not appear in the continental map"
    );
}

#[test]
fn resolve_domestic_berth_fields_preserves_evaluation_order() {
    let game = four_region_oversubscribed_pyramid();
    let field = resolve_domestic_berth_fields(&game)
        .remove("central")
        .expect("berth-fed");
    assert_eq!(
        field,
        vec![
            "n1".to_string(),
            "n2".to_string(),
            "s1".to_string(),
            "s2".to_string(),
            "e1".to_string(),
            "e2".to_string(),
            "w1".to_string(),
            "w2".to_string(),
        ],
        "field order is competition then berth then standings, not HashMap iteration"
    );
}

#[test]
fn apply_domestic_berth_merges_two_regional_groups_into_central() {
    let mut game = two_region_central_pyramid();
    let fields = resolve_domestic_berth_fields(&game);

    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);

    let central = &by_id("central").participant_ids;
    assert_eq!(
        central.len(),
        8,
        "Central League keeps its authored size: {central:?}"
    );
    // Survivors retained in place; the four regional winners appended.
    assert_eq!(
        &central[..4],
        &[
            "c1".to_string(),
            "c2".to_string(),
            "c3".to_string(),
            "c4".to_string()
        ],
        "top-four finishers survive: {central:?}"
    );
    let promoted: HashSet<&str> = central[4..].iter().map(String::as_str).collect();
    assert_eq!(promoted, HashSet::from(["n1", "n2", "s1", "s2"]));

    let north: HashSet<&str> = by_id("north")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    let south: HashSet<&str> = by_id("south")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert!(
        !north.contains("n1") && !north.contains("n2"),
        "north sent its place-getters up: {:?}",
        by_id("north").participant_ids
    );
    assert!(
        !south.contains("s1") && !south.contains("s2"),
        "south sent its place-getters up: {:?}",
        by_id("south").participant_ids
    );
    // Four dropouts (c5–c8) split round-robin across the two feeders.
    let dropouts: HashSet<&str> = north
        .union(&south)
        .copied()
        .filter(|id| id.starts_with('c'))
        .collect();
    assert_eq!(dropouts, HashSet::from(["c5", "c6", "c7", "c8"]));
    assert_eq!(
        by_id("north").participant_ids.len(),
        4,
        "north keeps its authored size"
    );
    assert_eq!(
        by_id("south").participant_ids.len(),
        4,
        "south keeps its authored size"
    );
    assert_eq!(
        north.intersection(&south).count(),
        0,
        "each dropout lands in exactly one feeder"
    );
}

#[test]
fn domestic_promotion_ignores_a_fallback_target() {
    // `fallbackTo` is a continental cascade. A club that misses its
    // primary target has nowhere to drop to domestically — it stays put.
    // Following the fallback here would add it to the fallback league
    // while the feeder-removal path, which only matches a direct target,
    // left it where it was.
    let eng = division(
        "eng-1",
        0,
        "ENG",
        &[("t1", 40), ("t2", 30), ("t3", 20), ("t4", 10)],
    );
    let mut wales = division("wal-1", 0, "WAL", &[("w1", 20), ("w2", 10)]);
    wales.berths = vec![Berth {
        target: "missing-continental".to_string(),
        rule: BerthRule::PositionRange { from: 1, to: 1 },
        fallback_to: Some("eng-1".to_string()),
    }];
    let mut game = empty_game();
    game.competitions = vec![eng, wales];

    let fields = resolve_domestic_berth_fields(&game);
    assert!(
        !fields
            .get("eng-1")
            .is_some_and(|field| field.contains(&"w1".to_string())),
        "a domestic fallback must not award a place: {fields:?}"
    );

    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    assert_eq!(
        by_id("wal-1").participant_ids,
        vec!["w1".to_string(), "w2".to_string()],
        "the club stays in its own league"
    );
    assert_eq!(
        by_id("eng-1").participant_ids,
        vec![
            "t1".to_string(),
            "t2".to_string(),
            "t3".to_string(),
            "t4".to_string()
        ],
        "the fallback league neither gains a club nor releases a place"
    );
}

#[test]
fn apply_domestic_berth_is_noop_without_incoming_berths() {
    let mut game = empty_game();
    let plain = division("eng-1", 0, "ENG", &[("t1", 30), ("t2", 20), ("t3", 10)]);
    game.competitions = vec![plain];
    let before = game.competitions[0].clone();

    let fields = resolve_domestic_berth_fields(&game);
    assert!(
        fields.is_empty(),
        "a league with no incoming berths is absent from the field map"
    );
    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    assert_eq!(
        serde_json::to_vec(&game.competitions[0]).expect("serialize after"),
        serde_json::to_vec(&before).expect("serialize before"),
        "plain leagues must be byte-for-byte unchanged"
    );
}

#[test]
fn apply_domestic_berth_merges_four_regional_champions() {
    let central = division(
        "central",
        0,
        "BR",
        &[
            ("c1", 80),
            ("c2", 70),
            ("c3", 60),
            ("c4", 50),
            ("c5", 40),
            ("c6", 30),
            ("c7", 20),
            ("c8", 10),
        ],
    );
    let regions = [
        ("north", "n1", "n2"),
        ("south", "s1", "s2"),
        ("east", "e1", "e2"),
        ("west", "w1", "w2"),
    ];
    let mut game = empty_game();
    game.competitions = std::iter::once(central)
        .chain(regions.iter().map(|(id, champ, runner)| {
            let mut league = division(id, 1, "BR", &[(champ, 20), (runner, 10)]);
            league.berths = vec![position_berth("central", 1, 1)];
            league
        }))
        .collect();

    let fields = resolve_domestic_berth_fields(&game);
    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);

    let central = &by_id("central").participant_ids;
    assert_eq!(central.len(), 8);
    let promoted: HashSet<&str> = central[4..].iter().map(String::as_str).collect();
    assert_eq!(promoted, HashSet::from(["n1", "s1", "e1", "w1"]));
    assert_eq!(
        &central[..4],
        &[
            "c1".to_string(),
            "c2".to_string(),
            "c3".to_string(),
            "c4".to_string()
        ]
    );

    let mut received_dropouts = HashSet::new();
    for (id, champ, _runner) in regions {
        let ids: HashSet<&str> = by_id(id)
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert!(
            !ids.contains(champ),
            "{id} must lose its champion {champ}: {:?}",
            by_id(id).participant_ids
        );
        assert_eq!(by_id(id).participant_ids.len(), 2);
        let dropout = by_id(id)
            .participant_ids
            .iter()
            .find(|club| club.starts_with('c'))
            .expect("each feeder receives one Central dropout");
        received_dropouts.insert(dropout.as_str());
    }
    assert_eq!(received_dropouts, HashSet::from(["c5", "c6", "c7", "c8"]));
}

#[test]
fn apply_domestic_berth_preserves_feeder_size_when_quotas_differ() {
    // North sends three, South sends one. A naive 2+2 split of the four
    // Central dropouts would shrink North and grow South.
    let central = division(
        "central",
        0,
        "BR",
        &[
            ("c1", 80),
            ("c2", 70),
            ("c3", 60),
            ("c4", 50),
            ("c5", 40),
            ("c6", 30),
            ("c7", 20),
            ("c8", 10),
        ],
    );
    let mut north = division(
        "north",
        1,
        "BR",
        &[("n1", 40), ("n2", 30), ("n3", 20), ("n4", 10)],
    );
    north.berths = vec![position_berth("central", 1, 3)];
    let mut south = division(
        "south",
        1,
        "BR",
        &[("s1", 40), ("s2", 30), ("s3", 20), ("s4", 10)],
    );
    south.berths = vec![position_berth("central", 1, 1)];
    let mut game = empty_game();
    game.competitions = vec![central, north, south];

    let fields = resolve_domestic_berth_fields(&game);
    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);

    let central = &by_id("central").participant_ids;
    assert_eq!(
        central.len(),
        8,
        "Central League keeps its authored size: {central:?}"
    );
    let central_set: HashSet<&str> = central.iter().map(String::as_str).collect();
    assert!(
        ["n1", "n2", "n3", "s1"]
            .iter()
            .all(|club| central_set.contains(club)),
        "unequal place-getters must all promote: {central:?}"
    );
    assert!(
        ["c5", "c6", "c7", "c8"]
            .iter()
            .all(|club| !central_set.contains(club)),
        "Central dropouts must leave: {central:?}"
    );

    let north: HashSet<&str> = by_id("north")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    let south: HashSet<&str> = by_id("south")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert!(
        !north.contains("n1") && !north.contains("n2") && !north.contains("n3"),
        "north sent its three place-getters up: {:?}",
        by_id("north").participant_ids
    );
    assert!(
        !south.contains("s1"),
        "south sent its champion up: {:?}",
        by_id("south").participant_ids
    );
    assert_eq!(
        by_id("north").participant_ids.len(),
        4,
        "north keeps its authored size after receiving three dropouts: {:?}",
        by_id("north").participant_ids
    );
    assert_eq!(
        by_id("south").participant_ids.len(),
        4,
        "south keeps its authored size after receiving one dropout: {:?}",
        by_id("south").participant_ids
    );
    let north_dropouts: HashSet<&str> = north
        .iter()
        .copied()
        .filter(|id| id.starts_with('c'))
        .collect();
    let south_dropouts: HashSet<&str> = south
        .iter()
        .copied()
        .filter(|id| id.starts_with('c'))
        .collect();
    assert_eq!(
        north_dropouts.len(),
        3,
        "north receives exactly as many dropouts as it promoted: {:?}",
        by_id("north").participant_ids
    );
    assert_eq!(
        south_dropouts.len(),
        1,
        "south receives exactly as many dropouts as it promoted: {:?}",
        by_id("south").participant_ids
    );
    assert_eq!(
        north_dropouts
            .union(&south_dropouts)
            .copied()
            .collect::<HashSet<_>>(),
        HashSet::from(["c5", "c6", "c7", "c8"])
    );
    assert_eq!(
        north.intersection(&south).count(),
        0,
        "each dropout lands in exactly one feeder"
    );
}

#[test]
fn apply_domestic_berth_keeps_surplus_place_getters_in_their_feeder() {
    // 4-club target, four feeders each sending two → 8 winners, 4 places.
    // The feeders rank equally, so they are ordered by id — east then
    // north — and those two take the released places while the rest stay
    // in their feeder. It used to be whichever pair the competition vector
    // happened to list first, which a save and reload does not preserve.
    let mut game = four_region_oversubscribed_pyramid();
    let fields = resolve_domestic_berth_fields(&game);
    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    let central: HashSet<&str> = by_id("central")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        central,
        HashSet::from(["e1", "e2", "n1", "n2"]),
        "the two best-ranked feeders occupy the released places: {:?}",
        by_id("central").participant_ids
    );

    let feeder = |id: &str| -> HashSet<&str> {
        by_id(id)
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect()
    };
    for id in ["north", "south", "east", "west"] {
        assert_eq!(
            by_id(id).participant_ids.len(),
            2,
            "{id} keeps authored size: {:?}",
            by_id(id).participant_ids
        );
    }
    // east and north go up, so their place-getters leave.
    assert!(!feeder("east").contains("e1") && !feeder("east").contains("e2"));
    assert!(!feeder("north").contains("n1") && !feeder("north").contains("n2"));
    assert!(
        feeder("south").contains("s1") && feeder("south").contains("s2"),
        "south surplus stay put: {:?}",
        by_id("south").participant_ids
    );
    assert!(
        feeder("west").contains("w1") && feeder("west").contains("w2"),
        "west surplus stay put: {:?}",
        by_id("west").participant_ids
    );
}

/// The same pyramid, with the competitions listed in a different order.
/// Persistence reloads them sorted by priority, season and name, so an
/// allocation that depended on the vector order promoted different clubs
/// after a save and reload than it did before one.
#[test]
fn oversubscribed_berths_ignore_the_order_competitions_are_listed_in() {
    let mut authored = four_region_oversubscribed_pyramid();
    let fields = resolve_domestic_berth_fields(&authored);
    apply_domestic_berth_promotion_relegation(&mut authored, &fields);
    let promoted: HashSet<String> = authored
        .competitions
        .iter()
        .find(|competition| competition.id == "central")
        .expect("central")
        .participant_ids
        .iter()
        .cloned()
        .collect();

    let mut reloaded = four_region_oversubscribed_pyramid();
    reloaded.competitions.reverse();
    let fields = resolve_domestic_berth_fields(&reloaded);
    apply_domestic_berth_promotion_relegation(&mut reloaded, &fields);
    let promoted_after: HashSet<String> = reloaded
        .competitions
        .iter()
        .find(|competition| competition.id == "central")
        .expect("central")
        .participant_ids
        .iter()
        .cloned()
        .collect();

    assert_eq!(
        promoted, promoted_after,
        "the same season must promote the same clubs whichever order the \
             competitions happen to be listed in"
    );
}

#[test]
fn apply_domestic_berth_does_not_promote_a_club_the_ladder_already_relegated() {
    // A feeder that both sends clubs up and swaps with the tier below is
    // handled by both passes in one rollover. The linear pass relegates
    // `m2` into `low`; the berth merge must not then promote it into
    // `central` off the frozen table, or `m2` plays in two leagues.
    let central = division(
        "central",
        0,
        "BR",
        &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
    );
    let mut mid = division("mid", 1, "BR", &[("m1", 20), ("m2", 10)]);
    mid.berths = vec![position_berth("central", 1, 2)];
    let low = division("low", 2, "BR", &[("l1", 20), ("l2", 10)]);

    let mut game = empty_game();
    game.competitions = vec![central, mid, low];

    let fields = resolve_domestic_berth_fields(&game);
    apply_pyramid_promotion_relegation(&mut game.competitions);
    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    let roster = |id: &str| -> HashSet<&str> {
        by_id(id)
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect()
    };

    assert_eq!(
        by_id("central").participant_ids.len(),
        4,
        "central keeps its authored size: {:?}",
        by_id("central").participant_ids
    );
    assert_eq!(
        by_id("mid").participant_ids.len(),
        2,
        "mid keeps its authored size: {:?}",
        by_id("mid").participant_ids
    );
    assert_eq!(
        by_id("low").participant_ids.len(),
        2,
        "low keeps its authored size: {:?}",
        by_id("low").participant_ids
    );

    let (central_ids, mid_ids, low_ids) = (roster("central"), roster("mid"), roster("low"));
    assert!(
        central_ids.is_disjoint(&mid_ids)
            && central_ids.is_disjoint(&low_ids)
            && mid_ids.is_disjoint(&low_ids),
        "no club may sit on two tables: central {:?} mid {:?} low {:?}",
        by_id("central").participant_ids,
        by_id("mid").participant_ids,
        by_id("low").participant_ids
    );
    assert!(
        low_ids.contains("m2"),
        "the ladder's relegation stands: {:?}",
        by_id("low").participant_ids
    );
    assert!(
        central_ids.contains("m1"),
        "the place-getter still in its feeder is promoted: {:?}",
        by_id("central").participant_ids
    );
}

#[test]
fn apply_domestic_berth_skips_when_target_or_feeder_is_unfinished() {
    // Linear P/R already leaves mid-season tables alone. The berth merge
    // must do the same for the whole target — skipping only an unfinished
    // feeder while still taking its clubs would duplicate them.
    let scheduled = Fixture {
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        ..Default::default()
    };
    for unfinished_id in ["central", "north"] {
        let mut game = two_region_central_pyramid();
        game.competitions
            .iter_mut()
            .find(|competition| competition.id == unfinished_id)
            .expect(unfinished_id)
            .fixtures
            .push(scheduled.clone());
        let before: Vec<Vec<String>> = game
            .competitions
            .iter()
            .map(|competition| competition.participant_ids.clone())
            .collect();

        let fields = resolve_domestic_berth_fields(&game);
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        for (competition, expected) in game.competitions.iter().zip(&before) {
            assert_eq!(
                &competition.participant_ids, expected,
                "unfinished {unfinished_id} must leave {} unchanged: {:?}",
                competition.id, competition.participant_ids
            );
        }
    }
}

#[test]
fn apply_domestic_berth_ignores_unfinished_non_position_range_source() {
    // A mid-season PlayoffWinner table aimed at Central must not block
    // finished PositionRange feeders — resolve cannot take those clubs.
    let scheduled = Fixture {
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        ..Default::default()
    };
    let mut game = two_region_central_pyramid();
    let mut playoff = division("playoff", 2, "BR", &[("p1", 20), ("p2", 10)]);
    playoff.berths = vec![Berth {
        target: "central".to_string(),
        rule: BerthRule::PlayoffWinner { from: 1, to: 2 },
        fallback_to: None,
    }];
    playoff.fixtures.push(scheduled);
    game.competitions.push(playoff);

    let fields = resolve_domestic_berth_fields(&game);
    apply_domestic_berth_promotion_relegation(&mut game, &fields);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    let central: HashSet<&str> = by_id("central")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert!(
        ["n1", "n2", "s1", "s2"]
            .iter()
            .all(|club| central.contains(club)),
        "finished feeders still promote: {:?}",
        by_id("central").participant_ids
    );
    assert!(
        ["c5", "c6", "c7", "c8"]
            .iter()
            .all(|club| !central.contains(club)),
        "Central dropouts must leave: {:?}",
        by_id("central").participant_ids
    );
    assert_eq!(
        by_id("playoff").participant_ids,
        vec!["p1".to_string(), "p2".to_string()],
        "PlayoffWinner source is not a feeder"
    );
}
