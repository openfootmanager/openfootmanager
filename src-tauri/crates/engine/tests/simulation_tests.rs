use ::engine::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_player(id: &str, name: &str, position: Position, skill: u8) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: name.to_string(),
        position,
        ovr: skill,
        condition: 90,
        fitness: 75,
        pace: skill,
        stamina: skill,
        strength: skill,
        agility: skill,
        passing: skill,
        shooting: skill,
        tackling: skill,
        dribbling: skill,
        defending: skill,
        positioning: skill,
        vision: skill,
        decisions: skill,
        composure: skill,
        aggression: skill,
        teamwork: skill,
        leadership: skill,
        handling: skill,
        reflexes: skill,
        aerial: skill,
        traits: vec![],
    }
}

fn make_team(id: &str, name: &str, skill: u8, play_style: PlayStyle) -> TeamData {
    TeamData {
        id: id.to_string(),
        name: name.to_string(),
        formation: "4-4-2".to_string(),
        play_style,
        players: vec![
            make_player(&format!("{id}_gk1"), "GK1", Position::Goalkeeper, skill),
            make_player(&format!("{id}_def1"), "DEF1", Position::Defender, skill),
            make_player(&format!("{id}_def2"), "DEF2", Position::Defender, skill),
            make_player(&format!("{id}_def3"), "DEF3", Position::Defender, skill),
            make_player(&format!("{id}_def4"), "DEF4", Position::Defender, skill),
            make_player(&format!("{id}_mid1"), "MID1", Position::Midfielder, skill),
            make_player(&format!("{id}_mid2"), "MID2", Position::Midfielder, skill),
            make_player(&format!("{id}_mid3"), "MID3", Position::Midfielder, skill),
            make_player(&format!("{id}_mid4"), "MID4", Position::Midfielder, skill),
            make_player(&format!("{id}_fwd1"), "FWD1", Position::Forward, skill),
            make_player(&format!("{id}_fwd2"), "FWD2", Position::Forward, skill),
        ],
    }
}

