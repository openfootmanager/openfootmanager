use ofm_core::messages;

// ---------------------------------------------------------------------------
// welcome_message
// ---------------------------------------------------------------------------

#[test]
fn welcome_message_has_correct_fields() {
    let msg = messages::welcome_message("Test FC", "team1", "2025-06-01");
    assert_eq!(msg.id, "welcome_1");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert!(msg.sender.is_empty());
    assert!(msg.sender_role.is_empty());
    assert_eq!(
        msg.i18n_params.get("team").map(String::as_str),
        Some("Test FC")
    );
    assert!(!msg.actions.is_empty(), "Should have actions");
}

#[test]
fn welcome_message_has_i18n_keys() {
    let msg = messages::welcome_message("Test FC", "team1", "2025-06-01");
    assert!(msg.subject_key.is_some(), "Should have subject i18n key");
    assert!(msg.body_key.is_some(), "Should have body i18n key");
    assert!(msg.sender_key.is_some(), "Should have sender i18n key");
}

#[test]
fn welcome_message_has_context() {
    let msg = messages::welcome_message("Test FC", "team1", "2025-06-01");
    assert_eq!(msg.context.team_id.as_deref(), Some("team1"));
}

// ---------------------------------------------------------------------------
// season_schedule_message
// ---------------------------------------------------------------------------

#[test]
fn season_schedule_message_fields() {
    let msg = messages::season_schedule_message("Premier League", "2025-08-10", "2025-06-01");
    assert_eq!(msg.id, "season_1");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert_eq!(
        msg.i18n_params.get("league").map(String::as_str),
        Some("Premier League")
    );
    assert_eq!(
        msg.i18n_params.get("start").map(String::as_str),
        Some("2025-08-10")
    );
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.competitionSecretary")
    );
}

#[test]
fn season_schedule_has_view_action() {
    let msg = messages::season_schedule_message("Premier League", "2025-08-10", "2025-06-01");
    let has_view = msg.actions.iter().any(|a| a.id == "view_schedule");
    assert!(has_view, "Should have view schedule action");
}

// ---------------------------------------------------------------------------
// pre_match_message
// ---------------------------------------------------------------------------

#[test]
fn pre_match_message_home() {
    let msg = messages::pre_match_message(
        "f1",
        "Rival FC",
        "team2",
        true,
        5,
        "2025-09-15",
        "2025-09-12",
    );
    assert_eq!(msg.id, "prematch_f1");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert_eq!(msg.subject_key.as_deref(), Some("be.msg.preMatch.subject"));
    assert!(matches!(
        msg.body_key.as_deref(),
        Some("be.msg.preMatch.body0Home" | "be.msg.preMatch.body1Home")
    ));
    assert_eq!(
        msg.i18n_params.get("opponent").map(String::as_str),
        Some("Rival FC")
    );
    assert_eq!(msg.i18n_params.get("venue").map(String::as_str), Some("H"));
}

#[test]
fn pre_match_message_away() {
    let msg = messages::pre_match_message(
        "f2",
        "Rival FC",
        "team2",
        false,
        6,
        "2025-09-22",
        "2025-09-19",
    );
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert!(matches!(
        msg.body_key.as_deref(),
        Some("be.msg.preMatch.body0Away" | "be.msg.preMatch.body1Away")
    ));
    assert_eq!(msg.i18n_params.get("venue").map(String::as_str), Some("A"));
}

#[test]
fn pre_match_has_tactics_action() {
    let msg = messages::pre_match_message(
        "f1",
        "Rival FC",
        "team2",
        true,
        5,
        "2025-09-15",
        "2025-09-12",
    );
    let has_tactics = msg.actions.iter().any(|a| a.id == "set_tactics");
    let has_scout = msg.actions.iter().any(|a| a.id == "view_opponent");
    assert!(has_tactics, "Should have set tactics action");
    assert!(has_scout, "Should have scout opponent action");
}

#[test]
fn pre_match_context_has_fixture_id() {
    let msg = messages::pre_match_message(
        "f1",
        "Rival FC",
        "team2",
        true,
        5,
        "2025-09-15",
        "2025-09-12",
    );
    assert_eq!(msg.context.fixture_id.as_deref(), Some("f1"));
}

// ---------------------------------------------------------------------------
// match_result_message
// ---------------------------------------------------------------------------

#[test]
fn match_result_victory() {
    let msg = messages::match_result_message(
        "f1",
        "Test FC",
        "Rival FC",
        3,
        1,
        "team1",
        "team2",
        "team1",
        10,
        "2025-10-01",
    );
    assert_eq!(msg.id, "result_f1");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.matchResult.subject.victory")
    );
    assert!(matches!(
        msg.body_key.as_deref(),
        Some("be.msg.matchResult.body.victory0" | "be.msg.matchResult.body.victory1")
    ));
    assert_eq!(
        msg.i18n_params.get("homeGoals").map(String::as_str),
        Some("3")
    );
    assert_eq!(
        msg.i18n_params.get("awayGoals").map(String::as_str),
        Some("1")
    );
}

#[test]
fn match_result_defeat() {
    let msg = messages::match_result_message(
        "f2",
        "Test FC",
        "Rival FC",
        0,
        2,
        "team1",
        "team2",
        "team1",
        11,
        "2025-10-08",
    );
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.matchResult.subject.defeat")
    );
    assert!(matches!(
        msg.body_key.as_deref(),
        Some("be.msg.matchResult.body.defeat0" | "be.msg.matchResult.body.defeat1")
    ));
}

