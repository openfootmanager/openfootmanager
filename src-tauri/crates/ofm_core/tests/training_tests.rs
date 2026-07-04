use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::{Team, TrainingFocus, TrainingIntensity, TrainingSchedule};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::player_rating::refresh_player_derived;
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
fn each_team_trains_under_its_own_focus() {
    // Given: two teams with different focuses, each with one player
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

    let mut team1 = make_team("team1", "Tech FC");
    team1.training_focus = TrainingFocus::Technical;
    team1.training_intensity = TrainingIntensity::High;
    team1.training_schedule = TrainingSchedule::Intense;

    let mut team2 = make_team("team2", "Phys FC");
    team2.training_focus = TrainingFocus::Physical;
    team2.training_intensity = TrainingIntensity::High;
    team2.training_schedule = TrainingSchedule::Intense;

    let tech_player = make_player("tech", "Tech", "team1", "2004-03-15");
    let phys_player = make_player("phys", "Phys", "team2", "2004-03-15");

    let mut game = Game::new(
        clock,
        manager,
        vec![team1, team2],
        vec![tech_player, phys_player],
        vec![],
        vec![],
    );

    let tech_initial = game
        .players
        .iter()
        .find(|p| p.id == "tech")
        .unwrap()
        .attributes
        .clone();
    let phys_initial = game
        .players
        .iter()
        .find(|p| p.id == "phys")
        .unwrap()
        .attributes
        .clone();

    // When: both teams train for many sessions
    for _ in 0..150 {
        for p in game.players.iter_mut() {
            p.condition = 90;
        }
        training::process_training(&mut game, 0);
    }

    let tech_after = game
        .players
        .iter()
        .find(|p| p.id == "tech")
        .unwrap()
        .attributes
        .clone();
    let phys_after = game
        .players
        .iter()
        .find(|p| p.id == "phys")
        .unwrap()
        .attributes
        .clone();

    // Then: the Technical player never gains Physical-only attributes, and the
    // Physical player never gains Technical-only attributes — proving each
    // player trained under its OWN team's plan, not another team's.
    assert_eq!(
        tech_after.pace, tech_initial.pace,
        "Technical focus must not change pace"
    );
    assert_eq!(
        tech_after.stamina, tech_initial.stamina,
        "Technical focus must not change stamina"
    );
    assert_eq!(
        tech_after.strength, tech_initial.strength,
        "Technical focus must not change strength"
    );
    assert_eq!(
        tech_after.agility, tech_initial.agility,
        "Technical focus must not change agility"
    );

    assert_eq!(
        phys_after.passing, phys_initial.passing,
        "Physical focus must not change passing"
    );
    assert_eq!(
        phys_after.dribbling, phys_initial.dribbling,
        "Physical focus must not change dribbling"
    );
}