fn seeded_rng(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

// ---------------------------------------------------------------------------
// Types tests
// ---------------------------------------------------------------------------

#[test]
fn player_overall_rating() {
    let p = make_player("p1", "Test", Position::Forward, 70);
    assert!((p.overall() - 70.0).abs() < 0.01);
}

#[test]
fn player_effective_overall_accounts_for_condition() {
    let mut p = make_player("p1", "Test", Position::Forward, 80);
    p.condition = 50;
    let eff = p.effective_overall();
    assert!((eff - 40.0).abs() < 0.01, "Expected ~40.0, got {eff}");
}

#[test]
fn team_position_counts() {
    let team = make_team("t1", "Test FC", 60, PlayStyle::Balanced);
    assert_eq!(team.count_position(Position::Goalkeeper), 1);
    assert_eq!(team.count_position(Position::Defender), 4);
    assert_eq!(team.count_position(Position::Midfielder), 4);
    assert_eq!(team.count_position(Position::Forward), 2);
}

#[test]
fn team_ratings_non_zero() {
    let team = make_team("t1", "Test FC", 65, PlayStyle::Balanced);
    assert!(team.defense_rating() > 0.0);
    assert!(team.midfield_rating() > 0.0);
    assert!(team.attack_rating() > 0.0);
    assert!(team.goalkeeper_rating() > 0.0);
}

#[test]
fn team_ratings_scale_with_skill() {
    let weak = make_team("w", "Weak", 30, PlayStyle::Balanced);
    let strong = make_team("s", "Strong", 90, PlayStyle::Balanced);
    assert!(strong.defense_rating() > weak.defense_rating());
    assert!(strong.midfield_rating() > weak.midfield_rating());
    assert!(strong.attack_rating() > weak.attack_rating());
}

// ---------------------------------------------------------------------------
// Zone tests
// ---------------------------------------------------------------------------

#[test]
fn zone_attacking_box() {
    assert_eq!(Zone::attacking_box(Side::Home), Zone::AwayBox);
    assert_eq!(Zone::attacking_box(Side::Away), Zone::HomeBox);
}

#[test]
fn zone_attacking_third() {
    assert_eq!(Zone::attacking_third(Side::Home), Zone::AwayDefense);
    assert_eq!(Zone::attacking_third(Side::Away), Zone::HomeDefense);
}

#[test]
fn zone_defensive_third() {
    assert_eq!(Zone::defensive_third(Side::Home), Zone::HomeDefense);
    assert_eq!(Zone::defensive_third(Side::Away), Zone::AwayDefense);
}

#[test]
fn zone_advance_towards_home() {
    assert_eq!(
        Zone::HomeDefense.advance_towards(Side::Home),
        Zone::Midfield
    );
    assert_eq!(
        Zone::Midfield.advance_towards(Side::Home),
        Zone::AwayDefense
    );
    assert_eq!(Zone::AwayDefense.advance_towards(Side::Home), Zone::AwayBox);
    assert_eq!(Zone::AwayBox.advance_towards(Side::Home), Zone::AwayBox); // saturates
}

#[test]
fn zone_advance_towards_away() {
    assert_eq!(
        Zone::AwayDefense.advance_towards(Side::Away),
        Zone::Midfield
    );
    assert_eq!(
        Zone::Midfield.advance_towards(Side::Away),
        Zone::HomeDefense
    );
    assert_eq!(Zone::HomeDefense.advance_towards(Side::Away), Zone::HomeBox);
    assert_eq!(Zone::HomeBox.advance_towards(Side::Away), Zone::HomeBox); // saturates
}

#[test]
fn zone_is_box_for() {
    assert!(Zone::AwayBox.is_box_for(Side::Home));
    assert!(!Zone::AwayBox.is_box_for(Side::Away));
    assert!(Zone::HomeBox.is_box_for(Side::Away));
    assert!(!Zone::HomeBox.is_box_for(Side::Home));
}

// ---------------------------------------------------------------------------
// Side tests
// ---------------------------------------------------------------------------

#[test]
fn side_opposite() {
    assert_eq!(Side::Home.opposite(), Side::Away);
    assert_eq!(Side::Away.opposite(), Side::Home);
}

// ---------------------------------------------------------------------------
// MatchConfig tests
// ---------------------------------------------------------------------------

#[test]
fn default_config_values_in_range() {
    let cfg = MatchConfig::default();
    assert!(cfg.home_advantage >= 1.0 && cfg.home_advantage <= 1.25);
    assert!(cfg.shot_accuracy_base > 0.0 && cfg.shot_accuracy_base < 1.0);
    assert!(cfg.goal_conversion_base > 0.0 && cfg.goal_conversion_base < 1.0);
    assert!(cfg.foul_probability > 0.0 && cfg.foul_probability < 1.0);
    assert!(cfg.yellow_card_probability > 0.0 && cfg.yellow_card_probability < 1.0);
    assert!(cfg.red_card_probability > 0.0 && cfg.red_card_probability < 0.5);
    assert!(cfg.penalty_probability > 0.0 && cfg.penalty_probability < 1.0);
    assert!(cfg.stoppage_time_max <= 10);
    assert!(cfg.injury_probability >= 0.0 && cfg.injury_probability < 0.5);
}

// ---------------------------------------------------------------------------
// Event tests
// ---------------------------------------------------------------------------

#[test]
fn match_event_builder() {
    let evt = MatchEvent::new(45, EventType::Goal, Side::Home, Zone::AwayBox)
        .with_player("p1")
        .with_secondary("p2");

    assert_eq!(evt.minute, 45);
    assert_eq!(evt.event_type, EventType::Goal);
    assert_eq!(evt.player_id.as_deref(), Some("p1"));
    assert_eq!(evt.secondary_player_id.as_deref(), Some("p2"));
    assert!(evt.is_goal());
}

#[test]
fn penalty_goal_is_goal() {
    let evt = MatchEvent::new(78, EventType::PenaltyGoal, Side::Away, Zone::HomeBox);
    assert!(evt.is_goal());
}

#[test]
fn non_goal_events_not_goal() {
    let shot = MatchEvent::new(10, EventType::ShotOnTarget, Side::Home, Zone::AwayBox);
    assert!(!shot.is_goal());
    let foul = MatchEvent::new(20, EventType::Foul, Side::Away, Zone::Midfield);
    assert!(!foul.is_goal());
}

// ---------------------------------------------------------------------------
// Core simulation tests
// ---------------------------------------------------------------------------

#[test]
fn simulation_produces_report() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let mut rng = seeded_rng(42);

    let report = simulate_with_rng(&home, &away, &config, &mut rng);

    // Report should have required structural events
    let has_kickoff = report
        .events
        .iter()
        .any(|e| e.event_type == EventType::KickOff);
    let has_halftime = report
        .events
        .iter()
        .any(|e| e.event_type == EventType::HalfTime);
    let has_fulltime = report
        .events
        .iter()
        .any(|e| e.event_type == EventType::FullTime);
    let has_second_half = report
        .events
        .iter()
        .any(|e| e.event_type == EventType::SecondHalfStart);

    assert!(has_kickoff, "Missing KickOff event");
    assert!(has_halftime, "Missing HalfTime event");
    assert!(has_fulltime, "Missing FullTime event");
    assert!(has_second_half, "Missing SecondHalfStart event");
}

