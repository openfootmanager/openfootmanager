use super::*;

#[test]
fn continental_fields_requalify_per_region_from_standings_and_cup_winners() {
    // Europe (ES) and South America (BR), each with six clubs whose reputations
    // mirror their finishing order. In each country the cup is won by the 5th-
    // placed club, so it qualifies despite finishing outside the top four.
    let mut teams = Vec::new();
    for (rank, id) in ["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"]
        .iter()
        .enumerate()
    {
        // es-f out-ranks the cup winner es-e on reputation, so a fifth slot filled by reputation
        // rather than by the cup berth would pick es-f and fail the assertions below.
        let reputation = if *id == "es-f" {
            725
        } else {
            900 - rank as u32 * 50
        };
        teams.push(make_club(id, "ES", reputation));
    }
    for (rank, id) in ["br-a", "br-b", "br-c", "br-d", "br-e", "br-f"]
        .iter()
        .enumerate()
    {
        let reputation = if *id == "br-f" {
            725
        } else {
            900 - rank as u32 * 50
        };
        teams.push(make_club(id, "BR", reputation));
    }

    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());

    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);
    game.competitions = vec![
        first_division(
            "es-1",
            "ES",
            "europe",
            &["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"],
        ),
        domestic_cup("es-cup", "ES", "europe", "es-e", "es-f"),
        first_division(
            "br-1",
            "BR",
            "south-america",
            &["br-a", "br-b", "br-c", "br-d", "br-e", "br-f"],
        ),
        domestic_cup("br-cup", "BR", "south-america", "br-e", "br-f"),
        // Five-club fields so the top four plus the cup winner exactly fill each
        // bracket (no reputation top-up masking the cup berth).
        continental_cup("europe-cup", "europe", 5),
        continental_cup("libertadores", "south-america", 5),
    ];

    let europe = game
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();
    let euro_field = continental_qualified_entrants(&game, europe);
    for club in ["es-a", "es-b", "es-c", "es-d", "es-e"] {
        assert!(
            euro_field.contains(&club.to_string()),
            "europe field should include {club}: {euro_field:?}"
        );
    }
    assert!(
        !euro_field.contains(&"es-f".to_string()),
        "es-f neither finished top four nor won the cup: {euro_field:?}"
    );
    assert!(
        euro_field.iter().all(|id| id.starts_with("es-")),
        "europe field drew a club from another confederation: {euro_field:?}"
    );

    let south_america = game
        .competitions
        .iter()
        .find(|c| c.id == "libertadores")
        .unwrap();
    let sa_field = continental_qualified_entrants(&game, south_america);
    for club in ["br-a", "br-b", "br-c", "br-d", "br-e"] {
        assert!(
            sa_field.contains(&club.to_string()),
            "south american field should include {club}: {sa_field:?}"
        );
    }
    assert!(
        sa_field.iter().all(|id| id.starts_with("br-")),
        "south american field drew a club from another confederation: {sa_field:?}"
    );
}

/// Characterization: the built-in default berths (first division positions 1–4
/// + cup winner) reproduce the inferred continental field exactly, so routing
///   the generated world through berths is behavior-preserving.
#[test]
fn default_berths_reproduce_the_inferred_continental_field() {
    let mut teams = Vec::new();
    for (rank, id) in ["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"]
        .iter()
        .enumerate()
    {
        teams.push(make_club(id, "ES", 900 - rank as u32 * 50));
    }

    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());
    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

    let mut first = first_division(
        "es-1",
        "ES",
        "europe",
        &["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"],
    );
    first.berths = vec![Berth {
        target: "europe-cup".to_string(),
        rule: BerthRule::PositionRange { from: 1, to: 4 },
        fallback_to: None,
    }];
    let mut cup = domestic_cup("es-cup", "ES", "europe", "es-e", "es-f");
    cup.berths = vec![Berth {
        target: "europe-cup".to_string(),
        rule: BerthRule::CupWinner,
        fallback_to: None,
    }];
    game.competitions = vec![first, cup, continental_cup("europe-cup", "europe", 5)];

    let europe = game
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();

    assert_eq!(
        berth_qualified_entrants(&game, europe),
        continental_qualified_entrants(&game, europe),
        "default berths must reproduce the inferred field"
    );
}

/// A cup that has already been renewed for its next edition: unplayed, one season on, with the
/// edition it just finished frozen in the archive under `winner`.
fn renewed_cup_with_archived_winner(game: &mut Game, cup: &mut League, winner: &str) {
    game.edition_archive
        .push(domain::edition_archive::CompletedEdition {
            competition_id: cup.id.clone(),
            season: cup.season,
            completed_on: "2026-05-10".to_string(),
            champion_id: winner.to_string(),
            participant_ids: vec![],
            standings: vec![],
            groups: vec![],
            knockout_rounds: vec![],
            fixtures: vec![],
        });
    cup.season += 1;
    for fixture in &mut cup.fixtures {
        fixture.status = FixtureStatus::Scheduled;
        fixture.result = None;
    }
    for round in &mut cup.knockout_rounds {
        round.completed = false;
    }
}

