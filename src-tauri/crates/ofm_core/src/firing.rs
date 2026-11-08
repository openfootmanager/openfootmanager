use crate::game::Game;
use domain::message::*;
use domain::news::{NewsArticle, NewsCategory};
use std::collections::HashMap;

/// The satisfaction at or below which the board warns its manager (`manager_standing` reads it
/// for the words the Manager tab shows).
pub(crate) const WARN_THRESHOLD: u8 = 25;
const FINAL_WARN_THRESHOLD: u8 = 18;
const FIRE_THRESHOLD: u8 = 10;
/// The satisfaction from which the board is satisfied with its manager: the Manager tab's
/// "satisfied" band starts here (`manager_standing`), and here the board forgives him and its
/// warning lapses (maintainer, 4 Nov 2026). One constant, so the tab can never call a board
/// satisfied while it still holds a warning; well clear of the warning threshold, so a manager
/// hovering near 25 is not sent a fresh letter each time he dips back to it.
pub(crate) const SATISFIED_FROM: u8 = 50;

const WARNING_ID_PREFIX: &str = "board_warning";
const FINAL_WARNING_ID_PREFIX: &str = "board_final_warning";
const FIRED_ID_PREFIX: &str = "board_fired";

// warning_stage on Manager: 0 = none, 1 = warning issued, 2 = final warning issued.
// Reset on hire/fire so warnings don't carry across clubs, and when the board's satisfaction climbs
// back to SATISFIED_FROM, so a manager who recovered is warned afresh before he is sacked.
/// The board has warned its manager once.
pub(crate) const STAGE_WARNING: u8 = 1;
/// The board has given its manager a final warning.
pub(crate) const STAGE_FINAL: u8 = 2;

enum FiringDecision {
    NoAction,
    Warning,
    FinalWarning,
    Fire,
    /// The board is satisfied again (from [`SATISFIED_FROM`]): its warning lapses.
    LiftWarning,
}

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Check manager satisfaction and issue warnings or fire.
/// Returns `true` if the manager was fired.
pub fn check_manager_firing(game: &mut Game) -> bool {
    game.sync_user_manager_record();

    let user_fired = check_user_manager_firing(game);
    check_ai_manager_firings(game);
    game.sync_user_manager_record();

    user_fired
}

/// What the board does at `satisfaction` with its manager at warning `stage`. One rule for the user and
/// every AI manager.
///
/// From [`SATISFIED_FROM`] a warning on record lapses (#1249); between the warning threshold and it,
/// the warning stands and no new letter is sent. It used to stay until the manager was hired or
/// sacked, so a manager who recovered and later slid again was sacked at 10 with no fresh warning,
/// while the Manager tab told him the board was satisfied.
fn firing_decision(satisfaction: u8, stage: u8) -> FiringDecision {
    if satisfaction > WARN_THRESHOLD {
        return if satisfaction >= SATISFIED_FROM && stage > 0 {
            FiringDecision::LiftWarning
        } else {
            FiringDecision::NoAction
        };
    }
    if satisfaction <= FIRE_THRESHOLD {
        if stage >= STAGE_WARNING {
            FiringDecision::Fire
        } else {
            FiringDecision::Warning
        }
    } else if satisfaction <= FINAL_WARN_THRESHOLD {
        if stage < STAGE_FINAL {
            FiringDecision::FinalWarning
        } else {
            FiringDecision::NoAction
        }
    } else if satisfaction <= WARN_THRESHOLD && stage < STAGE_WARNING {
        FiringDecision::Warning
    } else {
        FiringDecision::NoAction
    }
}

fn check_user_manager_firing(game: &mut Game) -> bool {
    if game.manager.team_id.is_none() {
        return false;
    }

    let satisfaction = game.manager.satisfaction;
    let stage = game.manager.warning_stage;

    match firing_decision(satisfaction, stage) {
        FiringDecision::Fire => {
            execute_firing(game);
            return true;
        }
        FiringDecision::Warning => send_warning(game),
        FiringDecision::FinalWarning => {
            send_final_warning(game);
        }
        FiringDecision::LiftWarning => game.manager.warning_stage = 0,
        FiringDecision::NoAction => {}
    }

    false
}