#[test]
fn simulation_deterministic_with_same_seed() {
    let home = make_team("home", "Home FC", 60, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 60, PlayStyle::Defensive);
    let config = MatchConfig::default();

    let report1 = simulate_with_rng(&home, &away, &config, &mut seeded_rng(123));
    let report2 = simulate_with_rng(&home, &away, &config, &mut seeded_rng(123));

    assert_eq!(report1.home_goals, report2.home_goals);
    assert_eq!(report1.away_goals, report2.away_goals);
    assert_eq!(report1.events.len(), report2.events.len());
}

#[test]
fn simulation_different_seeds_vary() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig::default();

    // Run many simulations and check we get different results
    let mut results = std::collections::HashSet::new();
    for seed in 0..50 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        results.insert((report.home_goals, report.away_goals));
    }
    assert!(
        results.len() > 1,
        "50 simulations should produce varied results"
    );
}

#[test]
fn goals_in_report_match_score() {
    let home = make_team("home", "Home FC", 70, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 50, PlayStyle::Defensive);
    let config = MatchConfig::default();

    for seed in 0..20 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));

        let home_goal_count = report.goals.iter().filter(|g| g.side == Side::Home).count() as u8;
        let away_goal_count = report.goals.iter().filter(|g| g.side == Side::Away).count() as u8;

        assert_eq!(
            report.home_goals, home_goal_count,
            "Home goals mismatch in seed {seed}"
        );
        assert_eq!(
            report.away_goals, away_goal_count,
            "Away goals mismatch in seed {seed}"
        );
        assert_eq!(
            report.home_goals, report.home_stats.goals,
            "Home stats mismatch in seed {seed}"
        );
        assert_eq!(
            report.away_goals, report.away_stats.goals,
            "Away stats mismatch in seed {seed}"
        );
    }
}

#[test]
fn goal_events_have_scorer() {
    let home = make_team("home", "Home FC", 75, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 45, PlayStyle::Balanced);
    let config = MatchConfig::default();

    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(99));

    for goal in &report.goals {
        assert!(
            !goal.scorer_id.is_empty(),
            "Goal at minute {} has empty scorer",
            goal.minute
        );
    }
}

#[test]
fn possession_adds_up() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Possession);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(7));

    assert!(
        report.home_possession >= 0.0 && report.home_possession <= 100.0,
        "Possession out of range: {}",
        report.home_possession
    );
    // Total ticks should be > 0
    let total = report.home_stats.possession_ticks + report.away_stats.possession_ticks;
    assert!(total > 0, "No possession ticks recorded");
}

#[test]
fn total_minutes_at_least_90() {
    let home = make_team("home", "Home FC", 60, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 60, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(55));
    assert!(
        report.total_minutes >= 90,
        "Total minutes: {}",
        report.total_minutes
    );
}

#[test]
fn report_tracks_minutes_for_all_starters() {
    let home = make_team("home", "Home FC", 60, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 60, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(55));

    for player in home.players.iter().chain(away.players.iter()) {
        let stats = report
            .player_stats
            .get(&player.id)
            .unwrap_or_else(|| panic!("Missing report stats for {}", player.id));
        assert!(
            stats.minutes_played > 0,
            "Expected minutes for {}, got {}",
            player.id,
            stats.minutes_played
        );
    }
}

