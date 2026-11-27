use super::*;

// ---------------------------------------------------------------------------
// process_end_of_season — messages
// ---------------------------------------------------------------------------

#[test]
fn champion_receives_prize_money_and_ledger_entry() {
    let mut game = make_completed_season_game();
    let initial_finance = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .unwrap()
        .finance;

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert_eq!(team1.finance, initial_finance + 5_000_000);
    assert_eq!(team1.season_income, 5_000_000);
    assert_eq!(team1.financial_ledger.len(), 1);
    assert_eq!(
        team1.financial_ledger[0].kind,
        FinancialTransactionKind::PrizeMoney
    );
    assert_eq!(team1.financial_ledger[0].amount, 5_000_000);
    assert_eq!(
        team1.financial_ledger[0].description,
        "be.msg.seasonPayout.ledgerDescription?season=1&position=1"
    );
}

/// A continental competition still playing its own season keeps its roster.
/// Handing it next season's qualifiers left a table scoring clubs that were no
/// longer in it and fixtures between clubs it no longer listed.
#[test]
fn an_unfinished_continental_table_keeps_its_roster() {
    let mut game = make_completed_season_game();
    for id in ["team3", "team4"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }
    for team in game.teams.iter_mut() {
        team.football_nation = "ENG".to_string();
    }

    let domestic = first_division("eng-d1", "ENG", "europe", &["team1", "team2"]);
    let mut domestic = domestic;
    domestic.fixtures = vec![make_completed_fixture("d1f", "team1", "team2", 2, 0)];

    // A continental competition scored as a table, one match still to play.
    let mut continental = continental_cup("ccc", "europe", 2);
    continental.participant_ids = vec!["team3".to_string(), "team4".to_string()];
    let mut pending = make_completed_fixture("ccc-live", "team3", "team4", 0, 0);
    pending.status = FixtureStatus::Scheduled;
    pending.result = None;
    continental.fixtures = vec![pending];
    continental.standings = vec![make_standing("team3", 1, 0, 0, 2, 0)];

    game.league = Some(domestic.clone());
    game.competitions = vec![domestic, continental];

    process_end_of_season(&mut game);

    let cup = game
        .competitions
        .iter()
        .find(|competition| competition.id == "ccc")
        .expect("ccc");
    // Exactly its own clubs — "still contains them" is not enough, because the
    // qualifier pass appends rather than replaces and the roster simply grew.
    assert_eq!(
        cup.participant_ids,
        vec!["team3".to_string(), "team4".to_string()],
        "a mid-season continental table keeps exactly the clubs its fixtures name"
    );
}

/// A save whose clock and competition seasons disagree must not replay years.
#[test]
fn regeneration_never_rewinds_a_finished_season() {
    let mut game = make_completed_season_game();
    let mut league = first_division("eng-d1", "ENG", "europe", &["team1", "team2"]);
    // Stamped far ahead of the clock, which sits in 2026.
    league.season = 2030;
    league.fixtures = vec![make_completed_fixture("f", "team1", "team2", 2, 0)];
    game.league = Some(league.clone());
    game.competitions = vec![league];

    process_end_of_season(&mut game);

    let rolled = game
        .competitions
        .iter()
        .find(|competition| competition.id == "eng-d1")
        .expect("eng-d1");
    assert!(
        rolled.season > 2030,
        "a competition that finished season 2030 cannot roll back to {}",
        rolled.season
    );
}

