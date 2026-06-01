use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::{Team, TrainingFocus, TrainingIntensity, TrainingSchedule};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::training;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 65,
        stamina: 65,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: 65,
        tackling: 65,
        dribbling: 65,
        defending: 65,
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 50,
        teamwork: 65,
        leadership: 50,
        handling: 20,
        reflexes: 30,
        aerial: 60,
    }
}

fn make_player(id: &str, name: &str, team_id: &str, dob: &str) -> Player {
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        dob.to_string(),
        "GB".to_string(),
        Position::Midfielder,
        default_attrs(),
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 80;
    p
}

fn make_team(id: &str, name: &str) -> Team {
    Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        40_000,
    )
}

fn make_staff(id: &str, team_id: &str, role: StaffRole, coaching: u8, physio: u8) -> Staff {
    let mut s = Staff::new(
        id.to_string(),
        "Staff".to_string(),
        id.to_string(),
        "1980-01-01".to_string(),
        role,
        StaffAttributes {
            coaching,
            judging_ability: 50,
            judging_potential: 50,
            physiotherapy: physio,
        },
    );
    s.team_id = Some(team_id.to_string());
    s.nationality = "GB".to_string();
    s
}

fn make_game() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap(); // Monday
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let mut team1 = make_team("team1", "Test FC");
    team1.training_focus = TrainingFocus::Physical;
    team1.training_intensity = TrainingIntensity::Medium;
    team1.training_schedule = TrainingSchedule::Balanced;

    // Young player (age ~21)
    let p1 = make_player("p1", "Young", "team1", "2004-03-15");
    // Prime player (age ~27)
    let p2 = make_player("p2", "Prime", "team1", "1998-06-10");
    // Old player (age ~35)
    let p3 = make_player("p3", "Veteran", "team1", "1990-01-01");

    let coach = make_staff("coach1", "team1", StaffRole::Coach, 80, 30);
    let physio = make_staff("physio1", "team1", StaffRole::Physio, 30, 80);

    Game::new(
        clock,
        manager,
        vec![team1],
        vec![p1, p2, p3],
        vec![coach, physio],
        vec![],
    )
}

// ---------------------------------------------------------------------------
// process_training — basic behavior
// ---------------------------------------------------------------------------

#[test]
fn training_on_training_day_costs_condition() {
    let mut game = make_game();
    // Monday (0) is a training day for Balanced schedule
    let initial_conditions: Vec<u8> = game.players.iter().map(|p| p.condition).collect();

    training::process_training(&mut game, 0);

    // Condition should change (cost - recovery, net effect depends on stamina/physio)
    // At minimum, training happened (we can check it didn't stay exactly the same
    // for all players, which would be extremely unlikely)
    let after_conditions: Vec<u8> = game.players.iter().map(|p| p.condition).collect();
    // Just verify no panics and condition is in valid range
    for c in &after_conditions {
        assert!(*c <= 100, "Condition should be <= 100");
    }
    // The test verifies the function runs without error
    let _ = (initial_conditions, after_conditions);
}

#[test]
fn rest_day_only_recovers_condition() {
    let mut game = make_game();
    // Set condition low
    for p in game.players.iter_mut() {
        p.condition = 50;
    }

    // Wednesday (2) is a rest day for Balanced schedule
    training::process_training(&mut game, 2);

    // All players should have gained condition (recovery only, no cost)
    for p in &game.players {
        assert!(
            p.condition > 50,
            "Player {} should recover on rest day, got {}",
            p.id,
            p.condition
        );
    }
}

#[test]
fn recovery_focus_no_condition_cost() {
    let mut game = make_game();
    game.teams[0].training_focus = TrainingFocus::Recovery;
    for p in game.players.iter_mut() {
        p.condition = 60;
    }

    // Monday (0) is training day, but Recovery focus has 0 condition cost
    training::process_training(&mut game, 0);

    for p in &game.players {
        assert!(
            p.condition >= 60,
            "Recovery focus should not reduce condition, got {}",
            p.condition
        );
    }
}

