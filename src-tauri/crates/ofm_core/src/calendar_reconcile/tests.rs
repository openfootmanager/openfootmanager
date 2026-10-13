use super::*;
use crate::calendar_booking::too_close;
use domain::league::{CompetitionFormat, FixtureCompetition as Kind, KnockoutRoundState};

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn march(d: u32) -> NaiveDate {
    day(2033, 3, d)
}

fn text(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn fx(competition: &str, id: &str, date: NaiveDate, home: &str, away: &str, kind: Kind) -> Fixture {
    Fixture {
        id: id.into(),
        competition_id: competition.into(),
        matchday: 1,
        date: text(date),
        home_team_id: home.into(),
        away_team_id: away.into(),
        competition: kind,
        ..Default::default()
    }
}

fn competition(id: &str, priority: u32, fixtures: Vec<Fixture>) -> League {
    let mut league = League::new(id.into(), id.into(), 2033, &[]);
    league.priority = priority;
    league.fixtures = fixtures;
    league
}

/// A knockout cup whose rounds are given as fixture lists; round `i` is complete when `completed[i]`.
fn knockout(id: &str, priority: u32, rounds: Vec<Vec<Fixture>>, completed: &[bool]) -> League {
    let mut cup = competition(id, priority, rounds.iter().flatten().cloned().collect());
    cup.rules.format = CompetitionFormat::Knockout;
    cup.rules.knockout_round_gap_days = 3;
    cup.rules.knockout_matches_per_day = 4;
    cup.knockout_rounds = rounds
        .iter()
        .enumerate()
        .map(|(index, fixtures)| KnockoutRoundState {
            id: format!("{id}-round-{index}"),
            fixture_ids: fixtures.iter().map(|f| f.id.clone()).collect(),
            completed: completed.get(index).copied().unwrap_or(false),
            ..Default::default()
        })
        .collect();
    cup.participant_ids = rounds[0]
        .iter()
        .flat_map(|f| [f.home_team_id.clone(), f.away_team_id.clone()])
        .collect();
    cup
}

fn none() -> BTreeSet<NaiveDate> {
    BTreeSet::new()
}

fn today() -> NaiveDate {
    day(2033, 2, 1)
}

fn run(world: &mut [League]) -> Plan {
    reconcile(world, &none(), today(), true).unwrap()
}

fn date_of(world: &[League], competition: &str, fixture: &str) -> NaiveDate {
    let found = world
        .iter()
        .find(|c| c.id == competition)
        .and_then(|c| c.fixtures.iter().find(|f| f.id == fixture))
        .unwrap_or_else(|| panic!("{competition}/{fixture} is missing"));
    NaiveDate::parse_from_str(&found.date, "%Y-%m-%d").unwrap()
}

/// Every pair of fixtures of one club that sit closer than the rest window.
fn clashes(world: &[League]) -> Vec<(String, String, String)> {
    let mut rows: Vec<(&str, NaiveDate, &str)> = Vec::new();
    for fixture in world.iter().flat_map(|c| &c.fixtures) {
        if fixture.competition == Kind::InternationalNation {
            continue;
        }
        let date = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").unwrap();
        rows.push((&fixture.home_team_id, date, &fixture.id));
        rows.push((&fixture.away_team_id, date, &fixture.id));
    }
    rows.sort();
    let mut found = Vec::new();
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            if a.0 != b.0 {
                break;
            }
            if too_close(a.1, b.1) {
                found.push((a.0.to_string(), a.2.to_string(), b.2.to_string()));
            }
        }
    }
    found
}

/// Given a cup tie and a league match of one club on the same day, when the calendar is
/// reconciled, then the league match moves to the first day three days clear of the cup tie.
#[test]
fn a_league_match_moves_to_the_earliest_day_free_for_both_clubs() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
        ),
        competition(
            "league",
            2,
            vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
        ),
    ];
    let plan = run(&mut world);
    assert_eq!(date_of(&world, "league", "l1"), march(11));
    assert_eq!(date_of(&world, "cup", "c1"), march(8));
    assert_eq!(plan.moves.len(), 1);
    assert_eq!(plan.moves[0].reason, RescheduleReason::CupTie);
}

