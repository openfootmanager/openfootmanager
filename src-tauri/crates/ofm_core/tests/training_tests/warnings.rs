//! The staff's fitness warning: when the squad's average is low enough to send it, whose training
//! schedule it describes, and who signs it. Each scenario puts another club, or another club's
//! staff, first in the list, so a lookup that matched the wrong club would show.

use super::*;
use domain::message::InboxMessage;

/// Every player of the user's squad at `condition`, then the day's fitness check: the message it
/// sent, if any.
fn warning_with_squad_at(game: &mut Game, condition: u8) -> Option<InboxMessage> {
    for player in game.players.iter_mut() {
        player.condition = condition;
    }
    training::check_squad_fitness_warnings(game);
    game.messages
        .iter()
        .find(|message| message.id.starts_with("fitness_warn_"))
        .cloned()
}

/// Given a squad averaging exactly 50, with nobody below 40
/// When the staff's fitness check runs
/// Then no warning is sent: only an average below 50 is a warning
#[test]
fn a_squad_averaging_exactly_fifty_gets_no_warning() {
    let mut game = make_game();

    let warning = warning_with_squad_at(&mut game, 50);

    assert_eq!(warning.map(|message| message.id), None);
}

/// Given the user's club on the intense schedule, and another club on the light one listed first
/// When a squad averaging 40 gets its warning
/// Then the warning describes the intense schedule, in its body and in its parameters
#[test]
fn the_warning_describes_the_users_own_training_schedule() {
    let mut game = make_game();
    game.teams[0].training_schedule = TrainingSchedule::Intense;
    let mut other = make_team("team0", "Other Town");
    other.training_schedule = TrainingSchedule::Light;
    game.teams.insert(0, other);

    let warning = warning_with_squad_at(&mut game, 40).expect("an average of 40 is a warning");

    assert_eq!(
        (
            warning.body_key.as_deref(),
            warning.i18n_params.get("schedule").map(String::as_str)
        ),
        (Some("be.msg.fitness.warning.body.intense"), Some("intense"))
    );
}

/// Given the user's club with a coach but no physio, and another club's physio
/// When a squad averaging 40 gets its warning
/// Then the assistant manager signs it: only the user's own physio does
#[test]
fn another_clubs_physio_does_not_sign_the_users_warning() {
    let mut game = make_game();
    game.staff.retain(|member| member.role != StaffRole::Physio);
    game.staff.insert(
        0,
        make_staff("their-physio", "team0", StaffRole::Physio, 30, 80),
    );

    let warning = warning_with_squad_at(&mut game, 40).expect("an average of 40 is a warning");

    assert_eq!(
        warning.sender_role_key.as_deref(),
        Some("be.role.assistantManager")
    );
}

/// Given the user's club with no physio, a coach and an assistant manager, and another club's
///       assistant manager listed before all of them
/// When a squad averaging 40 gets its warning
/// Then it carries the name of the user's own assistant manager
#[test]
fn the_warning_carries_the_users_own_assistant_managers_name() {
    let mut game = make_game();
    game.staff.retain(|member| member.role != StaffRole::Physio);
    game.staff.push(make_staff(
        "assistant",
        "team1",
        StaffRole::AssistantManager,
        60,
        30,
    ));
    game.staff.insert(
        0,
        make_staff(
            "their-assistant",
            "team0",
            StaffRole::AssistantManager,
            60,
            30,
        ),
    );

    let warning = warning_with_squad_at(&mut game, 40).expect("an average of 40 is a warning");

    assert_eq!(warning.sender, "Staff assistant");
}

/// Given the user's club with its own physio, and another club's physio listed before him
/// When a squad averaging 40 gets its warning
/// Then it carries the name of the user's own physio
#[test]
fn the_warning_carries_the_users_own_physios_name() {
    let mut game = make_game();
    game.staff.insert(
        0,
        make_staff("their-physio", "team0", StaffRole::Physio, 30, 80),
    );

    let warning = warning_with_squad_at(&mut game, 40).expect("an average of 40 is a warning");

    assert_eq!(warning.sender, "Staff physio1");
}