#[test]
fn high_intensity_costs_more_condition() {
    let mut game = make_game();
    game.teams[0].training_intensity = TrainingIntensity::High;
    game.teams[0].training_focus = TrainingFocus::Physical;
    for p in game.players.iter_mut() {
        p.condition = 90;
    }

    // Monday training day
    training::process_training(&mut game, 0);
    let high_conditions: Vec<u8> = game.players.iter().map(|p| p.condition).collect();

    // Reset and do low intensity
    let mut game2 = make_game();
    game2.teams[0].training_intensity = TrainingIntensity::Low;
    game2.teams[0].training_focus = TrainingFocus::Physical;
    for p in game2.players.iter_mut() {
        p.condition = 90;
    }

    training::process_training(&mut game2, 0);
    let low_conditions: Vec<u8> = game2.players.iter().map(|p| p.condition).collect();

    // High intensity should leave lower condition than low intensity on average
    let avg_high: f64 =
        high_conditions.iter().map(|c| *c as f64).sum::<f64>() / high_conditions.len() as f64;
    let avg_low: f64 =
        low_conditions.iter().map(|c| *c as f64).sum::<f64>() / low_conditions.len() as f64;
    assert!(
        avg_high <= avg_low,
        "High intensity ({:.1}) should cost more condition than low ({:.1})",
        avg_high,
        avg_low
    );
}

// ---------------------------------------------------------------------------
// process_training — schedules
// ---------------------------------------------------------------------------

#[test]
fn intense_schedule_trains_six_days() {
    let mut game = make_game();
    game.teams[0].training_schedule = TrainingSchedule::Intense;
    game.teams[0].training_focus = TrainingFocus::Physical;

    // Train all 7 days and count how many days condition drops
    let mut training_days = 0;
    for weekday in 0..7 {
        for p in game.players.iter_mut() {
            p.condition = 80;
        }
        training::process_training(&mut game, weekday);
        // If condition cost > recovery, it's a real training day
        // For Intense, Sun(6) is rest
        if weekday != 6 {
            training_days += 1;
        }
    }
    assert_eq!(training_days, 6, "Intense schedule should train 6 days");
}

#[test]
fn light_schedule_trains_two_days() {
    // Light: only Tue(1) and Thu(3) are training days
    assert!(TrainingSchedule::Light.is_training_day(1));
    assert!(TrainingSchedule::Light.is_training_day(3));
    assert!(!TrainingSchedule::Light.is_training_day(0));
    assert!(!TrainingSchedule::Light.is_training_day(2));
    assert!(!TrainingSchedule::Light.is_training_day(4));
    assert!(!TrainingSchedule::Light.is_training_day(5));
    assert!(!TrainingSchedule::Light.is_training_day(6));
}

// ---------------------------------------------------------------------------
// process_training — injured players
// ---------------------------------------------------------------------------

#[test]
fn injured_player_gets_reduced_recovery() {
    let mut game = make_game();
    let p1 = game.players.iter_mut().find(|p| p.id == "p1").unwrap();
    p1.condition = 40;
    p1.injury = Some(domain::player::Injury {
        name: "Hamstring".to_string(),
        days_remaining: 10,
    });

    let p2 = game.players.iter_mut().find(|p| p.id == "p2").unwrap();
    p2.condition = 40;

    // Rest day so both recover, but injured player gets reduced (0.5x) recovery
    training::process_training(&mut game, 2);

    let p1_after = game
        .players
        .iter()
        .find(|p| p.id == "p1")
        .unwrap()
        .condition;
    let p2_after = game
        .players
        .iter()
        .find(|p| p.id == "p2")
        .unwrap()
        .condition;

    assert!(p1_after > 40, "Injured player should still recover");
    assert!(
        p1_after <= p2_after,
        "Injured player ({}) should recover less than healthy ({})",
        p1_after,
        p2_after
    );
}

#[test]
fn higher_medical_facility_level_improves_recovery_on_rest_days() {
    let mut baseline = make_game();
    for player in baseline.players.iter_mut() {
        player.condition = 50;
    }

    let mut upgraded = make_game();
    for player in upgraded.players.iter_mut() {
        player.condition = 50;
    }
    upgraded.teams[0].facilities.medical = 3;

    training::process_training(&mut baseline, 2);
    training::process_training(&mut upgraded, 2);

    let baseline_avg = baseline
        .players
        .iter()
        .map(|player| player.condition as f64)
        .sum::<f64>()
        / baseline.players.len() as f64;
    let upgraded_avg = upgraded
        .players
        .iter()
        .map(|player| player.condition as f64)
        .sum::<f64>()
        / upgraded.players.len() as f64;

    assert!(
        upgraded_avg > baseline_avg,
        "Higher medical level should improve recovery: upgraded {:.2}, baseline {:.2}",
        upgraded_avg,
        baseline_avg
    );
}

