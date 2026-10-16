//! An AI club's stand-in manager hands the post to a permanent appointment.
//!
//! When an AI club falls vacant its assistant covers (`ai_hiring`), marked `interim`. Nothing
//! used to replace the assistant, so a stop-gap became permanent and the unemployed manager
//! market was never drawn from. Here a club that has had a stand-in for a month appoints the
//! unemployed manager whose reputation best matches its own, and the stand-in goes back to
//! being only an assistant (their manager record moves to the archive).

use chrono::NaiveDate;
use domain::manager::{Manager, ManagerCareerEntry};

use crate::game::Game;

/// How long a stand-in covers before the club looks outside. A month is long enough for the
/// assistant to be judged on real results and short enough that a stop-gap does not become the
/// manager by default (the bug this fixes).
const INTERIM_WINDOW_DAYS: i64 = 30;

/// After this long the club stops being picky about how well a candidate matches its stature.
/// Three months is a full transfer window and a half: a club that has not found a fitting
/// manager by then would rather have someone than the assistant for ever.
const SETTLE_AFTER_DAYS: i64 = 90;

/// How far a candidate's reputation may sit from the club's before the club keeps waiting.
/// 150 is a quarter of the 200-900 reputation span: a visible step up or down in stature, a
/// better fit than the worst candidate and not so tight that most clubs never find anyone.
const MAX_REPUTATION_GAP: u32 = 150;

fn days_covered(stand_in: &Manager, today: NaiveDate) -> Option<i64> {
    let started = stand_in
        .career_history
        .iter()
        .find(|entry| entry.end_date.is_none())?
        .start_date
        .as_str();
    let started = NaiveDate::parse_from_str(started, "%Y-%m-%d").ok()?;
    Some((today - started).num_days())
}

/// The unemployed manager a club of `club_reputation` would appoint after `days` of cover:
/// the closest match in reputation, the earlier id winning a tie, and nobody outside the gap
/// until the club settles.
fn best_candidate<'a>(
    game: &'a Game,
    club_reputation: u32,
    days: i64,
    user_id: &str,
) -> Option<&'a Manager> {
    let gap = |manager: &Manager| manager.reputation.abs_diff(club_reputation);
    game.managers
        .iter()
        .filter(|m| m.team_id.is_none() && !m.interim && m.id != user_id)
        .filter(|m| days >= SETTLE_AFTER_DAYS || gap(m) <= MAX_REPUTATION_GAP)
        .min_by(|a, b| gap(a).cmp(&gap(b)).then_with(|| a.id.cmp(&b.id)))
}

/// Replaces each AI club's long-serving stand-in with a permanent manager from the unemployed
/// market. Returns how many clubs appointed someone.
pub fn appoint_permanent_managers(game: &mut Game) -> usize {
    let today = game.clock.current_date.date_naive();
    let user_id = if game.manager_id.is_empty() {
        game.manager.id.clone()
    } else {
        game.manager_id.clone()
    };

    let mut clubs: Vec<(String, u32, String, i64)> = game
        .teams
        .iter()
        .filter_map(|team| {
            let stand_in = game
                .managers
                .iter()
                .find(|m| Some(&m.id) == team.manager_id.as_ref() && m.interim)?;
            let days = days_covered(stand_in, today).filter(|d| *d >= INTERIM_WINDOW_DAYS)?;
            Some((team.id.clone(), team.reputation, stand_in.id.clone(), days))
        })
        .collect();
    clubs.sort();

    let mut appointed = 0;
    for (team_id, reputation, stand_in_id, days) in clubs {
        let Some(chosen) = best_candidate(game, reputation, days, &user_id).map(|m| m.id.clone())
        else {
            continue;
        };
        appoint(game, &team_id, &stand_in_id, &chosen);
        appointed += 1;
    }
    appointed
}