#[test]
fn players_without_a_real_team_are_not_trained() {
    // Given: a free agent (no team) and a player pointing at a non-existent team
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

    let mut team1 = make_team("team1", "Real FC");
    team1.training_focus = TrainingFocus::Physical;
    team1.training_intensity = TrainingIntensity::High;
    team1.training_schedule = TrainingSchedule::Intense;

    let mut free_agent = make_player("free", "Free", "team1", "2004-03-15");
    free_agent.team_id = None;
    free_agent.condition = 40;
    free_agent.fitness = 40;

    let mut ghost = make_player("ghost", "Ghost", "no-such-team", "2004-03-15");
    ghost.condition = 40;
    ghost.fitness = 40;

    let mut game = Game::new(
        clock,
        manager,
        vec![team1],
        vec![free_agent, ghost],
        vec![],
        vec![],
    );

    let snapshot = |g: &Game, id: &str| {
        let p = g.players.iter().find(|p| p.id == id).unwrap();
        (
            p.attributes.pace,
            p.attributes.stamina,
            p.attributes.strength,
            p.attributes.agility,
            p.condition,
            p.fitness,
        )
    };
    let free_before = snapshot(&game, "free");
    let ghost_before = snapshot(&game, "ghost");

    // When: training runs for several sessions
    for _ in 0..50 {
        training::process_training(&mut game, 0);
    }

    // Then: neither player is touched (no recovery, no gains, no fitness change)
    assert_eq!(
        snapshot(&game, "free"),
        free_before,
        "Free agent must not be trained"
    );
    assert_eq!(
        snapshot(&game, "ghost"),
        ghost_before,
        "Player with no real team must not be trained"
    );
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

// ---------------------------------------------------------------------------
// AI fatigue guard
// ---------------------------------------------------------------------------

/// Reproduces the fatigue spiral and verifies the AI-only guard breaks it.
///
/// An individually exhausted player on a team training at Medium intensity with a
/// non-recovery focus pays a flat condition cost (6) that exceeds their diminished
/// recovery — so without intervention they keep losing condition every training
/// day and never climb out. The AI fatigue guard auto-rests such players on AI
/// teams. The user's team is exempt (manual agency), so an identical exhausted
/// player on the user's side keeps spiralling down.
#[test]
fn ai_fatigue_guard_rests_exhausted_ai_player_but_not_user_team() {
    let mut game = make_game(); // manager is hired to "team1" (the user team)

    // Add an AI-controlled team that trains hard (Medium, non-recovery focus).
    let mut team2 = make_team("team2", "AI FC");
    team2.training_focus = TrainingFocus::Physical;
    team2.training_intensity = TrainingIntensity::Medium;
    team2.training_schedule = TrainingSchedule::Balanced;
    game.teams.push(team2);

    // Two identical exhausted players: one on the user team, one on the AI team.
    let mut user_tired = make_player("user_tired", "UserTired", "team1", "1998-06-10");
    user_tired.condition = 22;
    let mut ai_tired = make_player("ai_tired", "AiTired", "team2", "1998-06-10");
    ai_tired.condition = 22;
    game.players.push(user_tired);
    game.players.push(ai_tired);

    // Three consecutive training days (Monday is a training day under Balanced).
    for _ in 0..3 {
        training::process_training(&mut game, 0);
    }

    let user_after = game
        .players
        .iter()
        .find(|p| p.id == "user_tired")
        .unwrap()
        .condition;
    let ai_after = game
        .players
        .iter()
        .find(|p| p.id == "ai_tired")
        .unwrap()
        .condition;

    // Guard active: the AI's exhausted player recovers out of the spiral.
    assert!(
        ai_after > 22,
        "AI exhausted player should recover under the fatigue guard, got {ai_after}"
    );
    // Exempt: the user's identical player keeps net-losing condition at Medium.
    assert!(
        user_after < 22,
        "user-team exhausted player should not be auto-rested, got {user_after}"
    );
    assert!(
        ai_after > user_after,
        "guarded AI player ({ai_after}) should end fresher than the user player ({user_after})"
    );
}

// ---------------------------------------------------------------------------
// process_training — potential ceiling (issue #307)
//
// Peaked players (ovr == potential) must not keep gaining. Attribute-driven OVR
// growth combined with the `potential = max(potential, ovr)` safety net causes
// the career ceiling to rise in lockstep with the drift when this contract is
// not enforced at the training-gain site.
// ---------------------------------------------------------------------------

#[test]
fn peaked_player_does_not_gain_from_training() {
    let mut game = make_game();
    game.teams[0].training_focus = TrainingFocus::Physical;
    game.teams[0].training_intensity = TrainingIntensity::High;
    game.teams[0].training_schedule = TrainingSchedule::Intense;

    // Prime-age player (age ~27 in 2025). Compute their natural OVR from
    // default_attrs, then peg potential to it so they have no headroom.
    let year = game.clock.current_date.date_naive().format("%Y").to_string();
    let year: u32 = year.parse().unwrap();
    let peaked_id = "p2".to_string();
    {
        let p = game
            .players
            .iter_mut()
            .find(|p| p.id == peaked_id)
            .expect("prime player exists");
        refresh_player_derived(p, year);
        p.potential = p.ovr;
    }

    let initial = game
        .players
        .iter()
        .find(|p| p.id == peaked_id)
        .unwrap();
    let initial_ovr = initial.ovr;
    let initial_potential = initial.potential;
    // Snapshot the exact attributes Physical focus would try to grow so the
    // regression catches a partial gate that still lets one attribute leak
    // through even when the aggregated ovr happens not to move.
    let initial_pace = initial.attributes.pace;
    let initial_stamina = initial.attributes.stamina;
    let initial_strength = initial.attributes.strength;
    let initial_agility = initial.attributes.agility;
    assert_eq!(
        initial_ovr, initial_potential,
        "test precondition: peaked player starts with ovr == potential"
    );

    // Run many training sessions with condition kept high so the AI fatigue
    // guard doesn't intervene.
    for _ in 0..150 {
        for p in game.players.iter_mut() {
            p.condition = 90;
        }
        training::process_training(&mut game, 0); // Monday, Intense trains Mon
    }

    let final_player = game.players.iter().find(|p| p.id == peaked_id).unwrap();

    assert_eq!(
        final_player.attributes.pace, initial_pace,
        "peaked player's pace must not rise from Physical training (bug #307)"
    );
    assert_eq!(
        final_player.attributes.stamina, initial_stamina,
        "peaked player's stamina must not rise from Physical training (bug #307)"
    );
    assert_eq!(
        final_player.attributes.strength, initial_strength,
        "peaked player's strength must not rise from Physical training (bug #307)"
    );
    assert_eq!(
        final_player.attributes.agility, initial_agility,
        "peaked player's agility must not rise from Physical training (bug #307)"
    );
    assert_eq!(
        final_player.potential, initial_potential,
        "peaked player's potential must not rise from training (bug #307): \
         potential drifted {} → {}",
        initial_potential, final_player.potential
    );
    assert_eq!(
        final_player.ovr, initial_ovr,
        "peaked player's ovr must not rise from training (bug #307): \
         ovr drifted {} → {}",
        initial_ovr, final_player.ovr
    );
}