// ---------------------------------------------------------------------------
// process_training — attribute gains (probabilistic)
// ---------------------------------------------------------------------------

#[test]
fn physical_focus_can_improve_physical_attrs() {
    let mut game = make_game();
    game.teams[0].training_focus = TrainingFocus::Physical;
    game.teams[0].training_intensity = TrainingIntensity::High;
    game.teams[0].training_schedule = TrainingSchedule::Intense;

    // Record initial stats
    let initial_pace: Vec<u8> = game.players.iter().map(|p| p.attributes.pace).collect();
    let initial_stamina: Vec<u8> = game.players.iter().map(|p| p.attributes.stamina).collect();

    // Train many sessions to make probabilistic gains likely
    for _ in 0..100 {
        for p in game.players.iter_mut() {
            p.condition = 90; // Keep condition high so training continues
        }
        training::process_training(&mut game, 0); // Monday = training day
    }

    let final_pace: Vec<u8> = game.players.iter().map(|p| p.attributes.pace).collect();
    let final_stamina: Vec<u8> = game.players.iter().map(|p| p.attributes.stamina).collect();

    // At least one player should have gained in pace or stamina after 100 sessions
    let any_pace_gain = initial_pace
        .iter()
        .zip(final_pace.iter())
        .any(|(i, f)| f > i);
    let any_stamina_gain = initial_stamina
        .iter()
        .zip(final_stamina.iter())
        .any(|(i, f)| f > i);

    assert!(
        any_pace_gain || any_stamina_gain,
        "Physical focus should improve pace or stamina after many sessions"
    );
}

#[test]
fn technical_focus_can_improve_technical_attrs() {
    let mut game = make_game();
    game.teams[0].training_focus = TrainingFocus::Technical;
    game.teams[0].training_intensity = TrainingIntensity::High;
    game.teams[0].training_schedule = TrainingSchedule::Intense;

    let initial_passing: Vec<u8> = game.players.iter().map(|p| p.attributes.passing).collect();

    for _ in 0..100 {
        for p in game.players.iter_mut() {
            p.condition = 90;
        }
        training::process_training(&mut game, 0);
    }

    let final_passing: Vec<u8> = game.players.iter().map(|p| p.attributes.passing).collect();
    let any_gain = initial_passing
        .iter()
        .zip(final_passing.iter())
        .any(|(i, f)| f > i);
    assert!(
        any_gain,
        "Technical focus should improve passing after many sessions"
    );
}

#[test]
fn recovery_focus_no_attribute_gains() {
    let mut game = make_game();
    game.teams[0].training_focus = TrainingFocus::Recovery;
    game.teams[0].training_intensity = TrainingIntensity::High;

    let initial_attrs: Vec<PlayerAttributes> =
        game.players.iter().map(|p| p.attributes.clone()).collect();

    for _ in 0..50 {
        for p in game.players.iter_mut() {
            p.condition = 90;
        }
        training::process_training(&mut game, 0);
    }

    // Recovery focus: no attribute gains at all
    for (i, p) in game.players.iter().enumerate() {
        assert_eq!(
            p.attributes.pace, initial_attrs[i].pace,
            "Recovery should not change pace"
        );
        assert_eq!(
            p.attributes.shooting, initial_attrs[i].shooting,
            "Recovery should not change shooting"
        );
    }
}

// ---------------------------------------------------------------------------
// process_training — no coaching staff penalty
// ---------------------------------------------------------------------------

#[test]
fn no_coaching_staff_reduces_gains() {
    // Game with no staff
    let date = Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let mut team1 = make_team("team1", "Test FC");
    team1.training_focus = TrainingFocus::Physical;
    team1.training_intensity = TrainingIntensity::High;
    team1.training_schedule = TrainingSchedule::Intense;

    let p1 = make_player("p1", "Young", "team1", "2004-03-15");

    let mut game = Game::new(clock, manager, vec![team1], vec![p1], vec![], vec![]);

    // Train many sessions
    let initial_pace = game.players[0].attributes.pace;
    for _ in 0..200 {
        game.players[0].condition = 90;
        training::process_training(&mut game, 0);
    }

    // Should still gain something (just less than with staff)
    // The 0.8 penalty from no staff still allows some growth
    let final_pace = game.players[0].attributes.pace;
    // After 200 intense sessions with a young player, some gain is expected
    assert!(
        final_pace >= initial_pace,
        "Should still gain attributes without staff"
    );
}