/// Given a league round where only one match clashes, when reconciled, then only that match moves.
#[test]
fn only_the_clashing_match_moves() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
        ),
        competition(
            "league",
            2,
            vec![
                fx("league", "l1", march(8), "a", "c", Kind::League),
                fx("league", "l2", march(8), "d", "e", Kind::League),
            ],
        ),
    ];
    run(&mut world);
    assert_eq!(date_of(&world, "league", "l2"), march(8));
}

/// Given a cup tie and a league match that clash, when reconciled, then the cup tie never moves for the league.
#[test]
fn a_cup_tie_never_moves_for_a_league_fixture() {
    let mut world = vec![
        competition(
            "league",
            1,
            vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
        ),
        competition(
            "cup",
            2,
            vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
        ),
    ];
    run(&mut world);
    assert_eq!(date_of(&world, "cup", "c1"), march(8));
    assert_ne!(date_of(&world, "league", "l1"), march(8));
}

/// Given a continental tie and a domestic cup tie that clash, when reconciled, then the cup tie yields.
#[test]
fn a_domestic_cup_yields_to_a_continental_competition() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
        ),
        competition(
            "euro",
            9,
            vec![fx("euro", "e1", march(8), "a", "z", Kind::ContinentalClub)],
        ),
    ];
    let plan = run(&mut world);
    assert_eq!(date_of(&world, "euro", "e1"), march(8));
    assert_eq!(date_of(&world, "cup", "c1"), march(11));
    assert_eq!(plan.moves[0].reason, RescheduleReason::ContinentalTie);
}

/// Given two cups colliding on a club, when reconciled, then the cup with the larger priority number yields.
#[test]
fn the_lower_priority_cup_moves_when_two_cups_collide() {
    let mut world = vec![
        competition(
            "a-cup-low",
            5,
            vec![fx("a-cup-low", "x", march(8), "a", "b", Kind::Cup)],
        ),
        competition(
            "z-cup-high",
            1,
            vec![fx("z-cup-high", "y", march(8), "a", "c", Kind::Cup)],
        ),
    ];
    run(&mut world);
    assert_eq!(date_of(&world, "z-cup-high", "y"), march(8));
    assert_eq!(date_of(&world, "a-cup-low", "x"), march(11));
}

/// Given a national league and a state championship colliding, when reconciled, then the larger priority number yields.
#[test]
fn the_state_championship_with_the_larger_priority_yields() {
    let mut world = vec![
        competition(
            "a-state",
            7,
            vec![fx("a-state", "s1", march(8), "a", "b", Kind::League)],
        ),
        competition(
            "z-serie-a",
            1,
            vec![fx("z-serie-a", "n1", march(8), "a", "c", Kind::League)],
        ),
    ];
    run(&mut world);
    assert_eq!(date_of(&world, "z-serie-a", "n1"), march(8));
    assert_eq!(date_of(&world, "a-state", "s1"), march(11));
}

/// Given a friendly clashing with a league match, when reconciled, then the friendly is dropped and the league match stays.
#[test]
fn a_friendly_yields_to_every_competitive_fixture() {
    let mut world = vec![competition(
        "league",
        1,
        vec![
            fx("league", "l1", march(8), "a", "c", Kind::League),
            fx("league", "f1", march(9), "a", "d", Kind::Friendly),
        ],
    )];
    let plan = run(&mut world);
    assert_eq!(date_of(&world, "league", "l1"), march(8));
    assert!(world[0].fixtures.iter().all(|f| f.id != "f1"));
    assert_eq!(
        plan.dropped_friendlies,
        vec![("league".to_string(), "f1".to_string())]
    );
}

/// Given a played fixture and a scheduled one that clash, when reconciled, then the played one never moves.
#[test]
fn a_played_or_live_fixture_never_moves() {
    for status in [FixtureStatus::Completed, FixtureStatus::InProgress] {
        let mut played = fx("league", "l1", march(8), "a", "c", Kind::League);
        played.status = status;
        let mut world = vec![
            competition("league", 2, vec![played]),
            competition(
                "cup",
                1,
                vec![fx("cup", "c1", march(9), "a", "b", Kind::Cup)],
            ),
        ];
        run(&mut world);
        assert_eq!(date_of(&world, "league", "l1"), march(8));
        assert_eq!(date_of(&world, "cup", "c1"), march(11));
    }
}