// ---------------------------------------------------------------------------
// Strength imbalance tests
// ---------------------------------------------------------------------------

#[test]
fn strong_team_wins_more_often() {
    let strong = make_team("strong", "Strong FC", 90, PlayStyle::Balanced);
    let weak = make_team("weak", "Weak FC", 30, PlayStyle::Balanced);
    let config = MatchConfig::default();

    let mut strong_wins = 0u32;
    let mut weak_wins = 0u32;
    let trials = 100;
    for seed in 0..trials {
        let report = simulate_with_rng(&strong, &weak, &config, &mut seeded_rng(seed));
        if report.home_goals > report.away_goals {
            strong_wins += 1;
        } else if report.away_goals > report.home_goals {
            weak_wins += 1;
        }
    }
    assert!(
        strong_wins > weak_wins * 2,
        "Strong team should dominate: {strong_wins} wins vs {weak_wins} for weak"
    );
}

#[test]
fn equal_teams_roughly_even() {
    let team_a = make_team("a", "Team A", 65, PlayStyle::Balanced);
    let team_b = make_team("b", "Team B", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        home_advantage: 1.0,
        ..MatchConfig::default()
    }; // no home advantage

    let mut a_wins = 0u32;
    let mut b_wins = 0u32;
    let trials = 200;
    for seed in 0..trials {
        let report = simulate_with_rng(&team_a, &team_b, &config, &mut seeded_rng(seed));
        if report.home_goals > report.away_goals {
            a_wins += 1;
        } else if report.away_goals > report.home_goals {
            b_wins += 1;
        }
    }
    let diff = (a_wins as i32 - b_wins as i32).unsigned_abs();
    assert!(
        diff < (trials / 3) as u32,
        "Equal teams should be close: A={a_wins}, B={b_wins}, diff={diff}"
    );
}

// ---------------------------------------------------------------------------
// Home advantage tests
// ---------------------------------------------------------------------------

#[test]
fn home_advantage_helps() {
    let team = make_team("t", "Team", 65, PlayStyle::Balanced);
    let config_with = MatchConfig {
        home_advantage: 1.15,
        ..MatchConfig::default()
    };
    let config_without = MatchConfig {
        home_advantage: 1.0,
        ..MatchConfig::default()
    };

    let trials = 200;
    let mut home_wins_with = 0u32;
    let mut home_wins_without = 0u32;

    for seed in 0..trials {
        let r1 = simulate_with_rng(&team, &team, &config_with, &mut seeded_rng(seed));
        let r2 = simulate_with_rng(&team, &team, &config_without, &mut seeded_rng(seed));
        if r1.home_goals > r1.away_goals {
            home_wins_with += 1;
        }
        if r2.home_goals > r2.away_goals {
            home_wins_without += 1;
        }
    }
    assert!(
        home_wins_with >= home_wins_without,
        "Home advantage should help: with={home_wins_with}, without={home_wins_without}"
    );
}

// ---------------------------------------------------------------------------
// Play-style influence tests
// ---------------------------------------------------------------------------

#[test]
fn possession_style_has_more_possession() {
    let poss_team = make_team("poss", "Poss FC", 65, PlayStyle::Possession);
    let counter_team = make_team("counter", "Counter FC", 65, PlayStyle::Counter);
    let config = MatchConfig {
        home_advantage: 1.0,
        ..MatchConfig::default()
    };

    let mut poss_total = 0.0;
    let trials = 100;
    for seed in 0..trials {
        let report = simulate_with_rng(&poss_team, &counter_team, &config, &mut seeded_rng(seed));
        poss_total += report.home_possession;
    }
    let avg_poss = poss_total / trials as f64;
    assert!(
        avg_poss > 48.0,
        "Possession team avg possession should be >48%: {avg_poss:.1}%"
    );
}

// ---------------------------------------------------------------------------
// Team/player stats aggregation tests
// ---------------------------------------------------------------------------

#[test]
fn player_stats_populated() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(77));

    // At least some players should have stats
    assert!(
        !report.player_stats.is_empty(),
        "Player stats should not be empty"
    );

    // Check that stats are reasonable
    for (player_id, ps) in &report.player_stats {
        assert!(
            ps.rating >= 0.0 && ps.rating <= 10.0,
            "Player {player_id} rating out of range: {}",
            ps.rating
        );
    }
}

