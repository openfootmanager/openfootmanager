//! How long a scout is away, the day his report arrives, and what a youth report says it looked
//! for.

use super::*;

fn with_scout(judging_ability: u8, judging_potential: u8) -> Game {
    let mut game = make_game();
    game.staff = vec![make_scout(
        "scout1",
        "team1",
        judging_ability,
        judging_potential,
    )];
    game
}

fn youth_search_days(
    judging_potential: u8,
    region: YouthScoutingRegion,
    objective: YouthScoutingObjective,
) -> u32 {
    let mut game = with_scout(70, judging_potential);
    start_youth_scouting(&mut game, "scout1", region, objective, None).expect("a search starts");
    game.youth_scouting_assignments[0].days_remaining
}

/// Given scouts judging potential 80, 79, 60, 59, 40 and 39, each sent on a domestic, balanced
///       youth search
/// When the search starts
/// Then it takes 4, 5, 5, 6, 6 and 7 days: a weaker judge of potential searches for longer
#[test]
fn a_weaker_judge_of_potential_searches_for_longer() {
    let days: Vec<u32> = [80, 79, 60, 59, 40, 39]
        .into_iter()
        .map(|jp| {
            youth_search_days(
                jp,
                YouthScoutingRegion::Domestic,
                YouthScoutingObjective::Balanced,
            )
        })
        .collect();

    assert_eq!(days, vec![4, 5, 5, 6, 6, 7]);
}

/// Given a scout judging potential 80 (a four-day domestic, balanced search)
/// When he searches abroad, for high potential, and for both
/// Then each takes a day longer: 5, 5 and 6 days
#[test]
fn a_search_abroad_or_for_high_potential_takes_a_day_longer_each() {
    use YouthScoutingObjective::{Balanced, HighPotential};
    use YouthScoutingRegion::{Domestic, International};

    assert_eq!(
        (
            youth_search_days(80, International, Balanced),
            youth_search_days(80, Domestic, HighPotential),
            youth_search_days(80, International, HighPotential)
        ),
        (5, 5, 6)
    );
}

fn scout_reports(game: &Game) -> usize {
    game.messages
        .iter()
        .filter(|m| m.category == MessageCategory::ScoutReport)
        .count()
}

/// Given a scout judging ability 65 sent to watch a player: a three-day job
/// When three days of scouting go by
/// Then no report arrives on the first two, and exactly one on the third
#[test]
fn a_scout_report_arrives_on_the_day_the_assignment_ends_and_not_before() {
    let mut game = with_scout(65, 60);
    send_scout(&mut game, "scout1", "p2").expect("the scout is sent");

    let mut reports_by_day = Vec::new();
    for _ in 0..3 {
        process_scouting(&mut game);
        reports_by_day.push(scout_reports(&game));
        game.clock.advance_days(1);
    }

    assert_eq!(reports_by_day, vec![0, 0, 1]);
}

fn youth_reports(game: &Game) -> usize {
    game.messages
        .iter()
        .filter(|m| m.subject_key.as_deref() == Some("be.msg.youthRecruitmentReport.subject"))
        .count()
}

/// Given a scout judging potential 75 on a domestic, balanced youth search: five days
/// When ten days of scouting go by
/// Then nothing arrives for four days, the report arrives on the fifth, and only once
#[test]
fn a_youth_report_arrives_once_when_the_search_ends() {
    let mut game = with_scout(70, 75);
    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        None,
    )
    .expect("a search starts");

    let mut reports_by_day = Vec::new();
    for _ in 0..10 {
        process_scouting(&mut game);
        reports_by_day.push(youth_reports(&game));
        game.clock.advance_days(1);
    }

    assert_eq!(reports_by_day, vec![0, 0, 0, 0, 1, 1, 1, 1, 1, 1]);
}

/// Given a scouting assignment already due (no days left), as an old save can hold
/// When a day of scouting goes by
/// Then its report is delivered that day and the assignment is gone
#[test]
fn an_assignment_already_due_is_delivered_that_day() {
    let mut game = make_game();
    game.scouting_assignments
        .push(ofm_core::game::ScoutingAssignment {
            id: "due".to_string(),
            scout_id: "scout1".to_string(),
            player_id: "p2".to_string(),
            days_remaining: 0,
        });

    process_scouting(&mut game);

    assert_eq!(
        (scout_reports(&game), game.scouting_assignments.len()),
        (1, 0)
    );
}

fn target_label_of_search_for(position: Position) -> Option<String> {
    let mut game = make_game();
    start_youth_scouting(
        &mut game,
        "scout1",
        YouthScoutingRegion::Domestic,
        YouthScoutingObjective::Balanced,
        Some(position),
    )
    .expect("a search starts");
    for _ in 0..10 {
        process_scouting(&mut game);
        game.clock.advance_days(1);
    }
    game.messages
        .iter()
        .find(|m| m.subject_key.as_deref() == Some("be.msg.youthRecruitmentReport.subject"))
        .and_then(|m| m.i18n_params.get("targetLabel").cloned())
}

/// Given youth searches targeted at a goalkeeper, a defender, a midfielder and a forward
/// When each report arrives
/// Then it names the position searched for
#[test]
fn a_targeted_youth_report_names_the_position_searched_for() {
    let labels: Vec<Option<String>> = [
        Position::Goalkeeper,
        Position::Defender,
        Position::Midfielder,
        Position::Forward,
    ]
    .into_iter()
    .map(target_label_of_search_for)
    .collect();

    assert_eq!(
        labels,
        [
            "common.positions.Goalkeeper",
            "common.positions.Defender",
            "common.positions.Midfielder",
            "common.positions.Forward",
        ]
        .map(|key| Some(key.to_string()))
    );
}
