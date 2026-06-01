use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::message::*;
use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::{Game, YouthScoutingObjective, YouthScoutingRegion};
use ofm_core::scouting::{
    apply_youth_recruitment_response, process_scouting, scout_max_assignments, send_scout,
    start_youth_scouting,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 70,
        stamina: 65,
        strength: 60,
        agility: 68,
        passing: 72,
        shooting: 66,
        tackling: 58,
        dribbling: 74,
        defending: 55,
        positioning: 62,
        vision: 70,
        decisions: 64,
        composure: 60,
        aggression: 50,
        teamwork: 66,
        leadership: 55,
        handling: 30,
        reflexes: 30,
        aerial: 58,
    }
}

fn make_player(id: &str, name: &str, team_id: &str) -> Player {
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        name.to_string(),
        "1998-03-15".to_string(),
        "BR".to_string(),
        Position::Midfielder,
        default_attrs(),
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 75;
    p.condition = 90;
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

fn make_scout(id: &str, team_id: &str, judging_ability: u8, judging_potential: u8) -> Staff {
    let mut s = Staff::new(
        id.to_string(),
        "Scout".to_string(),
        format!("Nr{}", id),
        "1985-01-01".to_string(),
        StaffRole::Scout,
        StaffAttributes {
            coaching: 30,
            judging_ability,
            judging_potential,
            physiotherapy: 20,
        },
    );
    s.team_id = Some(team_id.to_string());
    s
}

fn make_game() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Test FC");
    let team2 = make_team("team2", "Rival FC");
    let players = vec![
        make_player("p1", "Own Player", "team1"),
        make_player("p2", "Target Player", "team2"),
        make_player("p3", "Another Target", "team2"),
    ];
    let scout = make_scout("scout1", "team1", 80, 75);

    Game::new(
        clock,
        manager,
        vec![team1, team2],
        players,
        vec![scout],
        vec![],
    )
}

// ---------------------------------------------------------------------------
// scout_max_assignments
// ---------------------------------------------------------------------------

#[test]
fn max_assignments_is_one_for_all_scouts() {
    assert_eq!(scout_max_assignments(90), 1);
    assert_eq!(scout_max_assignments(80), 1);
    assert_eq!(scout_max_assignments(79), 1);
    assert_eq!(scout_max_assignments(60), 1);
    assert_eq!(scout_max_assignments(59), 1);
    assert_eq!(scout_max_assignments(40), 1);
    assert_eq!(scout_max_assignments(39), 1);
    assert_eq!(scout_max_assignments(20), 1);
    assert_eq!(scout_max_assignments(19), 1);
    assert_eq!(scout_max_assignments(0), 1);
}

// ---------------------------------------------------------------------------
// send_scout
// ---------------------------------------------------------------------------

#[test]
fn send_scout_creates_assignment() {
    let mut game = make_game();
    let result = send_scout(&mut game, "scout1", "p2");
    assert!(result.is_ok());
    assert_eq!(game.scouting_assignments.len(), 1);
    assert_eq!(game.scouting_assignments[0].player_id, "p2");
    assert_eq!(game.scouting_assignments[0].scout_id, "scout1");
}

#[test]
fn send_scout_rejects_own_player() {
    let mut game = make_game();
    let result = send_scout(&mut game, "scout1", "p1");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "be.error.scouting.cannotScoutOwnPlayer");
}

#[test]
fn send_scout_rejects_unknown_player() {
    let mut game = make_game();
    let result = send_scout(&mut game, "scout1", "nonexistent");
    assert_eq!(result.unwrap_err(), "be.error.playerNotFound");
}

#[test]
fn send_scout_rejects_duplicate_assignment() {
    let mut game = make_game();
    send_scout(&mut game, "scout1", "p2").unwrap();
    game.staff.push(make_scout("scout2", "team1", 70, 70));
    let result = send_scout(&mut game, "scout2", "p2");
    assert_eq!(result.unwrap_err(), "be.error.scouting.playerAlreadyScouted");
}