#[test]
fn match_result_draw() {
    let msg = messages::match_result_message(
        "f3",
        "Test FC",
        "Rival FC",
        1,
        1,
        "team1",
        "team2",
        "team1",
        12,
        "2025-10-15",
    );
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.matchResult.subject.draw")
    );
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.matchResult.body.draw")
    );
}

#[test]
fn match_result_away_perspective() {
    // User is team2 (away), and they won 0-2
    let msg = messages::match_result_message(
        "f4",
        "Home FC",
        "Test FC",
        0,
        2,
        "team1",
        "team2",
        "team2",
        13,
        "2025-10-22",
    );
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.matchResult.subject.victory")
    );
}

#[test]
fn match_result_has_context_with_score() {
    let msg = messages::match_result_message(
        "f1",
        "Test FC",
        "Rival FC",
        3,
        1,
        "team1",
        "team2",
        "team1",
        10,
        "2025-10-01",
    );
    assert_eq!(msg.context.fixture_id.as_deref(), Some("f1"));
    let result = msg.context.match_result.as_ref().unwrap();
    assert_eq!(result.home_goals, 3);
    assert_eq!(result.away_goals, 1);
}

#[test]
fn victory_has_normal_priority() {
    let msg =
        messages::match_result_message("f1", "A", "B", 2, 0, "t1", "t2", "t1", 1, "2025-01-01");
    assert_eq!(msg.priority, domain::message::MessagePriority::Normal);
}

#[test]
fn defeat_has_high_priority() {
    let msg =
        messages::match_result_message("f1", "A", "B", 0, 2, "t1", "t2", "t1", 1, "2025-01-01");
    assert_eq!(msg.priority, domain::message::MessagePriority::High);
}

// ---------------------------------------------------------------------------
// staff_advice_message
// ---------------------------------------------------------------------------

#[test]
fn staff_advice_message_fields() {
    let msg = messages::staff_advice_message("Test FC", "team1", "2025-06-01");
    assert_eq!(msg.id, "staff_advice_1");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.staffAdvice.subject")
    );
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.staffAdvice.body"));
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.assistantManager")
    );
}

#[test]
fn staff_advice_has_view_action() {
    let msg = messages::staff_advice_message("Test FC", "team1", "2025-06-01");
    let has_view = msg.actions.iter().any(|a| a.id == "view_staff");
    assert!(has_view, "Should have view staff action");
}

// ---------------------------------------------------------------------------
// board_expectations_message
// ---------------------------------------------------------------------------

#[test]
fn board_expectations_message_fields() {
    let msg = messages::board_expectations_message("Test FC", "team1", "2025-06-01");
    assert_eq!(msg.id, "board_expect_1");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.boardExpect.subject")
    );
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.boardExpect.body"));
    assert_eq!(
        msg.i18n_params.get("team").map(String::as_str),
        Some("Test FC")
    );
    assert_eq!(msg.sender_role_key.as_deref(), Some("be.role.chairman"));
}

#[test]
fn board_expectations_has_ack_action() {
    let msg = messages::board_expectations_message("Test FC", "team1", "2025-06-01");
    let has_ack = msg.actions.iter().any(|a| a.id == "ack_objectives");
    assert!(has_ack, "Should have acknowledge action");
}

// ---------------------------------------------------------------------------
// transfer_complete_message
// ---------------------------------------------------------------------------

#[test]
fn transfer_message_millions() {
    let msg = messages::transfer_complete_message("John Star", 5_500_000, "2025-08-01");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.transferComplete.subject")
    );
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.transferComplete.body")
    );
    assert_eq!(
        msg.i18n_params.get("player").map(String::as_str),
        Some("John Star")
    );
    assert_eq!(
        msg.i18n_params.get("fee").map(String::as_str),
        Some("€5.5M")
    );
    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.transferCommittee")
    );
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.directorOfFootball")
    );
}

#[test]
fn transfer_message_thousands() {
    let msg = messages::transfer_complete_message("Young Player", 250_000, "2025-08-01");
    assert_eq!(
        msg.i18n_params.get("fee").map(String::as_str),
        Some("€250K")
    );
}

#[test]
fn transfer_message_small_fee() {
    let msg = messages::transfer_complete_message("Free Agent", 500, "2025-08-01");
    assert_eq!(msg.i18n_params.get("fee").map(String::as_str), Some("€500"));
}

#[test]
fn transfer_message_has_unique_id() {
    let msg1 = messages::transfer_complete_message("A", 1000, "2025-08-01");
    let msg2 = messages::transfer_complete_message("B", 2000, "2025-08-01");
    assert_ne!(msg1.id, msg2.id, "Transfer messages should have unique IDs");
}

#[test]
fn incoming_transfer_offer_message_uses_i18n_keys_and_params() {
    let msg = messages::incoming_transfer_offer_message(
        "offer-1",
        "player-1",
        "John Star",
        "Rival FC",
        1_250_000,
        "2025-08-01",
    );

    assert_eq!(msg.id, "transfer_offer_offer-1");
    assert!(msg.subject.is_empty());
    assert!(msg.body.is_empty());
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.transferOffer.subject")
    );
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.transferOffer.body"));
    assert_eq!(
        msg.i18n_params.get("player").map(String::as_str),
        Some("John Star")
    );
    assert_eq!(
        msg.i18n_params.get("team").map(String::as_str),
        Some("Rival FC")
    );
    assert_eq!(
        msg.i18n_params.get("fee").map(String::as_str),
        Some("€1.2M")
    );
    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.directorOfFootball")
    );
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.directorOfFootball")
    );
    assert!(msg.actions.iter().any(|action| {
        action.id == "view_transfers"
            && action.label.is_empty()
            && action.label_key.as_deref() == Some("be.msg.transferOffer.actionReview")
    }));
}