/// Two divisions can share a rank — a berth moves clubs into its target, and
/// nothing makes a target outrank its feeder. A champion promoted along a berth
/// was told they had been relegated.
#[test]
fn a_berth_promotion_is_not_reported_as_a_relegation() {
    let mut game = make_completed_season_game();
    game.manager.hire("team3".to_string());
    for id in ["team3", "team4"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }

    let central = League {
        id: "central".to_string(),
        name: "Central League".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![make_completed_fixture("cf", "team1", "team2", 2, 0)],
        standings: vec![
            make_standing("team1", 1, 0, 0, 2, 0),
            make_standing("team2", 0, 0, 1, 0, 2),
        ],
        ..Default::default()
    };
    let mut feeder = League {
        id: "feeder".to_string(),
        name: "Feeder League".to_string(),
        country_id: Some("ENG".to_string()),
        // Deliberately the SAME rank as its target.
        priority: 0,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("ff", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    feeder.berths = vec![Berth {
        target: "central".to_string(),
        rule: BerthRule::PositionRange { from: 1, to: 1 },
        fallback_to: None,
    }];

    game.league = Some(feeder.clone());
    game.competitions = vec![central, feeder];

    process_end_of_season(&mut game);

    let central_now = game
        .competitions
        .iter()
        .find(|competition| competition.id == "central")
        .expect("central");
    assert!(
        central_now.participant_ids.contains(&"team3".to_string()),
        "precondition: the feeder's champion was promoted: {:?}",
        central_now.participant_ids
    );
    assert!(
        !game.messages.iter().any(|message| message
            .subject_key
            .as_deref()
            .is_some_and(|key| key.contains("relegation"))),
        "a promoted champion must not be told they were relegated: {:?}",
        game.messages
            .iter()
            .filter_map(|message| message.subject_key.clone())
            .collect::<Vec<_>>()
    );
}

/// Prize money halves for each tier below the top flight, and the tier is the
/// division's rank in its own pyramid. Ranking only the *finished* divisions
/// made a second division the top flight whenever the first was still playing,
/// and its champion banked a top-flight cheque.
#[test]
fn an_unfinished_upper_tier_does_not_pay_top_flight_prize_money() {
    let mut game = make_completed_season_game();
    game.manager.hire("team3".to_string());
    for id in ["team3", "team4"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }

    // The first division is still playing: one fixture left to go.
    let mut first = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        standings: vec![
            make_standing("team1", 1, 0, 0, 2, 0),
            make_standing("team2", 0, 0, 1, 0, 2),
        ],
        ..Default::default()
    };
    let mut pending = make_completed_fixture("d1-live", "team1", "team2", 0, 0);
    pending.status = FixtureStatus::Scheduled;
    pending.result = None;
    first.fixtures = vec![
        make_completed_fixture("d1-done", "team1", "team2", 2, 0),
        pending,
    ];

    let second = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };

    game.league = Some(second.clone());
    game.competitions = vec![first, second];
    let before = game
        .teams
        .iter()
        .find(|team| team.id == "team3")
        .expect("team3")
        .finance;

    process_end_of_season(&mut game);

    let gained = game
        .teams
        .iter()
        .find(|team| team.id == "team3")
        .expect("team3")
        .finance
        - before;
    assert_eq!(
        gained, 2_500_000,
        "the second division's champion earns half the top flight's 5,000,000, \
         even while the first division is unfinished"
    );
}

/// One division result must feed everything the player is told.
///
/// The payout is taken from the table a split-season country ends its year on,
/// but the summary, the club's record, the manager's trophies and the payout
/// message were all read from whichever table happened to come first. A club
/// could be crowned champion on screen, have second place written into its
/// history, be paid for second, and be told in the same breath that it earned
/// the champion's cheque.
#[test]
fn a_split_season_tells_the_player_one_consistent_story() {
    let mut game = make_completed_season_game();

    // The user wins the opening half and finishes second in the closing one.
    let mut apertura = first_division("ar-d1-apertura", "AR", "america", &["team1", "team2"]);
    apertura.priority = 0;
    apertura.fixtures = vec![make_completed_fixture("ap", "team1", "team2", 2, 0)];
    let mut clausura = first_division("ar-d1-clausura", "AR", "america", &["team2", "team1"]);
    clausura.priority = 1;
    let mut closing = make_completed_fixture("cl", "team2", "team1", 2, 0);
    closing.date = "2025-12-01".to_string();
    clausura.fixtures = vec![closing];

    game.league = Some(apertura.clone());
    game.competitions = vec![apertura, clausura];
    let before = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1")
        .finance;

    let summary = process_end_of_season(&mut game);

    let team1 = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1");
    let gained = team1.finance - before;
    let payout = game
        .messages
        .iter()
        .find(|message| message.id.starts_with("season_payout_"))
        .expect("a payout message");

    assert_eq!(
        summary.user_position, 2,
        "the closing table is the one the year ended on"
    );
    assert_eq!(team1.history[0].league_position, 2, "and the record agrees");
    assert_eq!(gained, 3_000_000, "paid for second");
    assert_eq!(
        payout.i18n_params.get("amount"),
        Some(&gained.to_string()),
        "the message must describe the payment actually made"
    );
    assert_eq!(
        game.manager.career_stats.trophies, 0,
        "second place is not a trophy"
    );
}