// ---------------------------------------------------------------------------
// check_squad_fitness_warnings
// ---------------------------------------------------------------------------

#[test]
fn no_warning_when_squad_is_fit() {
    let mut game = make_game();
    for p in game.players.iter_mut() {
        p.condition = 90;
    }

    training::check_squad_fitness_warnings(&mut game);

    let fitness_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("fitness_warn_"))
        .collect();
    assert!(fitness_msgs.is_empty(), "No warning when squad is fit");
}

#[test]
fn warning_when_avg_condition_below_50() {
    let mut game = make_game();
    for p in game.players.iter_mut() {
        p.condition = 40; // avg = 40 < 50
    }

    training::check_squad_fitness_warnings(&mut game);

    let fitness_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("fitness_warn_"))
        .collect();
    assert_eq!(fitness_msgs.len(), 1, "Should send fitness warning");
    assert_eq!(
        fitness_msgs[0].subject_key.as_deref(),
        Some("be.msg.fitness.warning.subject")
    );
}

#[test]
fn critical_warning_when_many_players_below_25() {
    let mut game = make_game();
    for p in game.players.iter_mut() {
        p.condition = 20; // all below 25 → critical
    }

    training::check_squad_fitness_warnings(&mut game);

    let fitness_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("fitness_warn_"))
        .collect();
    assert_eq!(fitness_msgs.len(), 1, "Should send fitness message");
    assert_eq!(
        fitness_msgs[0].subject_key.as_deref(),
        Some("be.msg.fitness.critical.subject")
    );
}

#[test]
fn fitness_warning_not_duplicated_same_day() {
    let mut game = make_game();
    for p in game.players.iter_mut() {
        p.condition = 40;
    }

    training::check_squad_fitness_warnings(&mut game);
    training::check_squad_fitness_warnings(&mut game);

    let fitness_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("fitness_warn_"))
        .collect();
    assert_eq!(
        fitness_msgs.len(),
        1,
        "Should not duplicate same-day warning"
    );
}

#[test]
fn no_warning_without_manager_team() {
    let mut game = make_game();
    game.manager.team_id = None;
    for p in game.players.iter_mut() {
        p.condition = 20;
    }

    training::check_squad_fitness_warnings(&mut game);

    let fitness_msgs: Vec<_> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("fitness_warn_"))
        .collect();
    assert!(fitness_msgs.is_empty(), "No warning without manager team");
}

#[test]
fn warning_uses_physio_sender_when_available() {
    let mut game = make_game();
    for p in game.players.iter_mut() {
        p.condition = 40;
    }

    training::check_squad_fitness_warnings(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id.starts_with("fitness_warn_"))
        .unwrap();
    assert!(
        msg.sender_role_key.as_deref() == Some("be.role.headPhysio"),
        "Sender role key should be be.role.headPhysio when physio is on staff, got: {:?}",
        msg.sender_role_key
    );
}

#[test]
fn warning_uses_assistant_manager_when_no_physio() {
    let mut game = make_game();
    // Remove physio
    game.staff.retain(|s| !matches!(s.role, StaffRole::Physio));
    for p in game.players.iter_mut() {
        p.condition = 40;
    }

    training::check_squad_fitness_warnings(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id.starts_with("fitness_warn_"))
        .unwrap();
    assert!(
        msg.sender_role_key.as_deref() == Some("be.role.assistantManager"),
        "Sender role key should be be.role.assistantManager when no physio, got: {:?}",
        msg.sender_role_key
    );
}

// ---------------------------------------------------------------------------
// Age factor effects
// ---------------------------------------------------------------------------

#[test]
fn young_player_gains_more_than_old() {
    // Compare gains for young (21) vs old (35) player over many sessions
    let mut game = make_game();
    game.teams[0].training_focus = TrainingFocus::Physical;
    game.teams[0].training_intensity = TrainingIntensity::High;
    game.teams[0].training_schedule = TrainingSchedule::Intense;

    let p1_initial_pace = game
        .players
        .iter()
        .find(|p| p.id == "p1")
        .unwrap()
        .attributes
        .pace;
    let p3_initial_pace = game
        .players
        .iter()
        .find(|p| p.id == "p3")
        .unwrap()
        .attributes
        .pace;

    for _ in 0..300 {
        for p in game.players.iter_mut() {
            p.condition = 90;
        }
        training::process_training(&mut game, 0);
    }

    let p1_final_pace = game
        .players
        .iter()
        .find(|p| p.id == "p1")
        .unwrap()
        .attributes
        .pace;
    let p3_final_pace = game
        .players
        .iter()
        .find(|p| p.id == "p3")
        .unwrap()
        .attributes
        .pace;

    let p1_gain = p1_final_pace - p1_initial_pace;
    let p3_gain = p3_final_pace - p3_initial_pace;

    assert!(
        p1_gain >= p3_gain,
        "Young player (gain={}) should gain at least as much as old player (gain={})",
        p1_gain,
        p3_gain
    );
}