/// Hands `team_id` to `chosen` and returns the stand-in to being only an assistant: their
/// manager record is closed and archived.
fn appoint(game: &mut Game, team_id: &str, stand_in_id: &str, chosen_id: &str) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_name = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| team_id.to_string());

    if let Some(position) = game.managers.iter().position(|m| m.id == stand_in_id) {
        let mut stand_in = game.managers.remove(position);
        stand_in.fire(&today);
        game.retired_managers.push(stand_in);
    }
    let Some(chosen) = game.managers.iter_mut().find(|m| m.id == chosen_id) else {
        return;
    };
    chosen.hire(team_id.to_string());
    chosen.satisfaction = crate::ai_hiring::BASE_AI_MANAGER_SATISFACTION as u8;
    chosen.career_history.push(ManagerCareerEntry::open(
        team_id.to_string(),
        team_name.clone(),
        today.clone(),
    ));
    let article = crate::news::managerial_appointment_article(
        &chosen.id,
        &chosen.full_name(),
        team_id,
        &team_name,
        &today,
    );

    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.manager_id = Some(chosen_id.to_string());
    }
    crate::job_offers::expire_outstanding_job_offers_for_team(game, team_id);
    game.news.push(article);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};

    fn game_on(day: &str) -> Game {
        let parts: Vec<u32> = day.split('-').map(|p| p.parse().unwrap()).collect();
        let clock = GameClock::new(
            Utc.with_ymd_and_hms(parts[0] as i32, parts[1], parts[2], 12, 0, 0)
                .unwrap(),
        );
        let user = Manager::new(
            "user".into(),
            "Una".into(),
            "User".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        Game::new(clock, user, vec![], vec![], vec![], vec![])
    }

    fn add_club_covered_since(game: &mut Game, id: &str, reputation: u32, since: &str) {
        let mut club = domain::team::Team::new(
            id.into(),
            format!("{id} FC"),
            id.into(),
            "ENG".into(),
            "City".into(),
            "Ground".into(),
            10_000,
        );
        club.reputation = reputation;
        let mut stand_in = Manager::new(
            format!("mgr_{id}_assistant"),
            "Stand".into(),
            "In".into(),
            "1975-01-01".into(),
            "England".into(),
        );
        stand_in.interim = true;
        stand_in.hire(id.into());
        stand_in.career_history.push(ManagerCareerEntry::open(
            id.into(),
            club.name.clone(),
            since.into(),
        ));
        club.manager_id = Some(stand_in.id.clone());
        game.teams.push(club);
        game.managers.push(stand_in);
    }

    fn add_unemployed(game: &mut Game, id: &str, reputation: u32) {
        let mut manager = Manager::new(
            id.into(),
            "Free".into(),
            id.into(),
            "1975-01-01".into(),
            "England".into(),
        );
        manager.reputation = reputation;
        game.managers.push(manager);
    }

    fn manager_of<'a>(game: &'a Game, club: &str) -> &'a str {
        game.teams
            .iter()
            .find(|t| t.id == club)
            .and_then(|t| t.manager_id.as_deref())
            .unwrap()
    }

    /// Given a club whose assistant has covered for a month and an unemployed manager of matching standing
    /// When the club's appointments are processed
    /// Then the unemployed manager takes the post, with an open career entry and the news,
    ///      and the stand-in leaves the active pool for the archive.
    #[test]
    fn a_long_serving_stand_in_is_replaced_from_the_unemployed_market() {
        let mut game = game_on("2031-08-01");
        add_club_covered_since(&mut game, "c1", 600, "2031-07-01");
        add_unemployed(&mut game, "free1", 620);

        let appointed = appoint_permanent_managers(&mut game);

        assert_eq!(appointed, 1);
        assert_eq!(manager_of(&game, "c1"), "free1");
        let hired = game.managers.iter().find(|m| m.id == "free1").unwrap();
        assert_eq!(hired.team_id.as_deref(), Some("c1"));
        assert_eq!(
            hired.satisfaction,
            crate::ai_hiring::BASE_AI_MANAGER_SATISFACTION as u8
        );
        assert_eq!(hired.career_history.len(), 1);
        assert_eq!(hired.career_history[0].start_date, "2031-08-01");
        assert!(hired.career_history[0].end_date.is_none());
        assert_eq!(game.news.len(), 1);
        assert!(game.managers.iter().all(|m| m.id != "mgr_c1_assistant"));
        let archived = &game.retired_managers[0];
        assert_eq!(archived.id, "mgr_c1_assistant");
        assert_eq!(
            archived.team_id, None,
            "the stand-in no longer holds the post"
        );
        assert_eq!(
            archived.career_history[0].end_date.as_deref(),
            Some("2031-08-01")
        );
    }

    /// Given a club whose stand-in has covered for a month and a fitting unemployed manager
    /// When the club appoints him
    /// Then his new career entry and the appointment news name the club, not its id
    #[test]
    fn an_appointment_records_the_clubs_name() {
        let mut game = game_on("2031-08-01");
        add_club_covered_since(&mut game, "c1", 600, "2031-07-01");
        add_unemployed(&mut game, "free1", 620);

        appoint_permanent_managers(&mut game);

        let hired = game.managers.iter().find(|m| m.id == "free1").unwrap();
        assert_eq!(hired.career_history[0].team_name, "c1 FC");
        assert_eq!(
            game.news[0].i18n_params.get("team").map(String::as_str),
            Some("c1 FC")
        );
    }

    /// Given a stand-in that has covered for under a month
    /// When appointments are processed
    /// Then the stand-in stays.
    #[test]
    fn a_stand_in_keeps_the_post_inside_the_window() {
        let mut game = game_on("2031-07-20");
        add_club_covered_since(&mut game, "c1", 600, "2031-07-01");
        add_unemployed(&mut game, "free1", 600);

        assert_eq!(appoint_permanent_managers(&mut game), 0);
        assert_eq!(manager_of(&game, "c1"), "mgr_c1_assistant");
    }

    /// Given three unemployed managers of different standing
    /// When a club of reputation 700 appoints
    /// Then it takes the closest match, not the most reputable.
    #[test]
    fn the_club_appoints_the_closest_match_in_reputation() {
        let mut game = game_on("2031-08-01");
        add_club_covered_since(&mut game, "c1", 700, "2031-07-01");
        add_unemployed(&mut game, "star", 900);
        // Inside the 150 gap, and more reputable than the match: closest still wins.
        add_unemployed(&mut game, "rival", 820);
        add_unemployed(&mut game, "match", 690);
        add_unemployed(&mut game, "journeyman", 300);

        appoint_permanent_managers(&mut game);

        assert_eq!(manager_of(&game, "c1"), "match");
    }

    /// Given the only candidate is far below the club's standing
    /// When the club has covered for 40 days, then for 100
    /// Then it waits at first and settles for him later.
    #[test]
    fn a_club_waits_for_a_fitting_candidate_and_eventually_settles() {
        let mut game = game_on("2031-08-10");
        add_club_covered_since(&mut game, "c1", 800, "2031-07-01");
        add_unemployed(&mut game, "journeyman", 300);
        assert_eq!(appoint_permanent_managers(&mut game), 0);

        game.clock = GameClock::new(Utc.with_ymd_and_hms(2031, 10, 10, 12, 0, 0).unwrap());
        assert_eq!(appoint_permanent_managers(&mut game), 1);
        assert_eq!(manager_of(&game, "c1"), "journeyman");
    }

    /// Given a club with a permanent manager, however long he has served
    /// When appointments are processed
    /// Then nothing changes.
    #[test]
    fn a_permanent_manager_is_never_replaced() {
        let mut game = game_on("2033-08-01");
        add_club_covered_since(&mut game, "c1", 600, "2031-07-01");
        game.managers[1].interim = false;
        add_unemployed(&mut game, "free1", 600);

        assert_eq!(appoint_permanent_managers(&mut game), 0);
        assert_eq!(manager_of(&game, "c1"), "mgr_c1_assistant");
    }

    /// Given an empty unemployed market
    /// When appointments are processed
    /// Then the stand-in keeps covering rather than the club going vacant.
    #[test]
    fn with_no_candidates_the_stand_in_keeps_covering() {
        let mut game = game_on("2031-08-01");
        add_club_covered_since(&mut game, "c1", 600, "2031-07-01");

        assert_eq!(appoint_permanent_managers(&mut game), 0);
        assert_eq!(manager_of(&game, "c1"), "mgr_c1_assistant");
        assert!(game.retired_managers.is_empty());
    }

    /// Given a stand-in already fired and unemployed, and the user's own manager
    /// When appointments are processed
    /// Then neither is offered to a club as a candidate.
    #[test]
    fn neither_a_stand_in_nor_the_user_is_a_candidate() {
        let mut game = game_on("2031-08-01");
        add_club_covered_since(&mut game, "c1", 600, "2031-07-01");
        game.manager.reputation = 600;
        game.managers[0].reputation = 600;
        let mut sacked_stand_in = Manager::new(
            "sacked".into(),
            "Old".into(),
            "Stand-in".into(),
            "1975-01-01".into(),
            "England".into(),
        );
        sacked_stand_in.interim = true;
        sacked_stand_in.reputation = 600;
        game.managers.push(sacked_stand_in);

        assert_eq!(appoint_permanent_managers(&mut game), 0);
    }
}