/// Given a fixture dated today, when reconciled, then it does not move; one dated tomorrow can.
#[test]
fn a_fixture_dated_today_never_moves() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", today(), "a", "b", Kind::Cup)],
        ),
        competition(
            "league",
            2,
            vec![fx("league", "l1", today(), "a", "c", Kind::League)],
        ),
    ];
    run(&mut world);
    assert_eq!(date_of(&world, "league", "l1"), today());
}

/// Given a blackout over a league match, when reconciled, then it moves off it with the window as the reason.
#[test]
fn a_league_match_moves_off_an_international_window() {
    let mut world = vec![competition(
        "league",
        1,
        vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
    )];
    let blackout: BTreeSet<_> = (8..=10).map(march).collect();
    let plan = reconcile(&mut world, &blackout, today(), true).unwrap();
    assert_eq!(date_of(&world, "league", "l1"), march(11));
    assert_eq!(plan.moves[0].reason, RescheduleReason::InternationalWindow);
}

/// Given national-team fixtures that share ids with no club, when reconciled, then they neither move nor block club fixtures.
#[test]
fn national_team_fixtures_are_ignored() {
    let mut world = vec![
        competition(
            "wc",
            9,
            vec![fx(
                "wc",
                "w1",
                march(8),
                "a",
                "z",
                Kind::InternationalNation,
            )],
        ),
        competition(
            "league",
            1,
            vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
        ),
    ];
    let plan = run(&mut world);
    assert!(plan.moves.is_empty());
}

/// Given a postponed league match that lands after the league's last round, when reconciled, then it is allowed to.
#[test]
fn a_postponed_match_may_land_after_the_last_round() {
    let mut world = vec![
        competition(
            "league",
            2,
            vec![
                fx("league", "l1", march(1), "a", "b", Kind::League),
                fx("league", "l2", march(8), "a", "b", Kind::League),
            ],
        ),
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(7), "a", "z", Kind::Cup)],
        ),
    ];
    run(&mut world);
    assert!(date_of(&world, "league", "l2") > march(8));
}

/// Given a final round of three matches with one clash, when reconciled, then that match moves alone.
#[test]
fn a_final_round_match_that_clashes_moves_alone() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(8), "a", "z", Kind::Cup)],
        ),
        competition(
            "league",
            2,
            vec![
                fx("league", "l1", march(8), "a", "b", Kind::League),
                fx("league", "l2", march(8), "c", "d", Kind::League),
                fx("league", "l3", march(8), "e", "f", Kind::League),
            ],
        ),
    ];
    run(&mut world);
    assert_eq!(date_of(&world, "league", "l2"), march(8));
    assert_eq!(date_of(&world, "league", "l3"), march(8));
    assert_ne!(date_of(&world, "league", "l1"), march(8));
}

/// Given a dormant cup draw that clashes with an active league fixture, when reconciled, then the league fixture moves.
#[test]
fn a_dormant_cup_draw_displaces_an_active_league_fixture() {
    let mut world = vec![
        competition(
            "league-active",
            1,
            vec![fx("league-active", "l1", march(8), "a", "b", Kind::League)],
        ),
        competition(
            "cup-dormant",
            0,
            vec![fx("cup-dormant", "c1", march(9), "a", "z", Kind::Cup)],
        ),
    ];
    run(&mut world);
    assert_eq!(date_of(&world, "cup-dormant", "c1"), march(9));
    assert!(!too_close(date_of(&world, "league-active", "l1"), march(9)));
}

/// Given a knockout whose next round is still to be drawn, when a league match must move, then it
/// never lands on the dates that round could claim for the clubs still in it.
#[test]
fn a_shadow_date_is_never_a_displacement_target() {
    // Round 1 (two ties, unplayed) is on Mar 1; the next round is predicted for Mar 4 (gap 3).
    let cup = knockout(
        "cup",
        1,
        vec![vec![
            fx("cup", "c1", march(1), "a", "b", Kind::Cup),
            fx("cup", "c2", march(1), "c", "d", Kind::Cup),
        ]],
        &[],
    );
    let league = competition(
        "league",
        2,
        vec![fx("league", "l1", march(2), "a", "x", Kind::League)],
    );
    let mut world = vec![cup, league];
    run(&mut world);
    // Mar 4 is the first firm-legal day for a, but the next cup round may claim it, so the league
    // match goes three days clear of that shadow.
    assert_eq!(date_of(&world, "league", "l1"), march(7));
}