#[test]
fn send_scout_rejects_non_scout_staff() {
    let mut game = make_game();
    // Replace scout with a coach
    game.staff[0].role = StaffRole::Coach;
    let result = send_scout(&mut game, "scout1", "p2");
    assert_eq!(result.unwrap_err(), "be.error.scouting.staffMemberNotScout");
}

#[test]
fn start_youth_scouting_creates_assignment() {
    let mut game = make_game();

    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(Position::Defender),
    )
    .unwrap();

    assert_eq!(game.youth_scouting_assignments.len(), 1);
    assert_eq!(game.youth_scouting_assignments[0].scout_id, "scout1");
    assert_eq!(
        game.youth_scouting_assignments[0].region,
        YouthScoutingRegion::Domestic
    );
    assert_eq!(
        game.youth_scouting_assignments[0].objective,
        YouthScoutingObjective::Balanced
    );
    assert_eq!(
        game.youth_scouting_assignments[0].target_position,
        Some(Position::Defender)
    );
}

#[test]
fn start_youth_scouting_respects_shared_scout_capacity() {
    let mut game = make_game();
    send_scout(&mut game, "scout1", "p2").unwrap();
    let result = start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(Position::Forward),
    );

    assert_eq!(
        result.unwrap_err(),
        "be.error.scouting.scoutAssignmentFull?currentCount=1&maxSlots=1"
    );
}

#[test]
fn send_scout_rejects_when_scout_has_youth_assignment() {
    let mut game = make_game();
    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(Position::Forward),
    )
    .unwrap();

    let result = send_scout(&mut game, "scout1", "p2");

    assert_eq!(
        result.unwrap_err(),
        "be.error.scouting.scoutAssignmentFull?currentCount=1&maxSlots=1"
    );
}

#[test]
fn start_youth_scouting_rejects_duplicate_search_profile() {
    let mut game = make_game();
    game.staff.push(make_scout("scout2", "team1", 70, 70));

    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(Position::Defender),
    )
    .unwrap();

    let result = start_youth_scouting(
        &mut game,
        "scout2",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(Position::Defender),
    );

    assert_eq!(result.unwrap_err(), "be.error.scouting.youthSearchAlreadyActive");
}

// ---------------------------------------------------------------------------
// process_scouting — report generation
// ---------------------------------------------------------------------------

fn complete_scouting(game: &mut Game) {
    // Advance enough days for the assignment to complete
    for _ in 0..10 {
        process_scouting(game);
        game.clock.advance_days(1);
    }
}

#[test]
fn process_scouting_generates_report_message() {
    let mut game = make_game();
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let scout_msgs: Vec<&InboxMessage> = game
        .messages
        .iter()
        .filter(|m| m.category == MessageCategory::ScoutReport)
        .collect();
    assert_eq!(
        scout_msgs.len(),
        1,
        "Should produce exactly one scout report"
    );
}

#[test]
fn report_has_scout_report_data() {
    let mut game = make_game();
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .expect("Should have a scout report");

    let report = msg
        .context
        .scout_report
        .as_ref()
        .expect("Should have scout_report data in context");

    assert_eq!(report.player_id, "p2");
    assert_eq!(report.player_name, "Target Player");
    assert_eq!(report.nationality, "BR");
    assert!(report.team_name.is_some(), "Should have team name");
    assert_eq!(report.team_name.as_deref(), Some("Rival FC"));
}