fn europe_game_with_a_renewed_cup(berth: bool) -> Game {
    let teams = ["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"]
        .iter()
        .enumerate()
        .map(|(rank, id)| make_club(id, "ES", 900 - rank as u32 * 50))
        .collect();
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());
    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);
    let mut cup = domestic_cup("es-cup", "ES", "europe", "es-f", "es-e");
    if berth {
        cup.berths = vec![Berth {
            target: "europe-cup".to_string(),
            rule: BerthRule::CupWinner,
            fallback_to: None,
        }];
    }
    renewed_cup_with_archived_winner(&mut game, &mut cup, "es-f");
    game.competitions = vec![
        first_division(
            "es-1",
            "ES",
            "europe",
            &["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"],
        ),
        cup,
        continental_cup("europe-cup", "europe", 5),
    ];
    game
}

/// Given a domestic cup that was archived and renewed before the rollover, so its live bracket is
/// the next edition and unplayed,
/// When the continental field is worked out,
/// Then the winner of the edition that finished still qualifies: the archive holds it.
#[test]
fn a_cup_renewed_before_the_rollover_still_sends_its_winner_to_continental_qualification() {
    let game = europe_game_with_a_renewed_cup(false);
    let europe = game
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();

    let field = continental_qualified_entrants(&game, europe);

    assert!(
        field.contains(&"es-f".to_string()),
        "the cup winner is missing: {field:?}"
    );
}

/// As above, for a data-defined `CupWinner` berth.
#[test]
fn a_renewed_cups_cup_winner_berth_reads_the_archived_champion() {
    let game = europe_game_with_a_renewed_cup(true);
    let europe = game
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();

    let field = berth_qualified_entrants(&game, europe);

    assert!(
        field.contains(&"es-f".to_string()),
        "the cup winner is missing: {field:?}"
    );
}

/// Given a cup the rollover keeps because its final was postponed past the league's last round, so
/// its edition is the one being settled, unfinished, and last season's champion is in the archive,
/// When the continental field is worked out,
/// Then last season's champion is not credited again: the cup contributes nobody this season.
#[test]
fn a_kept_cup_with_an_undecided_final_does_not_credit_last_seasons_champion_again() {
    for berth in [false, true] {
        let mut game = europe_game_with_a_renewed_cup(berth);
        // The edition being settled: same season as the league, its final still to be played.
        let league_season = game.competitions[0].season;
        game.competitions[1].season = league_season;
        game.edition_archive[0].season = league_season - 1;
        let europe = game
            .competitions
            .iter()
            .find(|c| c.id == "europe-cup")
            .unwrap();

        let field = if berth {
            berth_qualified_entrants(&game, europe)
        } else {
            continental_qualified_entrants(&game, europe)
        };

        // es-f is the weakest club in the league, so only the cup could have qualified it.
        assert!(
            !field.contains(&"es-f".to_string()),
            "last season's champion qualified again (berth: {berth}): {field:?}"
        );
    }
}

/// A game like `europe_game_with_a_renewed_cup`, with the league and the cup's seasons moved to
/// `league_season`, a foreign first division in front of the league by rank, and its own season.
fn europe_game_in_season(league_season: u32, berth: bool) -> Game {
    let mut game = europe_game_with_a_renewed_cup(berth);
    game.competitions[0].season = league_season;
    // The league ranks below the cup by priority, so a lookup that lets the cup (or any
    // competition of another kind) stand in for the league finds the wrong season.
    game.competitions[0].priority = 1;
    game.competitions[1].season = league_season + 1;
    game.edition_archive[0].season = league_season;
    let mut foreign = first_division("br-1", "BR", "south-america", &["br-a", "br-b"]);
    foreign.season = league_season + 5;
    foreign.priority = 0; // outranks the Spanish league by priority
    game.competitions.push(foreign);
    game
}

/// Given a cup renewed past its league's season, in a world whose other country's first division
/// is on a much later season,
/// When the continental field is worked out,
/// Then the cup is compared with its own country's league, not another's: its winner qualifies.
#[test]
fn a_renewed_cup_is_compared_with_its_own_countrys_league() {
    for berth in [false, true] {
        let game = europe_game_in_season(7, berth);
        let europe = game
            .competitions
            .iter()
            .find(|c| c.id == "europe-cup")
            .unwrap();

        let field = if berth {
            berth_qualified_entrants(&game, europe)
        } else {
            continental_qualified_entrants(&game, europe)
        };

        assert!(
            field.contains(&"es-f".to_string()),
            "berth {berth}: {field:?}"
        );
    }
}