/// Given a league match that already sits on a day a cup draw could claim but clashes with nothing firm,
/// when reconciled, then it stays: shadows constrain targets only.
#[test]
fn a_shadow_alone_never_moves_a_fixture() {
    let cup = knockout(
        "cup",
        1,
        vec![vec![
            fx("cup", "c1", march(1), "a", "b", Kind::Cup),
            fx("cup", "c2", march(1), "c", "d", Kind::Cup),
        ]],
        &[],
    );
    let league = competition(
        "league",
        2,
        vec![fx("league", "l1", march(5), "a", "x", Kind::League)],
    );
    let mut world = vec![cup, league];
    let plan = run(&mut world);
    assert!(plan.moves.is_empty());
}

/// Given a cup in its semifinals, when a league match of a surviving club and one of an eliminated club
/// both have to move, then only the survivor keeps clear of the dates the final could claim.
#[test]
fn an_eliminated_club_frees_its_shadow_dates() {
    let quarterfinals = vec![
        fx("cup", "q1", day(2033, 2, 20), "a", "b", Kind::Cup),
        fx("cup", "q2", day(2033, 2, 20), "c", "d", Kind::Cup),
        fx("cup", "q3", day(2033, 2, 20), "e", "f", Kind::Cup),
        fx("cup", "q4", day(2033, 2, 20), "g", "h", Kind::Cup),
    ];
    let semifinals = vec![
        fx("cup", "s1", march(1), "a", "c", Kind::Cup),
        fx("cup", "s2", march(1), "e", "g", Kind::Cup),
    ];
    let mut cup = knockout("cup", 1, vec![quarterfinals, semifinals], &[true, false]);
    for fixture in cup.fixtures.iter_mut().filter(|f| f.id.starts_with('q')) {
        fixture.status = FixtureStatus::Completed;
    }
    // The final would be predicted for Mar 4 (gap 3), but a continental tie of the surviving club
    // "a" on Mar 4 is another cup-layer date involving a finalist, so the final is predicted for
    // Mar 7 for a, c, e and g. Both league matches clash on Mar 4 with that tie: the survivor's
    // and the eliminated club's, which is its opponent.
    let euro = competition(
        "euro",
        9,
        vec![fx("euro", "e1", march(4), "b", "a", Kind::ContinentalClub)],
    );
    let league = competition(
        "league",
        2,
        vec![
            fx("league", "survivor", march(4), "a", "x", Kind::League),
            fx("league", "eliminated", march(4), "b", "y", Kind::League),
        ],
    );
    let mut world = vec![cup, euro, league];
    run(&mut world);
    assert_eq!(date_of(&world, "league", "eliminated"), march(7));
    assert_eq!(date_of(&world, "league", "survivor"), march(10));
}

/// Given the same world twice, when reconciled, then the second sweep moves nothing.
#[test]
fn the_sweep_changes_nothing_on_a_clean_calendar() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
        ),
        competition(
            "league",
            2,
            vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
        ),
    ];
    assert!(!run(&mut world).moves.is_empty());
    let settled = serde_json::to_value(&world).unwrap();
    assert!(run(&mut world).moves.is_empty());
    assert_eq!(serde_json::to_value(&world).unwrap(), settled);
    assert!(clashes(&world).is_empty());
}

/// Given the competitions in any order, when reconciled, then the result is identical.
#[test]
fn the_result_does_not_depend_on_input_order() {
    let build = || {
        vec![
            competition(
                "league",
                3,
                vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
            ),
            competition(
                "cup",
                1,
                vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
            ),
            competition(
                "euro",
                0,
                vec![fx("euro", "e1", march(9), "a", "z", Kind::ContinentalClub)],
            ),
        ]
    };
    let mut forward = build();
    let mut reversed = build();
    reversed.reverse();
    run(&mut forward);
    run(&mut reversed);
    reversed.reverse();
    assert_eq!(
        serde_json::to_value(&forward).unwrap(),
        serde_json::to_value(&reversed).unwrap()
    );
}

/// Given moves are applied, then every move goes forward and nothing already settled changes.
#[test]
fn moves_are_forward_only() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
        ),
        competition(
            "league",
            2,
            vec![fx("league", "l1", march(9), "a", "c", Kind::League)],
        ),
    ];
    let plan = run(&mut world);
    assert!(plan.moves.iter().all(|m| m.to > m.from));
}