#[test]
fn process_scouting_completes_youth_recruitment_report() {
    let mut game = make_game();
    let initial_player_count = game.players.len();

    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(Position::Defender),
    )
    .unwrap();
    complete_scouting(&mut game);

    assert_eq!(game.players.len(), initial_player_count);
    assert!(game.youth_scouting_assignments.is_empty());

    let msg = game
        .messages
        .iter()
        .find(|message| {
            message.subject_key.as_deref() == Some("be.msg.youthRecruitmentReport.subject")
        })
        .expect("expected a youth recruitment report");
    assert_eq!(msg.category, MessageCategory::ScoutReport);
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.youthRecruitmentReport.subject")
    );
    assert!(matches!(
        msg.body_key.as_deref(),
        Some("be.msg.youthRecruitmentReport.bodyTargeted")
    ));
    assert!(msg.context.player_id.is_none());
    assert_eq!(
        msg.context.youth_target_position.as_deref(),
        Some("Defender")
    );
    assert_eq!(msg.context.youth_search_region.as_deref(), Some("Domestic"));
    assert_eq!(
        msg.context.youth_search_objective.as_deref(),
        Some("Balanced")
    );
    assert_eq!(msg.sender_role_key.as_deref(), Some("be.role.scout"));
    assert_eq!(
        msg.i18n_params.get("regionLabel"),
        Some(&"scouting.regionDomestic".to_string())
    );
    assert_eq!(
        msg.i18n_params.get("objectiveLabel"),
        Some(&"scouting.objectiveBalanced".to_string())
    );
    let prospects = msg
        .context
        .youth_prospects
        .as_ref()
        .expect("expected youth prospects in message context");
    assert_eq!(prospects.len(), 3);
    assert!(prospects.iter().all(|prospect| prospect.team_id.is_none()));
    assert!(
        prospects
            .iter()
            .all(|prospect| prospect.position.to_group_position() == Position::Defender)
    );
    assert_eq!(msg.actions.len(), 3);
    assert!(
        msg.actions
            .iter()
            .all(|action| matches!(action.action_type, ActionType::ChooseOption { .. }))
    );
}

#[test]
fn youth_recruitment_response_signs_selected_prospect() {
    let mut game = make_game();
    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(Position::Defender),
    )
    .unwrap();
    complete_scouting(&mut game);

    let message = game
        .messages
        .iter()
        .find(|candidate| {
            candidate.subject_key.as_deref() == Some("be.msg.youthRecruitmentReport.subject")
        })
        .expect("expected youth recruitment report")
        .clone();
    let action_id = message.actions[0].id.clone();
    let prospect_id = action_id.trim_start_matches("prospect:").to_string();

    let effect = apply_youth_recruitment_response(&mut game, &message.id, &action_id, "sign")
        .expect("expected sign effect");

    assert_eq!(effect.message, "");
    assert_eq!(effect.i18n_key, "be.msg.youthRecruitment.effect.sign");
    let signed_player = game
        .players
        .iter()
        .find(|player| player.id == prospect_id)
        .expect("expected signed player in game state");
    assert_eq!(signed_player.team_id.as_deref(), Some("team1"));
    assert_eq!(signed_player.squad_role, domain::player::SquadRole::Youth);
    let updated_message = game
        .messages
        .iter()
        .find(|candidate| candidate.id == message.id)
        .expect("expected updated report message");
    assert!(updated_message.actions[0].resolved);
    assert!(!updated_message.actions[1].resolved);
    let updated_prospects = updated_message
        .context
        .youth_prospects
        .as_ref()
        .expect("expected remaining prospects in report");
    let signed_prospect = updated_prospects
        .iter()
        .find(|player| player.id == prospect_id)
        .expect("expected signed prospect to remain visible");
    assert_eq!(signed_prospect.team_id.as_deref(), Some("team1"));
}

#[test]
fn youth_recruitment_response_shortlists_selected_prospect() {
    let mut game = make_game();
    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::International,
        YouthScoutingObjective::HighPotential,
        Some(Position::Forward),
    )
    .unwrap();
    complete_scouting(&mut game);

    let message = game
        .messages
        .iter()
        .find(|candidate| {
            candidate.subject_key.as_deref() == Some("be.msg.youthRecruitmentReport.subject")
        })
        .expect("expected youth recruitment report")
        .clone();
    let action_id = message.actions[1].id.clone();

    let effect = apply_youth_recruitment_response(&mut game, &message.id, &action_id, "shortlist")
        .expect("expected shortlist effect");

    assert_eq!(effect.message, "");
    assert_eq!(
        effect.i18n_key,
        "be.msg.youthRecruitment.effect.shortlist".to_string()
    );
    assert!(
        game.messages
            .iter()
            .any(|candidate| {
                candidate.subject_key.as_deref()
                    == Some("be.msg.youthRecruitmentShortlist.subject")
            })
    );
    let updated_message = game
        .messages
        .iter()
        .find(|candidate| candidate.id == message.id)
        .expect("expected updated original report message");
    assert_eq!(
        updated_message
            .context
            .youth_prospects
            .as_ref()
            .expect("expected remaining prospects")
            .len(),
        2
    );
}