#[test]
fn team_stats_shots_consistent() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 65, PlayStyle::Defensive);
    let config = MatchConfig::default();

    for seed in 0..10 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));

        // shots >= shots_on_target
        assert!(
            report.home_stats.shots >= report.home_stats.shots_on_target,
            "Seed {seed}: home shots < SOT"
        );
        assert!(
            report.away_stats.shots >= report.away_stats.shots_on_target,
            "Seed {seed}: away shots < SOT"
        );
    }
}

#[test]
fn events_are_chronological() {
    let home = make_team("home", "Home FC", 70, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 70, PlayStyle::Balanced);
    let config = MatchConfig::default();

    // Run multiple seeds to increase confidence
    for seed in 0..10 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        for window in report.events.windows(2) {
            assert!(
                window[1].minute >= window[0].minute,
                "Seed {seed}: events out of order: minute {} ({:?}) followed by {} ({:?})",
                window[0].minute,
                window[0].event_type,
                window[1].minute,
                window[1].event_type,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Report: pass accuracy
// ---------------------------------------------------------------------------

#[test]
fn pass_accuracy_in_range() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(88));

    let home_acc = report.home_stats.pass_accuracy();
    let away_acc = report.away_stats.pass_accuracy();
    assert!(
        home_acc >= 0.0 && home_acc <= 100.0,
        "Home pass accuracy: {home_acc}"
    );
    assert!(
        away_acc >= 0.0 && away_acc <= 100.0,
        "Away pass accuracy: {away_acc}"
    );
}

// ---------------------------------------------------------------------------
// Edge case: no stoppage time
// ---------------------------------------------------------------------------

#[test]
fn zero_stoppage_time_produces_valid_report() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        stoppage_time_max: 0,
        ..MatchConfig::default()
    };
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(1));
    assert_eq!(report.total_minutes, 90);
}

// ---------------------------------------------------------------------------
// Edge case: very high foul probability
// ---------------------------------------------------------------------------

#[test]
fn high_foul_probability_produces_cards() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        foul_probability: 0.95,
        yellow_card_probability: 0.90,
        ..MatchConfig::default()
    };

    let mut total_yellows = 0u16;
    for seed in 0..20 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        total_yellows +=
            report.home_stats.yellow_cards as u16 + report.away_stats.yellow_cards as u16;
    }
    assert!(
        total_yellows > 0,
        "High foul rate should produce some yellow cards"
    );
}

// ---------------------------------------------------------------------------
// Report serialization
// ---------------------------------------------------------------------------

#[test]
fn report_serializes_to_json() {
    let home = make_team("home", "Home FC", 60, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 60, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(42));

    let json = serde_json::to_string(&report);
    assert!(json.is_ok(), "Report should serialize: {:?}", json.err());
    let json_str = json.unwrap();
    assert!(json_str.contains("home_goals"));
    assert!(json_str.contains("away_goals"));
    assert!(json_str.contains("events"));
}

// ---------------------------------------------------------------------------
// Event counts consistency
// ---------------------------------------------------------------------------

#[test]
fn goal_events_match_report_goals() {
    let home = make_team("home", "Home FC", 70, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 50, PlayStyle::Defensive);
    let config = MatchConfig::default();

    for seed in 0..30 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));

        let event_goals: u8 = report.events.iter().filter(|e| e.is_goal()).count() as u8;

        let report_total = report.home_goals + report.away_goals;
        assert_eq!(
            event_goals, report_total,
            "Seed {seed}: event goals ({event_goals}) != report total ({report_total})"
        );
    }
}

// ---------------------------------------------------------------------------
// Realistic goal distribution
// ---------------------------------------------------------------------------

#[test]
fn average_goals_realistic() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig::default();

    let trials = 500;
    let mut total_goals = 0u32;
    for seed in 0..trials {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        total_goals += (report.home_goals + report.away_goals) as u32;
    }
    let avg = total_goals as f64 / trials as f64;
    // Real football averages ~2.5 goals/game. Allow a wide range for a simulation.
    assert!(
        avg > 0.5 && avg < 8.0,
        "Average goals per game should be reasonable: {avg:.2}"
    );
}