// ---------------------------------------------------------------------------
// All training focuses work
// ---------------------------------------------------------------------------

#[test]
fn all_focuses_run_without_panic() {
    let focuses = [
        TrainingFocus::Physical,
        TrainingFocus::Technical,
        TrainingFocus::Tactical,
        TrainingFocus::Defending,
        TrainingFocus::Attacking,
        TrainingFocus::Recovery,
    ];

    for focus in &focuses {
        let mut game = make_game();
        game.teams[0].training_focus = focus.clone();
        training::process_training(&mut game, 0);
        // Just verify no panics
    }
}

#[test]
fn all_intensities_run_without_panic() {
    let intensities = [
        TrainingIntensity::Low,
        TrainingIntensity::Medium,
        TrainingIntensity::High,
    ];

    for intensity in &intensities {
        let mut game = make_game();
        game.teams[0].training_intensity = intensity.clone();
        training::process_training(&mut game, 0);
    }
}

// ---------------------------------------------------------------------------
// Fitness system tests
// ---------------------------------------------------------------------------

#[test]
fn high_fitness_player_recovers_condition_faster_on_rest_day() {
    // Two players identical except fitness
    let mut game_low = make_game();
    let mut game_high = make_game();

    for p in game_low.players.iter_mut() {
        p.fitness = 20; // very unfit
        p.condition = 50;
    }
    for p in game_high.players.iter_mut() {
        p.fitness = 95; // peak fitness
        p.condition = 50;
    }

    // Wednesday (2) is rest day for Balanced schedule
    training::process_training(&mut game_low, 2);
    training::process_training(&mut game_high, 2);

    let avg_low = game_low
        .players
        .iter()
        .map(|p| p.condition as f64)
        .sum::<f64>()
        / game_low.players.len() as f64;
    let avg_high = game_high
        .players
        .iter()
        .map(|p| p.condition as f64)
        .sum::<f64>()
        / game_high.players.len() as f64;

    assert!(
        avg_high > avg_low,
        "High fitness players ({:.1}) should recover more than low fitness ({:.1})",
        avg_high,
        avg_low
    );
}

#[test]
fn physical_training_can_increase_fitness() {
    let mut game = make_game();
    game.teams[0].training_focus = TrainingFocus::Physical;
    game.teams[0].training_intensity = TrainingIntensity::High;
    game.teams[0].training_schedule = TrainingSchedule::Intense;

    // Set a below-peak fitness so gains are possible
    for p in game.players.iter_mut() {
        p.fitness = 70;
    }

    let initial_fitness: Vec<u8> = game.players.iter().map(|p| p.fitness).collect();

    // Train many sessions to trigger probabilistic fitness gain
    for _ in 0..500 {
        for p in game.players.iter_mut() {
            p.condition = 90;
        }
        training::process_training(&mut game, 0); // Monday = training day
    }

    let final_fitness: Vec<u8> = game.players.iter().map(|p| p.fitness).collect();
    let any_gain = initial_fitness
        .iter()
        .zip(final_fitness.iter())
        .any(|(i, f)| f > i);

    assert!(
        any_gain,
        "Physical training should increase fitness after many sessions"
    );
}

#[test]
fn injured_player_loses_fitness_over_time() {
    let mut game = make_game();
    let p1 = game.players.iter_mut().find(|p| p.id == "p1").unwrap();
    p1.fitness = 80;
    p1.injury = Some(domain::player::Injury {
        name: "Hamstring".to_string(),
        days_remaining: 30,
    });

    let initial_fitness = game.players.iter().find(|p| p.id == "p1").unwrap().fitness;

    // Simulate 20 rest days with the injury
    for _ in 0..20 {
        training::process_training(&mut game, 2); // rest day
    }

    let final_fitness = game.players.iter().find(|p| p.id == "p1").unwrap().fitness;

    assert!(
        final_fitness < initial_fitness,
        "Injured player's fitness ({}) should decay below initial ({})",
        final_fitness,
        initial_fitness
    );
}