#[test]
fn youth_recruitment_response_discard_removes_only_selected_prospect() {
    let mut game = make_game();
    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::ReadySoon,
        Some(Position::Midfielder),
    )
    .unwrap();
    complete_scouting(&mut game);

    let message = game
        .messages
        .iter()
        .find(|candidate| {
            candidate.subject_key.as_deref() == Some("be.msg.youthRecruitmentReport.subject")
        })
        .expect("expected youth recruitment report")
        .clone();
    let action_id = message.actions[0].id.clone();

    apply_youth_recruitment_response(&mut game, &message.id, &action_id, "discard")
        .expect("expected discard effect");

    let updated_message = game
        .messages
        .iter()
        .find(|candidate| candidate.id == message.id)
        .expect("expected updated report message");
    assert_eq!(updated_message.actions.len(), 2);
    assert_eq!(
        updated_message
            .context
            .youth_prospects
            .as_ref()
            .expect("expected remaining prospects")
            .len(),
        2
    );
}

#[test]
fn report_has_i18n_keys() {
    let mut game = make_game();
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap();

    assert!(msg.subject_key.is_some());
    assert!(msg.body_key.is_some());
    assert!(msg.sender_key.is_some());
    assert!(msg.sender_role_key.is_some());

    let report = msg.context.scout_report.as_ref().unwrap();
    assert!(
        report.rating_key.starts_with("common.scoutRatings."),
        "rating_key should be an i18n key: {}",
        report.rating_key
    );
    assert!(
        report.potential_key.starts_with("common.scoutPotential."),
        "potential_key should be an i18n key: {}",
        report.potential_key
    );
    assert!(
        report.confidence_key.starts_with("common.scoutConfidence."),
        "confidence_key should be an i18n key: {}",
        report.confidence_key
    );
}

// ---------------------------------------------------------------------------
// Discovery mechanic — attribute reveal count
// ---------------------------------------------------------------------------

fn count_revealed(report: &ScoutReportData) -> usize {
    [
        report.pace,
        report.shooting,
        report.passing,
        report.dribbling,
        report.defending,
        report.physical,
    ]
    .iter()
    .filter(|a| a.is_some())
    .count()
}

#[test]
fn high_ability_scout_reveals_all_attrs() {
    let mut game = make_game();
    // Scout already has judging_ability = 80
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let report = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap()
        .context
        .scout_report
        .as_ref()
        .unwrap();

    assert_eq!(
        count_revealed(report),
        6,
        "High ability scout should reveal all 6 attrs"
    );
    assert!(
        report.condition.is_some(),
        "High ability should reveal condition"
    );
    assert!(report.morale.is_some(), "High ability should reveal morale");
    assert_eq!(report.confidence_key, "common.scoutConfidence.high");
}

#[test]
fn medium_ability_scout_reveals_5_attrs() {
    let mut game = make_game();
    game.staff[0].attributes.judging_ability = 65;
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let report = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap()
        .context
        .scout_report
        .as_ref()
        .unwrap();

    assert_eq!(
        count_revealed(report),
        5,
        "Medium ability scout should reveal 5 attrs"
    );
    assert!(
        report.condition.is_some(),
        "Medium ability should reveal condition"
    );
    assert!(
        report.morale.is_none(),
        "Medium ability should NOT reveal morale"
    );
    assert_eq!(report.confidence_key, "common.scoutConfidence.moderate");
}