/// A tier that cannot make progress must not hold the career hostage.
///
/// The day loop plays a fixture on the day it is dated and never looks back, so
/// a `Scheduled` fixture the clock has already passed is unreachable: its tier
/// never "ends". Waiting on the user's whole country then meant the season could
/// never complete — `advance_to_next_season` refuses forever and the
/// end-of-season screen never appears. Two years of days did not clear it.
#[test]
fn a_tier_with_only_overdue_fixtures_does_not_block_the_season() {
    let mut game = make_completed_season_game();
    for id in ["team3", "team4"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }

    let mut first = first_division("eng-d1", "ENG", "europe", &["team1", "team2"]);
    first.fixtures = vec![
        make_completed_fixture("d1f1", "team1", "team2", 2, 0),
        make_completed_fixture("d1f2", "team2", "team1", 0, 1),
    ];

    // Started, but its one remaining fixture is dated long before the clock.
    let mut second = first_division("eng-d2", "ENG", "europe", &["team3", "team4"]);
    second.priority = 1;
    let mut stranded = make_completed_fixture("d2-stale", "team3", "team4", 0, 0);
    stranded.date = "2020-01-01".to_string();
    stranded.status = FixtureStatus::Scheduled;
    stranded.result = None;
    second.fixtures = vec![
        make_completed_fixture("d2-played", "team3", "team4", 1, 0),
        stranded,
    ];

    game.league = Some(first.clone());
    game.competitions = vec![first, second];

    assert!(
        is_season_complete(&game),
        "a tier whose only remaining fixture is in the past is stale, not playing"
    );
}

/// The counterpart: a tier that genuinely still has a match to come does block.
#[test]
fn a_tier_with_a_fixture_still_to_come_blocks_the_season() {
    let mut game = make_completed_season_game();
    for id in ["team3", "team4"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }

    let mut first = first_division("eng-d1", "ENG", "europe", &["team1", "team2"]);
    first.fixtures = vec![
        make_completed_fixture("d1f1", "team1", "team2", 2, 0),
        make_completed_fixture("d1f2", "team2", "team1", 0, 1),
    ];

    let mut second = first_division("eng-d2", "ENG", "europe", &["team3", "team4"]);
    second.priority = 1;
    let mut upcoming = make_completed_fixture("d2-upcoming", "team3", "team4", 0, 0);
    upcoming.date = "2026-06-20".to_string(); // the clock sits in May 2026
    upcoming.status = FixtureStatus::Scheduled;
    upcoming.result = None;
    second.fixtures = vec![
        make_completed_fixture("d2-played", "team3", "team4", 1, 0),
        upcoming,
    ];

    game.league = Some(first.clone());
    game.competitions = vec![first, second];

    assert!(
        !is_season_complete(&game),
        "the rest of the pyramid is still playing"
    );
}