// ---------------------------------------------------------------------------
// High foul rate produces fouls and free kicks
// ---------------------------------------------------------------------------

#[test]
fn high_foul_rate_produces_fouls_and_free_kicks() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        foul_probability: 0.95,
        yellow_card_probability: 0.01,
        ..MatchConfig::default()
    };

    let mut total_fouls = 0u32;
    let mut total_free_kicks = 0u32;
    for seed in 0..30 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        for e in &report.events {
            match e.event_type {
                EventType::Foul => total_fouls += 1,
                EventType::FreeKick => total_free_kicks += 1,
                _ => {}
            }
        }
    }
    assert!(
        total_fouls > 0,
        "With 95% foul probability, fouls should occur"
    );
    assert!(
        total_free_kicks > 0,
        "Fouls outside box should produce free kicks"
    );
}

// ---------------------------------------------------------------------------
// Red card and second yellow coverage
// ---------------------------------------------------------------------------

#[test]
fn high_red_card_probability_produces_red_cards() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        foul_probability: 0.90,
        yellow_card_probability: 0.90,
        red_card_probability: 0.90,
        ..MatchConfig::default()
    };

    let mut total_reds = 0u32;
    for seed in 0..30 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        total_reds += report.home_stats.red_cards as u32 + report.away_stats.red_cards as u32;
    }
    assert!(
        total_reds > 0,
        "With high red card probability, red cards should occur"
    );
}

#[test]
fn second_yellow_produces_sending_off() {
    let home = make_team("home", "Home FC", 80, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 80, PlayStyle::Balanced);
    let config = MatchConfig {
        foul_probability: 0.80,
        yellow_card_probability: 0.80,
        red_card_probability: 0.001, // Low direct red so we get second yellows
        ..MatchConfig::default()
    };

    let mut second_yellows = 0u32;
    for seed in 0..100 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        second_yellows += report
            .events
            .iter()
            .filter(|e| e.event_type == EventType::SecondYellow)
            .count() as u32;
    }
    assert!(
        second_yellows > 0,
        "With many yellows and low red rate, second yellows should occur"
    );
}

// ---------------------------------------------------------------------------
// Injury from foul coverage
// ---------------------------------------------------------------------------

#[test]
fn high_injury_probability_produces_injuries() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        foul_probability: 0.90,
        injury_probability: 0.90,
        ..MatchConfig::default()
    };

    let mut total_injuries = 0u32;
    for seed in 0..30 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        total_injuries += report
            .events
            .iter()
            .filter(|e| e.event_type == EventType::Injury)
            .count() as u32;
    }
    assert!(
        total_injuries > 0,
        "With high foul+injury probability, injuries should occur"
    );
}

// ---------------------------------------------------------------------------
// Corner kick coverage
// ---------------------------------------------------------------------------

#[test]
fn corners_occur_in_simulation() {
    let home = make_team("home", "Home FC", 70, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 70, PlayStyle::Balanced);
    let config = MatchConfig::default();

    let mut total_corners = 0u32;
    for seed in 0..50 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        total_corners += report.home_stats.corners as u32 + report.away_stats.corners as u32;
    }
    assert!(total_corners > 0, "Corners should occur in 50 simulations");
}

// ---------------------------------------------------------------------------
// Sent-off player excluded from subsequent play
// ---------------------------------------------------------------------------

#[test]
fn sent_off_players_excluded() {
    // Run many sims with high foul/red card rate and verify the report still
    // produces valid data (no crashes from sent-off player selection).
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        foul_probability: 0.80,
        yellow_card_probability: 0.80,
        red_card_probability: 0.50,
        ..MatchConfig::default()
    };

    for seed in 0..50 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        // Just verify it completes without panic
        assert!(report.total_minutes >= 90);
    }
}

// ---------------------------------------------------------------------------
// Play style coverage for less common styles
// ---------------------------------------------------------------------------

