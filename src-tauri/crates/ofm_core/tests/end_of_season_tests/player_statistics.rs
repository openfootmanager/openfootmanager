use super::*;

/// Given a completed season with nonempty competitive and league statistics, when the season
/// rolls over, then the totals reset and every competition entry is cleared.
#[test]
fn player_stats_reset() {
    let mut game = make_completed_season_game();
    let mut second = game.league.clone().unwrap();
    second.id = "second-completed".into();
    second.priority = 1;
    game.competitions = vec![game.league.clone().unwrap(), second];
    for player in &mut game.players {
        player.league_stats_by_competition.insert(
            "second-completed".into(),
            PlayerSeasonStats {
                appearances: 5,
                goals: 4,
                clean_sheets: 2,
                ..Default::default()
            },
        );
        assert_eq!(player.league_stats_by_competition.len(), 2);
    }
    process_end_of_season(&mut game);

    let p1 = game.players.iter().find(|p| p.id == "p1").unwrap();
    assert_eq!(p1.stats.appearances, 0);
    assert_eq!(p1.stats.goals, 0);
    assert!(p1.league_stats_by_competition.is_empty());
    assert_eq!(p1.stats.assists, 0);
}

/// Given a retained foreign league and completed domestic league with user and AI stints,
/// when the user's season rolls over, then retained statistics and award standings survive.
#[test]
fn a_rollover_preserves_user_and_ai_statistics_for_retained_league_editions() {
    let mut game = make_completed_season_game();
    game.teams
        .extend([make_club("team3", "BR", 500), make_club("team4", "BR", 500)]);
    let mut retained = first_division("retained", "BR", "south-america", &["team3", "team4"]);
    let mut future = make_completed_fixture("future", "team4", "team3", 0, 0);
    future.status = FixtureStatus::Scheduled;
    future.result = None;
    future.date = (game.clock.current_date + chrono::Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();
    assert!(future.date > game.clock.current_date.format("%Y-%m-%d").to_string());
    retained.fixtures = vec![
        make_completed_fixture("past", "team3", "team4", 2, 0),
        future,
    ];
    assert!(!ofm_core::end_of_season::is_regenerated_at_rollover(
        &retained
    ));
    let retained_stats = PlayerSeasonStats {
        appearances: 8,
        goals: 7,
        assists: 3,
        clean_sheets: 2,
        rated_appearances: 6,
        avg_rating: 7.25,
        minutes_played: 640,
        ..Default::default()
    };
    game.players[0]
        .league_stats_by_competition
        .insert("retained".into(), retained_stats.clone());
    let mut ai = make_player("foreign", "Foreign Star", "team3", Position::Forward);
    ai.league_stats_by_competition
        .insert("retained".into(), retained_stats.clone());
    ai.league_stats_by_competition.insert(
        "league1".into(),
        PlayerSeasonStats {
            appearances: 3,
            goals: 2,
            ..Default::default()
        },
    );
    game.players.push(ai);
    game.competitions = vec![game.league.clone().unwrap(), retained.clone()];

    process_end_of_season(&mut game);

    for id in ["p1", "foreign"] {
        let player = game.players.iter().find(|player| player.id == id).unwrap();
        assert_eq!(
            player.league_stats_by_competition.get("retained"),
            Some(&retained_stats)
        );
        assert!(!player.league_stats_by_competition.contains_key("league1"));
    }
    let live_retained = game
        .competitions
        .iter()
        .find(|competition| competition.id == "retained")
        .unwrap();
    assert_eq!(live_retained.season, retained.season);
    assert!(
        live_retained
            .fixtures
            .iter()
            .any(|fixture| fixture.id == "future" && fixture.status == FixtureStatus::Scheduled)
    );
    let awards = ofm_core::season_awards::compute_division_season_awards(&game, live_retained);
    assert_eq!(awards.golden_boot.len(), 2);
    assert!(awards.golden_boot.iter().all(|entry| entry.value == 7.0));
}

/// Given a player released mid-season, with appearances, still unsigned when the season ends
/// When the season ends
/// Then his career entry names no club (no id, no name): the screen words it in the player's
///      language, so no English literal is stored.
#[test]
fn a_free_agent_career_entry_names_no_club() {
    let mut game = make_completed_season_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p1")
        .unwrap()
        .team_id = None;

    process_end_of_season(&mut game);

    let entry = &game.players.iter().find(|p| p.id == "p1").unwrap().career[0];
    assert_eq!(
        (entry.team_id.as_str(), entry.team_name.as_str()),
        ("", ""),
        "a free agent's entry has no club"
    );
}