fn check_ai_manager_firings(game: &mut Game) {
    let user_manager_id = if game.manager_id.is_empty() {
        game.manager.id.clone()
    } else {
        game.manager_id.clone()
    };

    let ai_manager_ids: Vec<String> = game
        .managers
        .iter()
        .filter(|manager| manager.id != user_manager_id && manager.team_id.is_some())
        .map(|manager| manager.id.clone())
        .collect();

    for manager_id in ai_manager_ids {
        let Some(index) = game
            .managers
            .iter()
            .position(|manager| manager.id == manager_id)
        else {
            continue;
        };

        let satisfaction = game.managers[index].satisfaction;
        let stage = game.managers[index].warning_stage;

        match firing_decision(satisfaction, stage) {
            FiringDecision::Warning => {
                game.managers[index].warning_stage = STAGE_WARNING;
            }
            FiringDecision::FinalWarning => {
                game.managers[index].warning_stage = STAGE_FINAL;
            }
            FiringDecision::Fire => {
                execute_ai_firing(game, index);
            }
            FiringDecision::LiftWarning => {
                game.managers[index].warning_stage = 0;
            }
            FiringDecision::NoAction => {}
        }
    }
}

fn execute_ai_firing(game: &mut Game, manager_index: usize) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    execute_ai_firing_on_date(game, manager_index, &today);
}

fn execute_ai_firing_on_date(game: &mut Game, manager_index: usize, today: &str) {
    let manager = game.managers[manager_index].clone();
    let team_id = manager.team_id.clone().unwrap_or_default();
    let team_name = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_default();

    if let Some(team) = game.teams.iter_mut().find(|team| team.id == team_id) {
        team.manager_id = None;
    }

    game.managers[manager_index].fire(today);
    // A stand-in was never a candidate: they go back to being only an assistant, so the directory
    // and the unemployed pool never list them while they wait for the season-end clear-out.
    if game.managers[manager_index].interim {
        let stand_in = game.managers.remove(manager_index);
        game.retired_managers.push(stand_in);
    }
    game.news.push(ai_managerial_change_article(
        &manager.id,
        &manager.full_name(),
        &team_id,
        &team_name,
        today,
    ));
}

pub fn fire_ai_manager_for_team(game: &mut Game, team_id: &str, date: &str) -> bool {
    let user_manager_id = if game.manager_id.is_empty() {
        game.manager.id.clone()
    } else {
        game.manager_id.clone()
    };

    let Some(index) = game.managers.iter().position(|manager| {
        manager.id != user_manager_id && manager.team_id.as_deref() == Some(team_id)
    }) else {
        return false;
    };

    execute_ai_firing_on_date(game, index, date);
    true
}

fn ai_managerial_change_article(
    manager_id: &str,
    manager_name: &str,
    team_id: &str,
    team_name: &str,
    date: &str,
) -> NewsArticle {
    NewsArticle::new(
        format!("managerial_change_{}_{}", team_id, date),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
        NewsCategory::ManagerialChange,
    )
    .with_teams(vec![team_id.to_string()])
    .with_i18n(
        "be.news.managerialChange.headline",
        "be.news.managerialChange.body",
        "be.source.leagueWire",
        params(&[
            ("team", team_name),
            ("manager", manager_name),
            ("managerId", manager_id),
        ]),
    )
}