/// Only a country's own league tiers are divisions. A regional competition
/// scored as a table was ranked as one, and because it shares a roster with the
/// real division it could win the collapse and take its prize money and its
/// place in the club's history.
#[test]
fn a_regional_table_is_not_a_division_for_prize_money() {
    let mut game = make_completed_season_game();
    let roster = ["team1", "team2"];

    let mut league = first_division("eng-d1", "ENG", "europe", &["team1", "team2"]);
    league.fixtures = vec![make_completed_fixture("d1f", "team1", "team2", 2, 0)];

    // Same clubs, same country, scored as a table — but a regional side
    // competition, not a rung of the pyramid. It finishes later, so if it were
    // admitted it would win the collapse and crown team2 instead.
    let mut regional = first_division("eng-regional", "ENG", "europe", &["team2", "team1"]);
    regional.scope = CompetitionScope::Regional;
    regional.priority = 1;
    let mut later = make_completed_fixture("rf", "team2", "team1", 3, 0);
    later.date = "2026-06-01".to_string();
    regional.fixtures = vec![later];
    let _ = roster;

    game.league = Some(league.clone());
    game.competitions = vec![league, regional];
    let before = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1")
        .finance;

    process_end_of_season(&mut game);

    let team1 = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1");
    assert_eq!(
        team1.finance - before,
        5_000_000,
        "team1 won its division and is paid as champion of it"
    );
    assert_eq!(
        team1.history.len(),
        1,
        "one division, one history entry: {:?}",
        team1.history
    );
    assert_eq!(
        team1.history[0].league_position, 1,
        "the division's table decides the record, not a regional side competition"
    );
}

/// A split-season country plays the same clubs through an Apertura and a
/// Clausura. Counted separately they were ranked as though the second were a
/// tier below the first, so a club banked a top-flight prize for one half and a
/// second-division prize for the other — 7,500,000 for one year — and gained
/// two career-history entries every season.
#[test]
fn a_split_season_pays_and_records_one_division_once() {
    let mut game = make_completed_season_game();
    // Different champions in the two halves, on purpose: team2 wins the
    // Apertura and team1 the Clausura. team1's payout is then 5,000,000 only if
    // the closing table was the one retained — with the same champion in both,
    // keeping either table would satisfy every assertion below.
    let mut apertura = first_division("ar-d1-apertura", "AR", "america", &["team2", "team1"]);
    apertura.priority = 0;
    apertura.fixtures = vec![make_completed_fixture("ap", "team2", "team1", 2, 0)];
    let mut clausura = first_division("ar-d1-clausura", "AR", "america", &["team1", "team2"]);
    clausura.priority = 1;
    let mut closing = make_completed_fixture("cl", "team1", "team2", 1, 0);
    // The Clausura closes the year, so its table is the one that counts.
    closing.date = "2025-12-01".to_string();
    clausura.fixtures = vec![closing];

    game.league = Some(apertura.clone());
    game.competitions = vec![apertura, clausura];
    let before = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1")
        .finance;

    process_end_of_season(&mut game);

    let team1 = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1");
    assert_eq!(
        team1.history.len(),
        1,
        "one season, one history entry: {:?}",
        team1.history
    );
    assert_eq!(
        team1.finance - before,
        5_000_000,
        "the champion is paid once, at top-flight rate"
    );
    assert_eq!(
        team1
            .financial_ledger
            .iter()
            .filter(|row| matches!(row.kind, FinancialTransactionKind::PrizeMoney))
            .count(),
        1,
        "one prize payment"
    );
}

/// A league on another calendar finishes a different season from the user's.
/// Every division's history used to be stamped with the user's season number,
/// so a foreign league's season 1 was recorded as season 2.
#[test]
fn each_division_records_its_own_season() {
    let mut game = make_completed_season_game();
    for id in ["team3", "team4"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }

    let home = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 2,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![make_completed_fixture("hf1", "team1", "team2", 2, 0)],
        standings: vec![
            make_standing("team1", 1, 0, 0, 2, 0),
            make_standing("team2", 0, 0, 1, 0, 2),
        ],
        ..Default::default()
    };
    let foreign = League {
        id: "bra-1".to_string(),
        name: "BRA First Division".to_string(),
        country_id: Some("BR".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("ff1", "team3", "team4", 1, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 1, 0),
            make_standing("team4", 0, 0, 1, 0, 1),
        ],
        ..Default::default()
    };

    game.league = Some(home.clone());
    game.competitions = vec![home, foreign];

    process_end_of_season(&mut game);

    let seasons_for = |id: &str| -> Vec<u32> {
        game.teams
            .iter()
            .find(|team| team.id == id)
            .unwrap_or_else(|| panic!("{id}"))
            .history
            .iter()
            .map(|record| record.season)
            .collect()
    };
    assert_eq!(
        seasons_for("team1"),
        vec![2],
        "the user's league is season 2"
    );
    assert_eq!(
        seasons_for("team3"),
        vec![1],
        "the foreign league finished its own season 1, not the user's season 2"
    );
}