/// Given a calendar with no legal day inside the search cap, when reconciled, then it fails loudly instead of hanging.
#[test]
fn a_search_past_the_cap_fails_loudly_instead_of_hanging() {
    let mut world = vec![
        competition(
            "cup",
            1,
            vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
        ),
        competition(
            "league",
            2,
            vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
        ),
    ];
    let blackout: BTreeSet<_> = (0..=800)
        .map(|d| march(1) + Days::new(d))
        .filter(|d| *d != march(8))
        .collect();
    let error = reconcile(&mut world, &blackout, today(), true).unwrap_err();
    assert!(matches!(error, ReconcileError::SearchCapExceeded { .. }));
}

/// Given a fixture with an unreadable date, when reconciled, then it fails instead of guessing.
#[test]
fn an_unreadable_date_is_an_error_not_a_guess() {
    let mut bad = fx("league", "l1", march(8), "a", "c", Kind::League);
    bad.date = "soon".into();
    let mut world = vec![competition("league", 1, vec![bad])];
    assert!(matches!(
        reconcile(&mut world, &none(), today(), true),
        Err(ReconcileError::UnreadableDate { .. })
    ));
}

/// Given moves are applied for a published calendar, then the original date and reason are recorded once;
/// generation passes false and records nothing.
#[test]
fn postponements_are_recorded_only_when_asked_and_keep_the_first_date() {
    let build = || {
        vec![
            competition(
                "cup",
                1,
                vec![fx("cup", "c1", march(8), "a", "b", Kind::Cup)],
            ),
            competition(
                "league",
                2,
                vec![fx("league", "l1", march(8), "a", "c", Kind::League)],
            ),
        ]
    };
    let mut quiet = build();
    reconcile(&mut quiet, &none(), today(), false).unwrap();
    assert!(quiet[1].fixtures[0].postponement.is_none());

    let mut published = build();
    reconcile(&mut published, &none(), today(), true).unwrap();
    let first = published[1].fixtures[0].postponement.clone().unwrap();
    assert_eq!(first.original_date, "2033-03-08");
    // A later cup draw moves it again; the first original date is kept.
    published.push(competition(
        "late",
        0,
        vec![fx("late", "x", march(11), "a", "q", Kind::ContinentalClub)],
    ));
    reconcile(&mut published, &none(), today(), true).unwrap();
    assert_eq!(
        published[1].fixtures[0]
            .postponement
            .as_ref()
            .unwrap()
            .original_date,
        "2033-03-08"
    );
    assert!(published[1].fixtures[0].date > text(march(11)));
}

/// Given a feeding round pushed past its successor's date by an international window, when reconciled,
/// then the successor moves to after the last match that feeds it.
#[test]
fn a_seeded_round_is_never_dated_before_its_feeding_round() {
    let round_one = vec![
        fx("cup", "c1", march(9), "a", "b", Kind::Cup),
        fx("cup", "c2", march(9), "c", "d", Kind::Cup),
    ];
    let round_two = vec![fx("cup", "c3", march(10), "e", "f", Kind::Cup)];
    let mut world = vec![knockout("cup", 1, vec![round_one, round_two], &[])];
    let blackout: BTreeSet<_> = (9..=11).map(march).collect();
    reconcile(&mut world, &blackout, today(), true).unwrap();
    assert_eq!(date_of(&world, "cup", "c1"), march(12));
    assert!(date_of(&world, "cup", "c3") > date_of(&world, "cup", "c1"));
}

/// Given round two authored before round one, when reconciled, then round two is moved to after
/// the last match of round one.
#[test]
fn a_successor_authored_before_its_feeders_is_moved_after_them() {
    let round_one = vec![
        fx("cup", "c1", march(12), "a", "b", Kind::Cup),
        fx("cup", "c2", march(12), "c", "d", Kind::Cup),
    ];
    let round_two = vec![fx("cup", "c3", march(10), "e", "f", Kind::Cup)];
    let mut world = vec![knockout("cup", 1, vec![round_one, round_two], &[])];
    let plan = run(&mut world);
    assert_eq!(date_of(&world, "cup", "c1"), march(12));
    assert!(date_of(&world, "cup", "c3") > march(12));
    assert_eq!(plan.moves.len(), 1);
}