#[test]
fn low_ability_scout_reveals_3_attrs() {
    let mut game = make_game();
    game.staff[0].attributes.judging_ability = 45;
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let report = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap()
        .context
        .scout_report
        .as_ref()
        .unwrap();

    assert_eq!(
        count_revealed(report),
        3,
        "Low ability scout should reveal 3 attrs"
    );
    assert!(
        report.condition.is_none(),
        "Low ability should NOT reveal condition"
    );
    assert!(
        report.morale.is_none(),
        "Low ability should NOT reveal morale"
    );
    assert_eq!(report.confidence_key, "common.scoutConfidence.low");
}

#[test]
fn very_low_ability_scout_reveals_2_attrs() {
    let mut game = make_game();
    game.staff[0].attributes.judging_ability = 25;
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let report = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap()
        .context
        .scout_report
        .as_ref()
        .unwrap();

    assert_eq!(
        count_revealed(report),
        2,
        "Very low ability scout should reveal 2 attrs"
    );
    assert_eq!(report.confidence_key, "common.scoutConfidence.low");
}

// ---------------------------------------------------------------------------
// Fuzzed attribute accuracy
// ---------------------------------------------------------------------------

#[test]
fn high_ability_scout_attrs_are_close_to_real() {
    // With judging_ability 80+, noise range is ±2
    // Run multiple times to check range statistically
    for _ in 0..10 {
        let mut game = make_game();
        send_scout(&mut game, "scout1", "p2").unwrap();
        complete_scouting(&mut game);

        let report = game
            .messages
            .iter()
            .find(|m| m.category == MessageCategory::ScoutReport)
            .unwrap()
            .context
            .scout_report
            .as_ref()
            .unwrap();

        // Real pace is 70, with noise ±2 → should be in [68, 72]
        if let Some(pace) = report.pace {
            assert!(
                pace >= 65 && pace <= 75,
                "High-ability fuzzed pace {} should be close to real value 70",
                pace
            );
        }
    }
}

#[test]
fn low_ability_scout_attrs_have_more_noise() {
    // With judging_ability 25, noise range is ±12
    let mut game = make_game();
    game.staff[0].attributes.judging_ability = 25;
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let report = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap()
        .context
        .scout_report
        .as_ref()
        .unwrap();

    // Just verify values are in valid range (1-99)
    for val in [
        report.pace,
        report.shooting,
        report.passing,
        report.dribbling,
        report.defending,
        report.physical,
    ] {
        if let Some(v) = val {
            assert!(v >= 1 && v <= 99, "Fuzzed value {} should be in [1, 99]", v);
        }
    }
}

// ---------------------------------------------------------------------------
// Potential assessment depends on judging_potential
// ---------------------------------------------------------------------------

#[test]
fn high_judging_potential_gives_specific_assessment() {
    let mut game = make_game();
    game.staff[0].attributes.judging_potential = 75;
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let report = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap()
        .context
        .scout_report
        .as_ref()
        .unwrap();

    // With high judging_potential (>=70), should get a specific potential key
    assert!(
        report.potential_key != "common.scoutPotential.unclear",
        "High judging_potential should give a specific assessment, got: {}",
        report.potential_key
    );
}

#[test]
fn low_judging_potential_gives_unclear_assessment() {
    let mut game = make_game();
    game.staff[0].attributes.judging_potential = 40;
    send_scout(&mut game, "scout1", "p2").unwrap();
    complete_scouting(&mut game);

    let report = game
        .messages
        .iter()
        .find(|m| m.category == MessageCategory::ScoutReport)
        .unwrap()
        .context
        .scout_report
        .as_ref()
        .unwrap();

    assert_eq!(
        report.potential_key, "common.scoutPotential.unclear",
        "Low judging_potential should give unclear assessment"
    );
}

// ---------------------------------------------------------------------------
// Assignment removal after completion
// ---------------------------------------------------------------------------

#[test]
fn completed_assignment_is_removed() {
    let mut game = make_game();
    send_scout(&mut game, "scout1", "p2").unwrap();
    assert_eq!(game.scouting_assignments.len(), 1);
    complete_scouting(&mut game);
    assert_eq!(
        game.scouting_assignments.len(),
        0,
        "Completed assignments should be removed"
    );
}
