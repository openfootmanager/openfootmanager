use super::*;

// ---------------------------------------------------------------------------
// advance_to_next_season: the one rollover sequence
// ---------------------------------------------------------------------------

/// The command a player triggers and the season harness both roll a season over
/// with this function, so the order they get is the same order by construction.
/// It used to be inline in the Tauri command, which left the harness to copy it.
#[test]
fn advancing_refuses_a_season_that_is_not_complete_and_touches_nothing() {
    let mut game = make_completed_season_game();
    let league = game.league.as_mut().unwrap();
    league.fixtures[1].status = FixtureStatus::Scheduled;
    league.fixtures[1].result = None;
    assert!(
        !is_season_complete(&game),
        "the precondition: an unplayed fixture holds the season open"
    );
    let satisfaction = game.manager.satisfaction;

    let refusal = advance_to_next_season(&mut game).expect_err("an unfinished season must refuse");

    assert_eq!(refusal, "be.error.seasonNotComplete");
    assert!(
        game.competitions.is_empty(),
        "a refused rollover must not promote the legacy league into competitions"
    );
    assert_eq!(game.league.as_ref().unwrap().season, 1);
    assert_eq!(game.manager.satisfaction, satisfaction);
}

#[test]
fn advancing_a_complete_season_rolls_it_over_and_returns_the_summary() {
    let mut game = make_completed_season_game();
    assert!(is_season_complete(&game));

    let summary = advance_to_next_season(&mut game).expect("a finished season advances");

    assert_eq!(
        summary.season, 1,
        "the summary describes the season just ended"
    );
    assert_eq!(summary.champion_id, "team1");
    let regenerated = game
        .competitions
        .first()
        .expect("the legacy league is promoted into competitions by the rollover");
    assert!(
        regenerated.season > summary.season,
        "the competition moves past the season that just ended, got {}",
        regenerated.season
    );
    assert!(
        regenerated.standings.iter().all(|row| row.played == 0),
        "and starts the new season on a fresh table"
    );
    let clubs: Vec<&str> = regenerated
        .standings
        .iter()
        .map(|row| row.team_id.as_str())
        .collect();
    assert_eq!(
        clubs.len(),
        2,
        "the fresh table has exactly the two clubs: {clubs:?}"
    );
    assert!(
        clubs.contains(&"team1") && clubs.contains(&"team2"),
        "both clubs stay in the fresh table: {clubs:?}"
    );
    assert!(
        game.manager.team_id.is_some(),
        "a manager with a healthy board keeps the job"
    );
}

/// Given a finished season (on the 20th, not the 1st, so the monthly sweep cannot be the cause)
///       with read mail from over a year ago, read mail from last week, and unread old mail
/// When the season is rolled over
/// Then the rollover itself archives the old read mail, and leaves the rest in the inbox.
#[test]
fn rolling_a_season_over_archives_old_read_mail() {
    let mut game = make_completed_season_game();
    let mail = |id: &str, date: &str, read: bool| {
        let mut message = domain::message::InboxMessage::new(
            id.to_string(),
            "Subject".to_string(),
            "Body".to_string(),
            "Board".to_string(),
            date.to_string(),
        );
        message.read = read;
        message
    };
    game.messages = vec![
        mail("old-read", "2025-01-01", true),
        mail("old-unread", "2025-01-01", false),
        mail("recent-read", "2026-05-13", true),
    ];

    advance_to_next_season(&mut game).expect("a finished season advances");

    let archived: Vec<&str> = game
        .archived_messages
        .iter()
        .map(|message| message.id.as_str())
        .collect();
    assert_eq!(archived, ["old-read"]);
    for kept in ["old-unread", "recent-read"] {
        assert!(
            game.messages.iter().any(|message| message.id == kept),
            "{kept}"
        );
    }
}

/// Given a finished season rolled over once, when advancing is asked for again,
/// then it is refused and no second youth intake is taken. The intake keeps no
/// marker of its own — like aging, it runs inside the one rollover, and the
/// rollover's completeness check is what makes it once a season.
#[test]
fn a_season_takes_one_youth_intake_however_often_advancing_is_asked_for() {
    let mut game = make_completed_season_game();
    let before = game.players.len();

    advance_to_next_season(&mut game).expect("a finished season advances");
    let after_one = game.players.len();
    assert!(
        after_one > before,
        "the rollover takes the season's youth intake"
    );

    assert!(advance_to_next_season(&mut game).is_err());
    assert_eq!(
        game.players.len(),
        after_one,
        "a second advance took a second intake"
    );
}

/// The rollover itself is what sacks this manager: they are safe at satisfaction
/// 20 going in, and the board's failed objective takes 15 off on the way out,
/// leaving 5 on a standing warning. Checking *before* the rollover would keep
/// them, so this fails if the two steps are ever run in the other order.
#[test]
fn advancing_runs_the_firing_check_after_the_rollover() {
    let mut game = make_completed_season_game();
    game.manager.satisfaction = 20;
    game.manager.warning_stage = 1;
    game.board_objectives.push(BoardObjective {
        id: "obj1".to_string(),
        objective_type: ObjectiveType::LeaguePosition,
        description: "Finish top 1".to_string(),
        target: 1,
        met: false,
    });

    advance_to_next_season(&mut game).expect("a finished season advances");

    assert!(
        game.manager.team_id.is_none(),
        "a manager the rollover pushes past the threshold is sacked by it, not a day later"
    );
}