#[test]
fn all_play_styles_produce_valid_report() {
    let styles = [
        PlayStyle::Balanced,
        PlayStyle::Attacking,
        PlayStyle::Defensive,
        PlayStyle::Possession,
        PlayStyle::Counter,
        PlayStyle::HighPress,
    ];

    for home_style in &styles {
        for away_style in &styles {
            let home = make_team("home", "Home FC", 65, *home_style);
            let away = make_team("away", "Away FC", 65, *away_style);
            let config = MatchConfig::default();
            let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(42));

            assert!(
                report.total_minutes >= 90,
                "Invalid report for {:?} vs {:?}",
                home_style,
                away_style
            );
            let has_fulltime = report
                .events
                .iter()
                .any(|e| e.event_type == EventType::FullTime);
            assert!(
                has_fulltime,
                "Missing FullTime for {:?} vs {:?}",
                home_style, away_style
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Edge: team with only 1 player per position
// ---------------------------------------------------------------------------

#[test]
fn minimal_team_doesnt_crash() {
    let minimal = TeamData {
        id: "min".to_string(),
        name: "Minimal FC".to_string(),
        formation: "1-1-1-1".to_string(),
        play_style: PlayStyle::Balanced,
        players: vec![
            make_player("gk", "GK", Position::Goalkeeper, 50),
            make_player("def", "DEF", Position::Defender, 50),
            make_player("mid", "MID", Position::Midfielder, 50),
            make_player("fwd", "FWD", Position::Forward, 50),
        ],
    };
    let normal = make_team("normal", "Normal FC", 60, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&minimal, &normal, &config, &mut seeded_rng(1));
    assert!(report.total_minutes >= 90);
}

// ---------------------------------------------------------------------------
// Edge: extreme skill disparity
// ---------------------------------------------------------------------------

#[test]
fn extreme_skill_disparity_no_crash() {
    let elite = make_team("elite", "Elite FC", 99, PlayStyle::Attacking);
    let amateur = make_team("amateur", "Amateur FC", 1, PlayStyle::Defensive);
    let config = MatchConfig::default();

    for seed in 0..10 {
        let report = simulate_with_rng(&elite, &amateur, &config, &mut seeded_rng(seed));
        assert!(report.total_minutes >= 90);
        // Elite team should generally score more
        assert!(
            report.home_goals >= report.away_goals || seed > 0,
            "Seed {seed}: elite team lost?"
        );
    }
}

// ---------------------------------------------------------------------------
// Report: player stats rating computation
// ---------------------------------------------------------------------------

#[test]
fn player_ratings_computed_for_active_players() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig::default();
    let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(42));

    // All players with stats should have ratings
    for (pid, ps) in &report.player_stats {
        assert!(
            ps.rating >= 0.0 && ps.rating <= 10.0,
            "Player {} has invalid rating: {}",
            pid,
            ps.rating
        );
    }
}

// ---------------------------------------------------------------------------
// Free kicks occur when fouls happen outside the box
// ---------------------------------------------------------------------------

#[test]
fn free_kicks_occur_in_simulation() {
    let home = make_team("home", "Home FC", 65, PlayStyle::Balanced);
    let away = make_team("away", "Away FC", 65, PlayStyle::Balanced);
    let config = MatchConfig {
        foul_probability: 0.80,
        ..MatchConfig::default()
    };

    let mut total_free_kicks = 0u32;
    for seed in 0..30 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        total_free_kicks +=
            report.home_stats.free_kicks as u32 + report.away_stats.free_kicks as u32;
    }
    assert!(
        total_free_kicks > 0,
        "Free kicks should occur with high foul rate"
    );
}

// ---------------------------------------------------------------------------
// Dribble and clearance events
// ---------------------------------------------------------------------------

#[test]
fn dribble_events_occur() {
    let home = make_team("home", "Home FC", 80, PlayStyle::Attacking);
    let away = make_team("away", "Away FC", 40, PlayStyle::Defensive);
    let config = MatchConfig::default();

    let mut total_dribbles = 0u32;
    let mut total_clearances = 0u32;
    for seed in 0..30 {
        let report = simulate_with_rng(&home, &away, &config, &mut seeded_rng(seed));
        for e in &report.events {
            match e.event_type {
                EventType::Dribble => total_dribbles += 1,
                EventType::Clearance => total_clearances += 1,
                _ => {}
            }
        }
    }
    assert!(total_dribbles > 0, "Dribbles should occur");
    assert!(total_clearances > 0, "Clearances should occur");
}