#[test]
fn top_half_finish_receives_expected_prize_money() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team2", 6, 0, 0, 12, 2),
            make_standing("team1", 4, 0, 2, 8, 5),
            make_standing("team3", 2, 0, 4, 4, 8),
            make_standing("team4", 0, 0, 6, 2, 12),
        ];
    }

    let initial_finance = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .unwrap()
        .finance;

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert_eq!(team1.finance, initial_finance + 3_000_000);
}

#[test]
fn lower_table_finish_receives_expected_prize_money() {
    let mut game = make_completed_season_game();

    for i in 3..=10 {
        let team_id = format!("team{}", i);
        game.teams
            .push(make_team(&team_id, &format!("Team{} FC", i)));
    }

    if let Some(league) = &mut game.league {
        let mut standings = Vec::new();

        for i in 2..=10 {
            standings.push(make_standing(&format!("team{}", i), 10, 2, 6, 20, 15));
        }

        standings.push(make_standing("team1", 0, 0, 18, 2, 40));
        league.standings = standings;
    }

    let initial_finance = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .unwrap()
        .finance;

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert_eq!(team1.finance, initial_finance + 150_000);
}

#[test]
fn prize_money_message_sent_once_per_season() {
    let mut game = make_completed_season_game();
    ofm_core::inbox::emit(
        &mut game,
        domain::message::InboxMessage::new(
            "season_payout_1".to_string(),
            "Already exists".to_string(),
            "...".to_string(),
            "Board".to_string(),
            "2026-05-20".to_string(),
        ),
    );

    process_end_of_season(&mut game);

    let payout_messages = game
        .messages
        .iter()
        .filter(|message| message.id == "season_payout_1")
        .count();

    assert_eq!(payout_messages, 1);
}

#[test]
fn season_end_message_sent() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game.messages.iter().find(|m| m.id == "season_end_1");
    assert!(msg.is_some(), "Should send season end message");
    let msg = msg.unwrap();
    assert_eq!(msg.subject, "");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.seasonReview.subject")
    );
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn new_season_schedule_message_sent() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game.messages.iter().find(|m| m.id == "new_season_2");
    assert!(msg.is_some(), "Should send new season message");
    let msg = msg.unwrap();
    assert_eq!(msg.subject, "");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.newSeasonSchedule.subject")
    );
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn messages_not_duplicated() {
    let mut game = make_completed_season_game();
    // Pre-add the messages
    ofm_core::inbox::emit(
        &mut game,
        domain::message::InboxMessage::new(
            "season_end_1".to_string(),
            "Already exists".to_string(),
            "...".to_string(),
            "Board".to_string(),
            "2026-05-20".to_string(),
        ),
    );
    ofm_core::inbox::emit(
        &mut game,
        domain::message::InboxMessage::new(
            "new_season_2".to_string(),
            "Already exists".to_string(),
            "...".to_string(),
            "League".to_string(),
            "2026-05-20".to_string(),
        ),
    );

    process_end_of_season(&mut game);

    let season_end_count = game
        .messages
        .iter()
        .filter(|m| m.id == "season_end_1")
        .count();
    let new_season_count = game
        .messages
        .iter()
        .filter(|m| m.id == "new_season_2")
        .count();
    assert_eq!(
        season_end_count, 1,
        "Should not duplicate season_end message"
    );
    assert_eq!(
        new_season_count, 1,
        "Should not duplicate new_season message"
    );
}