/// Given a cup with a round still to be drawn and many unrelated cups playing nearby dates, when a
/// league match of a club in the first cup is displaced, then the unrelated cups do not push the
/// first cup's predicted round away: the match avoids the predicted date.
#[test]
fn unrelated_cups_do_not_hide_a_cups_predicted_round() {
    let aug = |d: u32| day(2033, 8, d);
    let ties = vec![
        fx("cup", "t1", aug(1), "a", "b", Kind::Cup),
        fx("cup", "t2", aug(1), "c", "d", Kind::Cup),
    ];
    let mut world = vec![
        knockout("cup", 1, vec![ties], &[]),
        competition(
            "league",
            9,
            vec![fx("league", "l1", aug(2), "a", "z", Kind::League)],
        ),
    ];
    for index in 0..19u32 {
        let id = format!("other-{index}");
        let fixture = fx(
            &id,
            "o1",
            aug(2 + index),
            &format!("p{index}"),
            &format!("q{index}"),
            Kind::Cup,
        );
        world.push(knockout(&id, 2, vec![vec![fixture]], &[]));
    }
    run(&mut world);
    assert_eq!(date_of(&world, "league", "l1"), aug(7));
}

/// Given a tie of a friendly cup that clashes with a league match, when reconciled, then the tie is
/// moved, not dropped: the next round of its bracket names the winner.
#[test]
fn a_clashing_friendly_cup_tie_moves_instead_of_being_dropped() {
    let tie = fx("fc", "t1", march(10), "a", "b", Kind::FriendlyCup);
    let mut world = vec![
        knockout("fc", 5, vec![vec![tie]], &[]),
        competition(
            "league",
            1,
            vec![fx("league", "l1", march(10), "a", "z", Kind::League)],
        ),
    ];
    let plan = run(&mut world);
    assert!(plan.dropped_friendlies.is_empty());
    assert!(date_of(&world, "fc", "t1") > march(10));
    assert_eq!(date_of(&world, "league", "l1"), march(10));
}

/// Given a friendly outside any knockout bracket that clashes with a league match, when reconciled, then it is still dropped.
#[test]
fn a_clashing_one_off_friendly_is_still_dropped() {
    let mut world = vec![competition(
        "league",
        1,
        vec![
            fx("league", "l1", march(10), "a", "z", Kind::League),
            fx("league", "f1", march(10), "a", "y", Kind::Friendly),
        ],
    )];
    let plan = run(&mut world);
    assert_eq!(plan.dropped_friendlies.len(), 1);
}

/// Given a cup whose round gap is seven days and a feeder moved to a later day, when reconciled,
/// then the successor is dated the gap after the feeder, not one day after.
#[test]
fn a_successor_waits_the_cups_round_gap_after_its_feeder() {
    let round_one = vec![fx("cup", "c1", march(10), "a", "b", Kind::Cup)];
    let round_two = vec![fx("cup", "c2", march(11), "c", "d", Kind::Cup)];
    let mut cup = knockout("cup", 1, vec![round_one, round_two], &[]);
    cup.rules.knockout_round_gap_days = 7;
    let mut world = vec![cup];
    run(&mut world);
    assert_eq!(date_of(&world, "cup", "c1"), march(10));
    assert_eq!(date_of(&world, "cup", "c2"), march(17));
}

/// Given six entrants whose first round has two byes and the next round four clubs, when shadows
/// are booked one match a day, then the final's shadow lands where the four-club round leaves it.
#[test]
fn byes_are_not_halved_with_the_players_when_predicting_rounds() {
    let ties = vec![
        fx("cup", "c1", march(10), "a", "b", Kind::Cup),
        fx("cup", "c2", march(10), "c", "d", Kind::Cup),
    ];
    let mut cup = knockout("cup", 1, vec![ties], &[]);
    cup.rules.knockout_matches_per_day = 1;
    cup.knockout_rounds[0].bye_team_ids = vec!["e".into(), "f".into()];
    let settled: HashMap<(&str, &str), NaiveDate> = cup
        .fixtures
        .iter()
        .map(|f| (("cup", f.id.as_str()), march(10)))
        .collect();
    let mut occupancy = Occupancy::default();

    shadows::book_cup_shadows(
        &mut occupancy,
        &cup,
        &settled,
        &none(),
        &none(),
        Layer::DomesticCup,
    );

    // Round two: four clubs, two ties, one a day: 13 and 14 March. The final follows on 17 March,
    // so its shadow blocks 19 March. Halving the byes too would put it on 16 March.
    assert!(!occupancy.is_free_of_shadows("e", march(19)));
}