/// Given a cup kept at the rollover in a late season (not season 1), When the field is worked
/// out, Then last season's champion is still not credited.
#[test]
fn a_kept_cup_in_a_late_season_does_not_credit_the_archive() {
    for berth in [false, true] {
        let mut game = europe_game_in_season(7, berth);
        game.competitions[1].season = 7; // not renewed: the edition being settled
        game.edition_archive[0].season = 6;
        let europe = game
            .competitions
            .iter()
            .find(|c| c.id == "europe-cup")
            .unwrap();

        let field = if berth {
            berth_qualified_entrants(&game, europe)
        } else {
            continental_qualified_entrants(&game, europe)
        };

        assert!(
            !field.contains(&"es-f".to_string()),
            "berth {berth}: {field:?}"
        );
    }
}

/// Given an unplayed cup that was never archived (a first edition), When the field is worked out,
/// Then it has no winner to send: nothing is invented.
#[test]
fn a_cup_with_no_archived_edition_and_no_winner_sends_nobody() {
    let mut game = europe_game_with_a_renewed_cup(false);
    game.edition_archive.clear();
    let europe = game
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();

    let field = continental_qualified_entrants(&game, europe);

    // es-f finished last in a six-club league and is the weakest club, so only the cup could
    // have qualified it.
    assert!(!field.contains(&"es-f".to_string()), "{field:?}");
    // The data-defined path agrees: with no live winner and no archived edition, a CupWinner
    // berth awards nobody either.
    let mut with_berth = europe_game_with_a_renewed_cup(true);
    with_berth.edition_archive.clear();
    let europe = with_berth
        .competitions
        .iter()
        .find(|c| c.id == "europe-cup")
        .unwrap();
    assert!(!berth_qualified_entrants(&with_berth, europe).contains(&"es-f".to_string()));
}

/// Multi-tier qualification: a club lands in only its most prestigious target,
/// so a cup winner who'd otherwise be a Europa club via league position is
/// promoted to the Champions Cup and dropped from Europa.
#[test]
fn berths_keep_each_club_in_its_most_prestigious_continental_target() {
    let mut teams = Vec::new();
    for (rank, id) in ["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"]
        .iter()
        .enumerate()
    {
        teams.push(make_club(id, "ES", 900 - rank as u32 * 50));
    }
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());
    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

    let mut first = first_division(
        "es-1",
        "ES",
        "europe",
        &["es-a", "es-b", "es-c", "es-d", "es-e", "es-f"],
    );
    first.berths = vec![
        Berth {
            target: "ucl".to_string(),
            rule: BerthRule::PositionRange { from: 1, to: 2 },
            fallback_to: None,
        },
        Berth {
            target: "uel".to_string(),
            rule: BerthRule::PositionRange { from: 3, to: 4 },
            fallback_to: None,
        },
    ];
    // The cup is won by the 4th-placed club, whose cup berth outranks its Europa
    // league berth.
    let mut cup = domestic_cup("es-cup", "ES", "europe", "es-d", "es-e");
    cup.berths = vec![Berth {
        target: "ucl".to_string(),
        rule: BerthRule::CupWinner,
        fallback_to: Some("uel".to_string()),
    }];

    let mut ucl = continental_cup("ucl", "europe", 3);
    ucl.priority = 100;
    let mut uel = continental_cup("uel", "europe", 2);
    uel.priority = 101;
    game.competitions = vec![first, cup, ucl, uel];

    let fields = resolve_continental_fields(&game);
    let ucl_field = &fields["ucl"];
    let uel_field = &fields["uel"];

    assert!(
        ucl_field.contains(&"es-d".to_string()),
        "cup winner promoted to the top tier: {ucl_field:?}"
    );
    assert!(
        !uel_field.contains(&"es-d".to_string()),
        "a club sits in only one continental tier: {uel_field:?}"
    );
    assert!(ucl_field.contains(&"es-a".to_string()) && ucl_field.contains(&"es-b".to_string()));
    assert!(uel_field.contains(&"es-c".to_string()));
}