fn execute_firing(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_id = game.manager.team_id.clone().unwrap_or_default();
    let team_name = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    // Clear manager from team
    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.manager_id = None;
    }

    // Close career history and unassign
    game.manager
        .ensure_open_career_entry(&team_id, &team_name, &today);
    game.manager.fire(&today);

    // Send dismissal message (unique ID so it doesn't collide with a future firing at another club)
    let msg = InboxMessage::new(
        format!("{}_{}_{}", FIRED_ID_PREFIX, team_id, today),
        String::new(),
        String::new(),
        String::new(),
        today,
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.boardFired.subject",
        "be.msg.boardFired.body",
        params(&[("team", &team_name)]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");

    game.messages.push(msg);
}

fn send_warning(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_id = game.manager.team_id.clone().unwrap_or_default();
    let team_name = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    game.manager.warning_stage = STAGE_WARNING;

    let msg = InboxMessage::new(
        format!("{}_{}_{}", WARNING_ID_PREFIX, team_id, today),
        String::new(),
        String::new(),
        String::new(),
        today,
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_i18n(
        "be.msg.boardWarning.subject",
        "be.msg.boardWarning.body",
        params(&[("team", &team_name)]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");

    game.messages.push(msg);
}

fn send_final_warning(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_id = game.manager.team_id.clone().unwrap_or_default();
    let team_name = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    game.manager.warning_stage = STAGE_FINAL;

    let msg = InboxMessage::new(
        format!("{}_{}_{}", FINAL_WARNING_ID_PREFIX, team_id, today),
        String::new(),
        String::new(),
        String::new(),
        today,
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.boardFinalWarning.subject",
        "be.msg.boardFinalWarning.body",
        params(&[("team", &team_name)]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");

    game.messages.push(msg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::{Manager, ManagerCareerEntry};
    use domain::news::NewsCategory;
    use domain::team::Team;

    fn make_game(satisfaction: u8) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 10, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());
        manager.satisfaction = satisfaction;
        manager.career_history.push(ManagerCareerEntry {
            team_id: "team1".to_string(),
            team_name: "Test FC".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: None,
            matches: 10,
            wins: 2,
            draws: 3,
            losses: 5,
            best_league_position: Some(12),
        });

        let mut team = Team::new(
            "team1".to_string(),
            "Test FC".to_string(),
            "TST".to_string(),
            "England".to_string(),
            "Testville".to_string(),
            "Test Ground".to_string(),
            20_000,
        );
        team.manager_id = Some("mgr1".to_string());

        Game::new(clock, manager, vec![team], vec![], vec![], vec![])
    }

    /// Given a save from before career history existed: the manager has a club, an empty history
    /// and no matches on record, because `matches_managed` only grows at season end
    /// When the board sacks the manager in that first season
    /// Then the career stage is Unemployed (job offers), not AwaitingClub (any club for free).
    #[test]
    fn a_pre_history_save_sacked_in_its_first_season_is_unemployed() {
        let mut game = make_game(0);
        game.manager.career_history.clear();
        game.manager.career_stats.matches_managed = 0;

        execute_firing(&mut game);

        assert_eq!(
            crate::career_stage::career_stage(&game),
            crate::career_stage::CareerStage::Unemployed
        );
        let entry = game
            .manager
            .career_history
            .first()
            .expect("backfilled entry");
        assert_eq!(entry.team_id, "team1");
        assert_eq!(entry.team_name, "Test FC");
        assert_eq!(entry.end_date.as_deref(), Some("2026-10-15"));
    }

    #[test]
    fn no_action_when_satisfaction_above_warning_threshold() {
        let mut game = make_game(50);
        let fired = check_manager_firing(&mut game);
        assert!(!fired);
        assert!(game.manager.team_id.is_some());
        assert!(game.messages.is_empty());
    }

    #[test]
    fn warning_sent_at_warning_threshold() {
        let mut game = make_game(25);
        let fired = check_manager_firing(&mut game);
        assert!(!fired);
        assert!(game.manager.team_id.is_some());
        assert_eq!(game.messages.len(), 1);
        assert!(game.messages[0].id.starts_with(WARNING_ID_PREFIX));
        assert_eq!(game.messages[0].priority, MessagePriority::High);
        assert_eq!(
            game.messages[0].subject_key.as_deref(),
            Some("be.msg.boardWarning.subject")
        );
        assert_eq!(
            game.messages[0].body_key.as_deref(),
            Some("be.msg.boardWarning.body")
        );
        assert_eq!(
            game.messages[0].sender_key.as_deref(),
            Some("be.sender.boardOfDirectors")
        );
        assert_eq!(
            game.messages[0].sender_role_key.as_deref(),
            Some("be.role.chairman")
        );
        assert_eq!(game.manager.warning_stage, STAGE_WARNING);
    }

    #[test]
    fn final_warning_sent_at_final_warning_threshold() {
        let mut game = make_game(18);
        let fired = check_manager_firing(&mut game);
        assert!(!fired);
        assert!(game.manager.team_id.is_some());
        assert_eq!(game.messages.len(), 1);
        assert!(game.messages[0].id.starts_with(FINAL_WARNING_ID_PREFIX));
        assert_eq!(game.messages[0].priority, MessagePriority::Urgent);
        assert_eq!(
            game.messages[0].subject_key.as_deref(),
            Some("be.msg.boardFinalWarning.subject")
        );
        assert_eq!(
            game.messages[0].body_key.as_deref(),
            Some("be.msg.boardFinalWarning.body")
        );
        assert_eq!(game.manager.warning_stage, STAGE_FINAL);
    }

    #[test]
    fn not_fired_at_fire_threshold_without_prior_warning() {
        let mut game = make_game(5);
        let fired = check_manager_firing(&mut game);
        assert!(!fired);
        assert!(game.manager.team_id.is_some());
        // Should send the initial warning first (normal progression)
        assert_eq!(game.messages.len(), 1);
        assert!(game.messages[0].id.starts_with(WARNING_ID_PREFIX));
    }

    #[test]
    fn fired_at_fire_threshold_with_prior_warning() {
        let mut game = make_game(5);
        game.manager.warning_stage = STAGE_WARNING;

        let fired = check_manager_firing(&mut game);
        assert!(fired);
        assert!(game.manager.team_id.is_none());
        assert_eq!(game.messages.len(), 1);
        assert!(game.messages[0].id.starts_with(FIRED_ID_PREFIX));
        assert_eq!(game.messages[0].priority, MessagePriority::Urgent);
        assert_eq!(
            game.messages[0].subject_key.as_deref(),
            Some("be.msg.boardFired.subject")
        );
        assert_eq!(
            game.messages[0].body_key.as_deref(),
            Some("be.msg.boardFired.body")
        );
        assert_eq!(
            game.messages[0].sender_key.as_deref(),
            Some("be.sender.boardOfDirectors")
        );
        assert_eq!(
            game.messages[0].sender_role_key.as_deref(),
            Some("be.role.chairman")
        );
    }

    #[test]
    fn career_history_closed_on_firing() {
        let mut game = make_game(5);
        game.manager.warning_stage = STAGE_FINAL;

        check_manager_firing(&mut game);
        let entry = &game.manager.career_history[0];
        assert_eq!(entry.end_date, Some("2026-10-15".to_string()));
    }

    #[test]
    fn team_manager_id_cleared_on_firing() {
        let mut game = make_game(5);
        game.manager.warning_stage = STAGE_WARNING;

        check_manager_firing(&mut game);
        assert!(game.teams[0].manager_id.is_none());
    }

    #[test]
    fn warning_stage_does_not_carry_across_clubs() {
        // Manager previously warned/fired at an old club; after re-hire,
        // a new ≤10 satisfaction drop must not instantly fire them.
        let mut game = make_game(5);
        game.manager.warning_stage = 0; // simulate fresh hire
        let fired = check_manager_firing(&mut game);
        assert!(!fired);
        assert!(game.manager.team_id.is_some());
        assert_eq!(game.messages.len(), 1);
        assert!(game.messages[0].id.starts_with(WARNING_ID_PREFIX));
    }

    /// Given a manager on a final warning whose board climbs back to 50, then slides to 24
    /// When the board checks him at 50 and again at 24
    /// Then at 50, satisfied, the warning lapses, and at 24 he gets a fresh warning and keeps his job (the old
    ///      warning stayed on record, so 24 brought no letter and 10 would have sacked him unwarned)
    #[test]
    fn a_manager_who_recovers_is_warned_afresh_when_he_slips_again() {
        let mut game = make_game(50);
        game.manager.warning_stage = STAGE_FINAL;

        check_manager_firing(&mut game);
        assert_eq!(game.manager.warning_stage, 0);
        assert!(game.messages.is_empty());

        game.manager.satisfaction = 24;
        let fired = check_manager_firing(&mut game);

        assert!(!fired);
        assert_eq!(game.manager.warning_stage, STAGE_WARNING);
        let ids: Vec<&str> = game.messages.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec![format!("{WARNING_ID_PREFIX}_team1_2026-10-15")]);
    }

    /// Given a warned manager whose board is at 25, 26, 40 or 49
    /// When the board checks him
    /// Then the warning stands and no second letter is sent: the board forgives only once it is
    ///      satisfied, at 50
    #[test]
    fn a_warning_stands_until_the_board_is_satisfied_again() {
        for satisfaction in [25, 26, 40, 49] {
            let mut game = make_game(satisfaction);
            game.manager.warning_stage = STAGE_WARNING;

            check_manager_firing(&mut game);

            assert_eq!(
                game.manager.warning_stage, STAGE_WARNING,
                "at {satisfaction}"
            );
            assert!(game.messages.is_empty(), "at {satisfaction}");
        }
    }

    /// Given two AI managers on a final warning, one whose board is back to 50 and one at 49
    /// When the boards check their managers
    /// Then the first's warning lapses and the second's stands: one rule for the user and every AI
    ///      manager
    #[test]
    fn an_ai_managers_warning_lapses_when_his_board_recovers() {
        let mut game = make_game(60);
        let mut ai_manager = Manager::new(
            "mgr2".to_string(),
            "Marco".to_string(),
            "Rossi".to_string(),
            "1978-03-12".to_string(),
            "Italy".to_string(),
        );
        ai_manager.hire("team2".to_string());
        ai_manager.satisfaction = 50;
        ai_manager.warning_stage = STAGE_FINAL;
        let mut still_warned = ai_manager.clone();
        still_warned.id = "mgr3".to_string();
        still_warned.hire("team3".to_string());
        // A hire clears the warning stage, so his final warning is set after it.
        still_warned.warning_stage = STAGE_FINAL;
        still_warned.satisfaction = 49;
        game.managers = vec![game.manager.clone(), ai_manager, still_warned];

        check_manager_firing(&mut game);

        let stage_of = |id: &str| {
            game.managers
                .iter()
                .find(|m| m.id == id)
                .map(|m| m.warning_stage)
        };
        assert_eq!(stage_of("mgr2"), Some(0));
        assert_eq!(stage_of("mgr3"), Some(STAGE_FINAL));
    }

    #[test]
    fn warning_message_ids_are_unique_per_club_and_date() {
        let mut game = make_game(25);
        check_manager_firing(&mut game);
        let first_id = game.messages[0].id.clone();
        assert!(first_id.contains("team1"));
        assert!(first_id.contains("2026-10-15"));
    }

    #[test]
    fn warning_deduplication() {
        let mut game = make_game(25);
        check_manager_firing(&mut game);
        assert_eq!(game.messages.len(), 1);
        // Call again — should not add a second warning
        check_manager_firing(&mut game);
        assert_eq!(game.messages.len(), 1);
    }

    #[test]
    fn no_action_when_manager_has_no_team() {
        let mut game = make_game(5);
        game.manager.team_id = None;
        let fired = check_manager_firing(&mut game);
        assert!(!fired);
        assert!(game.messages.is_empty());
    }

    #[test]
    fn ai_manager_firing_generates_news_without_firing_user_manager() {
        let mut game = make_game(60);

        let mut ai_team = Team::new(
            "team2".to_string(),
            "Rival FC".to_string(),
            "RIV".to_string(),
            "England".to_string(),
            "Riverton".to_string(),
            "Rival Ground".to_string(),
            18_000,
        );
        ai_team.manager_id = Some("mgr2".to_string());
        game.teams.push(ai_team);

        let mut ai_manager = Manager::new(
            "mgr2".to_string(),
            "Marco".to_string(),
            "Rossi".to_string(),
            "1978-03-12".to_string(),
            "Italy".to_string(),
        );
        ai_manager.hire("team2".to_string());
        ai_manager.satisfaction = 5;
        ai_manager.warning_stage = STAGE_WARNING;

        game.managers = vec![game.manager.clone(), ai_manager];

        let fired = check_manager_firing(&mut game);

        assert!(!fired, "AI firings should not report the user as fired");
        assert_eq!(game.manager.team_id, Some("team1".to_string()));
        assert!(
            game.teams
                .iter()
                .find(|team| team.id == "team2")
                .and_then(|team| team.manager_id.clone())
                .is_none(),
            "The AI club should become vacant after the firing"
        );
        assert!(
            game.managers
                .iter()
                .find(|manager| manager.id == "mgr2")
                .and_then(|manager| manager.team_id.clone())
                .is_none(),
            "The fired AI manager should no longer be attached to the club"
        );
        assert!(
            game.news.iter().any(|article| {
                article.category == NewsCategory::ManagerialChange
                    && article.team_ids.contains(&"team2".to_string())
                    && article.headline.is_empty()
                    && article.body.is_empty()
                    && article.source.is_empty()
                    && article.headline_key.as_deref() == Some("be.news.managerialChange.headline")
                    && article.body_key.as_deref() == Some("be.news.managerialChange.body")
                    && article.source_key.as_deref() == Some("be.source.leagueWire")
            }),
            "An AI managerial dismissal should create a managerial-change news article"
        );
    }
}