/// `fallbackTo` as the *only* route into a field: the cup winner's primary
/// target does not exist in this world, and it is both outside the league's
/// qualifying positions and the worst-reputation club, so neither a position
/// berth nor the reputation top-up would reach it.
#[test]
fn a_continental_fallback_places_a_club_no_other_route_would() {
    let clubs = [
        "es-a", "es-b", "es-c", "es-d", "es-e", "es-f", "es-g", "es-h", "es-i",
    ];
    let mut teams: Vec<Team> = clubs
        .iter()
        .enumerate()
        .map(|(rank, id)| make_club(id, "ES", 900 - rank as u32 * 50))
        .collect();
    // The cup winner is the least-reputable club in the country, so a thin
    // field would top up with es-i long before it reached es-h.
    teams[7].reputation = 10;

    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("es-a".to_string());
    let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

    let mut first = first_division("es-1", "ES", "europe", &clubs);
    first.berths = vec![
        Berth {
            target: "ucl".to_string(),
            rule: BerthRule::PositionRange { from: 1, to: 4 },
            fallback_to: None,
        },
        Berth {
            target: "uel".to_string(),
            rule: BerthRule::PositionRange { from: 5, to: 7 },
            fallback_to: None,
        },
    ];
    // 8th-placed es-h wins the cup, but the cup's primary target is not part
    // of this world — only the fallback can seat it.
    let mut cup = domestic_cup("es-cup", "ES", "europe", "es-h", "es-i");
    cup.berths = vec![Berth {
        target: "super-cup".to_string(),
        rule: BerthRule::CupWinner,
        fallback_to: Some("uel".to_string()),
    }];

    let mut ucl = continental_cup("ucl", "europe", 4);
    ucl.priority = 100;
    let mut uel = continental_cup("uel", "europe", 4);
    uel.priority = 101;
    game.competitions = vec![first, cup, ucl, uel];

    let fields = resolve_continental_fields(&game);
    let uel_field = &fields["uel"];

    assert!(
        uel_field.contains(&"es-h".to_string()),
        "the cup winner takes its fallback place: {uel_field:?}"
    );
    assert!(
        !uel_field.contains(&"es-i".to_string()),
        "es-h fills the fourth slot through the fallback, not through the \
         reputation top-up that would have taken es-i: {uel_field:?}"
    );
    assert!(
        !fields["ucl"].contains(&"es-h".to_string()),
        "an unresolvable primary target seats nobody: {:?}",
        fields["ucl"]
    );
}

#[test]
fn region_for_country_prefers_world_data_over_the_catalog() {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "m".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
    // A data-defined confederation: ZZ sits in "galaxy", declared by its league.
    game.competitions = vec![first_division("zz-1", "ZZ", "galaxy", &["zz-a", "zz-b"])];

    assert_eq!(
        game.region_for_country("ZZ"),
        "galaxy",
        "the world's own data wins over the catalog"
    );
    assert_eq!(
        game.region_for_country("ES"),
        "europe",
        "countries the world doesn't place fall back to the catalog"
    );
    assert_eq!(game.region_for_country("BR"), "south-america");
}

#[test]
fn rollover_passes_continental_qualifiers_into_the_next_season() {
    // 2027 is neither a World Cup summer nor a qualifying season, so the rollover
    // just regenerates club competitions.
    let clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 20, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("eng-a".to_string());

    let teams = vec![
        make_club("eng-a", "ENG", 800),
        make_club("eng-b", "ENG", 700),
    ];
    let mut p1 = make_player("p1", "Star", "eng-a", Position::Forward);
    p1.stats = PlayerSeasonStats {
        appearances: 30,
        goals: 20,
        ..PlayerSeasonStats::default()
    };
    p1.league_stats_by_competition
        .insert("eng-1".into(), p1.stats.clone());
    let mut p2 = make_player("p2", "Rival", "eng-b", Position::Forward);
    p2.stats = PlayerSeasonStats {
        appearances: 28,
        goals: 8,
        ..PlayerSeasonStats::default()
    };
    p2.league_stats_by_competition
        .insert("eng-1".into(), p2.stats.clone());

    let league = League {
        id: "eng-1".to_string(),
        name: "England League".to_string(),
        season: 1,
        kind: CompetitionType::League,
        scope: CompetitionScope::Domestic,
        country_id: Some("ENG".to_string()),
        region_id: Some("europe".to_string()),
        participant_ids: vec!["eng-a".to_string(), "eng-b".to_string()],
        fixtures: vec![
            make_completed_fixture("f1", "eng-a", "eng-b", 2, 0),
            make_completed_fixture("f2", "eng-b", "eng-a", 0, 1),
        ],
        // eng-a won both: 6 pts; eng-b: 0.
        standings: vec![
            make_standing("eng-a", 2, 0, 0, 3, 0),
            make_standing("eng-b", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let continental = continental_cup("ucl", "europe", 2);

    let mut game = Game::new(clock, manager, teams, vec![p1, p2], vec![], vec![]);
    game.competitions = vec![league, continental];

    process_end_of_season(&mut game);

    let ucl = game.competitions.iter().find(|c| c.id == "ucl").unwrap();
    assert!(
        ucl.participant_ids.contains(&"eng-a".to_string())
            && ucl.participant_ids.contains(&"eng-b".to_string()),
        "next season's continental field should be the domestic qualifiers, got {:?}",
        ucl.participant_ids
    );
    assert!(
        !ucl.participant_ids.iter().any(|id| id.starts_with("seed-")),
        "placeholder seeds should be replaced by real qualifiers: {:?}",
        ucl.participant_ids
    );
}
