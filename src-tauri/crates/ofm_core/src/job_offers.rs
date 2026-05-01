use crate::game::Game;
use domain::manager::ManagerCareerEntry;
use domain::message::*;
use log::info;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOpportunity {
    pub team_id: String,
    pub team_name: String,
    pub city: String,
    pub reputation: u32,
    pub last_league_position: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobApplicationResult {
    Hired,
    Rejected,
    InvalidTeam,
    AlreadyEmployed,
    SameTeam,
    NotBetterClub,
}

/// Predicate used while the manager is already employed: defines when a target
/// club counts as a "better" job worth surfacing as an offer or application.
/// Strict improvement is the default — any tuning lives here.
fn is_better_club(current_team_reputation: u32, target_team_reputation: u32) -> bool {
    target_team_reputation > current_team_reputation
}

fn opportunity_from(team: &domain::team::Team) -> JobOpportunity {
    JobOpportunity {
        team_id: team.id.clone(),
        team_name: team.name.clone(),
        city: team.city.clone(),
        reputation: team.reputation,
        last_league_position: team.history.last().map(|h| h.league_position),
    }
}

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Shared hiring flow used by both offer-accept and application-accept paths.
pub fn hire_manager(game: &mut Game, team_id: &str, date: &str) -> Result<String, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or_else(|| format!("Team {} not found", team_id))?;
    let team_name = team.name.clone();
    let manager_id = game.manager.id.clone();

    // Assign manager to team
    game.manager.hire(team_id.to_string());
    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.manager_id = Some(manager_id);
    }

    // Create new career history entry
    game.manager.career_history.push(ManagerCareerEntry {
        team_id: team_id.to_string(),
        team_name: team_name.clone(),
        start_date: date.to_string(),
        end_date: None,
        matches: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        best_league_position: None,
    });

    // Reset satisfaction to neutral
    game.manager.satisfaction = 50;

    // Clear job offer timer
    game.days_since_last_job_offer = None;

    // Send welcome message
    let msg = InboxMessage::new(
        format!("job_welcome_{}_{}", team_id, date),
        format!("Welcome to {}", team_name),
        format!(
            "The board of directors at {} is delighted to welcome you as the new manager. \
             We look forward to working with you and achieving great things together.",
            team_name
        ),
        "Board of Directors".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::High)
    .with_sender_role("Chairman")
    .with_i18n(
        "be.msg.jobHired.subject",
        "be.msg.jobHired.body",
        params(&[("team", &team_name)]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");

    game.messages.push(msg);

    info!(
        "[job_offers] Manager {} hired at {} (satisfaction reset to 50)",
        game.manager.full_name(),
        team_name
    );

    Ok(team_name)
}

/// Switches an employed manager from their current club to `new_team_id`.
/// Closes the open career entry, clears the previous team's `manager_id`, then
/// delegates to `hire_manager` for the new appointment. Safe to call when the
/// manager is already employed; reuses `Manager::fire` to keep the
/// "all departures go through the same path" invariant.
pub fn switch_manager_team(
    game: &mut Game,
    new_team_id: &str,
    date: &str,
) -> Result<String, String> {
    let previous_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or_else(|| "Manager has no current team to switch from".to_string())?;

    if previous_team_id == new_team_id {
        return Err(format!(
            "Manager is already employed at {}",
            previous_team_id
        ));
    }

    let previous_team_name = game
        .teams
        .iter_mut()
        .find(|t| t.id == previous_team_id)
        .map(|t| {
            t.manager_id = None;
            t.name.clone()
        })
        .unwrap_or_default();
    game.manager.fire(date);

    info!(
        "[job_offers] Manager {} resigning from {} to take new role",
        game.manager.full_name(),
        previous_team_name
    );

    hire_manager(game, new_team_id, date)
}

/// Single entry point for moving the manager into `new_team_id`, used by both
/// the inbox-accept and active-application paths. Dispatches to
/// `switch_manager_team` when employed and `hire_manager` when not.
fn appoint_manager(game: &mut Game, new_team_id: &str, date: &str) -> Result<String, String> {
    if game.manager.team_id.is_some() {
        switch_manager_team(game, new_team_id, date)
    } else {
        hire_manager(game, new_team_id, date)
    }
}

/// Called daily. Generates passive job offers for the manager — to unemployed
/// managers from any club within the reputation gap, and to employed managers
/// only from clubs that are a step up (per `is_better_club`).
pub fn check_job_offers(game: &mut Game) {
    let mut rng = rand::rng();
    let days = game.days_since_last_job_offer.unwrap_or(0);

    let threshold = if days == 0 {
        if game.days_since_last_job_offer.is_none() {
            game.days_since_last_job_offer = Some(0);
        }
        rng.random_range(1..=3)
    } else {
        rng.random_range(5..=10)
    };

    if days < threshold {
        game.days_since_last_job_offer = Some(days + 1);
        return;
    }

    let candidates = get_offer_candidates(game, &mut rng);
    if let Some(team) = candidates.first() {
        send_job_offer(game, team, &mut rng);
    }

    game.days_since_last_job_offer = Some(0);
}

/// Returns clubs the manager could plausibly take on, applying:
///   - reputation-gap filter (200, widening to 400 if fewer than 2 candidates)
///   - employed-manager filter: must be a "better club" and not the current one
fn find_eligible_clubs(game: &Game) -> Vec<JobOpportunity> {
    let mgr_rep = game.manager.reputation;
    let current = game.manager.team_id.as_ref().and_then(|id| {
        game.teams
            .iter()
            .find(|t| &t.id == id)
            .map(|t| (t.id.clone(), t.reputation))
    });

    let eligible = |t: &domain::team::Team, gap: u32| -> bool {
        let diff = (t.reputation as i32 - mgr_rep as i32).unsigned_abs();
        if diff > gap {
            return false;
        }
        match &current {
            Some((cur_id, cur_rep)) => &t.id != cur_id && is_better_club(*cur_rep, t.reputation),
            None => true,
        }
    };

    let mut clubs: Vec<JobOpportunity> = game
        .teams
        .iter()
        .filter(|t| eligible(t, 200))
        .map(opportunity_from)
        .collect();

    if clubs.len() < 2 {
        clubs = game
            .teams
            .iter()
            .filter(|t| eligible(t, 400))
            .map(opportunity_from)
            .collect();
    }

    clubs
}

fn get_offer_candidates(game: &Game, rng: &mut impl rand::Rng) -> Vec<JobOpportunity> {
    let mut candidates = find_eligible_clubs(game);
    let len = candidates.len();
    if len > 1 {
        for i in (1..len).rev() {
            let j = rng.random_range(0..=(i as u32)) as usize;
            candidates.swap(i, j);
        }
    }
    candidates
}

fn send_job_offer(game: &mut Game, opportunity: &JobOpportunity, _rng: &mut impl rand::Rng) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let msg_id = format!("job_offer_{}_{}", opportunity.team_id, today);

    if game.messages.iter().any(|m| m.id == msg_id) {
        return;
    }

    let pos_label = opportunity
        .last_league_position
        .map(|p| p.to_string())
        .unwrap_or_else(|| "-".to_string());

    let msg = InboxMessage::new(
        msg_id,
        format!("Managerial Vacancy — {}", opportunity.team_name),
        format!(
            "The board at {} ({}) is looking for a new manager to lead the club forward. \
             Last season finish: {}.\n\n\
             After reviewing your credentials, we believe you could be the right fit. \
             Would you be interested in taking on this challenge?",
            opportunity.team_name, opportunity.city, pos_label
        ),
        "Board of Directors".to_string(),
        today.clone(),
    )
    .with_category(MessageCategory::JobOffer)
    .with_priority(MessagePriority::High)
    .with_sender_role("Chairman")
    .with_context(MessageContext {
        team_id: Some(opportunity.team_id.clone()),
        player_id: None,
        fixture_id: None,
        match_result: None,
        scout_report: None,
        delegated_renewal_report: None,
    })
    .with_i18n(
        "be.msg.jobOffer.subject",
        "be.msg.jobOffer.body",
        params(&[
            ("team", &opportunity.team_name),
            ("city", &opportunity.city),
            ("league_position", &pos_label),
        ]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
    .with_action(MessageAction {
        id: format!("respond_{}", opportunity.team_id),
        label: "Respond".to_string(),
        action_type: ActionType::ChooseOption {
            options: vec![
                ActionOption {
                    id: "accept".to_string(),
                    label: "Accept the position".to_string(),
                    description: format!("Join {} as their new manager", opportunity.team_name),
                    label_key: Some("be.msg.jobOffer.accept".to_string()),
                    description_key: None,
                },
                ActionOption {
                    id: "decline".to_string(),
                    label: "Decline the offer".to_string(),
                    description: "Continue looking for other opportunities".to_string(),
                    label_key: Some("be.msg.jobOffer.decline".to_string()),
                    description_key: None,
                },
            ],
        },
        resolved: false,
        label_key: None,
    });

    info!(
        "[job_offers] Sent offer from {} to {} (rep: {} vs {})",
        opportunity.team_name,
        game.manager.full_name(),
        opportunity.reputation,
        game.manager.reputation
    );

    game.messages.push(msg);
}

/// Returns up to 4 job opportunities suitable for the manager. For an
/// unemployed manager, every club within the reputation gap is listed. For an
/// employed manager, only clubs that count as a step up (per `is_better_club`)
/// and that are not the current club are listed.
pub fn get_available_jobs(game: &Game) -> Vec<JobOpportunity> {
    let mut jobs = find_eligible_clubs(game);
    jobs.sort_by(|a, b| b.reputation.cmp(&a.reputation));
    jobs.truncate(4);
    jobs
}

/// Active application by the manager for a specific team's job. Works for both
/// unemployed managers (any team within the rep gap) and employed managers
/// (only "better" clubs per `is_better_club`, hire path goes through
/// `switch_manager_team`).
pub fn apply_for_job(game: &mut Game, team_id: &str) -> JobApplicationResult {
    let team = match game.teams.iter().find(|t| t.id == team_id) {
        Some(t) => t,
        None => return JobApplicationResult::InvalidTeam,
    };

    let team_rep = team.reputation;
    let mgr_rep = game.manager.reputation;
    let team_name = team.name.clone();

    let current_team = game.manager.team_id.as_ref().and_then(|id| {
        game.teams
            .iter()
            .find(|t| &t.id == id)
            .map(|t| (t.id.clone(), t.reputation))
    });

    if let Some((cur_id, cur_rep)) = &current_team {
        if cur_id == team_id {
            return JobApplicationResult::SameTeam;
        }
        if !is_better_club(*cur_rep, team_rep) {
            return JobApplicationResult::NotBetterClub;
        }
    }

    let gap = team_rep.saturating_sub(mgr_rep);

    let success_pct = if gap == 0 {
        90
    } else if gap <= 100 {
        70
    } else if gap <= 200 {
        50
    } else if gap <= 300 {
        30
    } else {
        10
    };

    let mut rng = rand::rng();
    let roll = rng.random_range(1..=100);

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    if roll <= success_pct {
        match appoint_manager(game, team_id, &today) {
            Ok(_) => {
                info!(
                    "[job_offers] Application accepted: {} at {} (gap={}, roll={}/{})",
                    game.manager.full_name(),
                    team_name,
                    gap,
                    roll,
                    success_pct
                );
                JobApplicationResult::Hired
            }
            Err(_) => JobApplicationResult::InvalidTeam,
        }
    } else {
        let msg = InboxMessage::new(
            format!("job_rejection_{}_{}", team_id, today),
            format!("Application Update — {}", team_name),
            format!(
                "Thank you for your interest in the managerial position at {}. \
                 After careful consideration, we have decided to pursue other candidates. \
                 We wish you the best in your future career.",
                team_name
            ),
            "Board of Directors".to_string(),
            today,
        )
        .with_category(MessageCategory::JobOffer)
        .with_priority(MessagePriority::Normal)
        .with_sender_role("Chairman")
        .with_i18n(
            "be.msg.jobRejection.subject",
            "be.msg.jobRejection.body",
            params(&[("team", &team_name)]),
        )
        .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");

        game.messages.push(msg);

        info!(
            "[job_offers] Application rejected: {} at {} (gap={}, roll={}/{})",
            game.manager.full_name(),
            team_name,
            gap,
            roll,
            success_pct
        );
        JobApplicationResult::Rejected
    }
}

/// Handles accept/decline response to an inbox job offer message.
pub fn apply_job_offer_response(
    game: &mut Game,
    message_id: &str,
    action_id: &str,
    option_id: &str,
) -> Option<String> {
    if !message_id.starts_with("job_offer_") {
        return None;
    }

    let team_id = game
        .messages
        .iter()
        .find(|m| m.id == message_id)
        .and_then(|m| m.context.team_id.clone())?;

    let team_name = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id)
        && let Some(action) = msg.actions.iter_mut().find(|a| a.id == action_id)
    {
        action.resolved = true;
    }

    match option_id {
        "accept" => {
            // Defensive guard: an inbox offer may have been targeted at the
            // manager's current club (e.g. via stale state). Decline silently
            // rather than create a no-op career entry.
            if game.manager.team_id.as_deref() == Some(team_id.as_str()) {
                return Some(format!("You are already the manager of {}", team_name));
            }
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            match appoint_manager(game, &team_id, &today) {
                Ok(name) => Some(format!("You have been appointed manager of {}", name)),
                Err(e) => Some(format!("Failed to accept position: {}", e)),
            }
        }
        "decline" => Some(format!("You declined the offer from {}", team_name)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;

    fn make_game(satisfaction: u8, has_team: bool) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 11, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.reputation = 500;
        manager.satisfaction = satisfaction;
        if has_team {
            manager.hire("team1".to_string());
        }

        let mut team1 = Team::new(
            "team1".to_string(),
            "Old FC".to_string(),
            "OLD".to_string(),
            "England".to_string(),
            "Oldville".to_string(),
            "Old Ground".to_string(),
            20_000,
        );
        team1.reputation = 500;
        if has_team {
            team1.manager_id = Some("mgr1".to_string());
        }

        let mut team2 = Team::new(
            "team2".to_string(),
            "New FC".to_string(),
            "NEW".to_string(),
            "England".to_string(),
            "Newville".to_string(),
            "New Ground".to_string(),
            25_000,
        );
        team2.reputation = 450;

        let mut team3 = Team::new(
            "team3".to_string(),
            "Elite FC".to_string(),
            "ELT".to_string(),
            "England".to_string(),
            "Elitetown".to_string(),
            "Elite Arena".to_string(),
            40_000,
        );
        team3.reputation = 800;

        Game::new(
            clock,
            manager,
            vec![team1, team2, team3],
            vec![],
            vec![],
            vec![],
        )
    }

    #[test]
    fn hire_manager_sets_team_id_and_manager_id() {
        let mut game = make_game(10, false);
        let result = hire_manager(&mut game, "team2", "2026-11-01");
        assert!(result.is_ok());
        assert_eq!(game.manager.team_id, Some("team2".to_string()));
        assert_eq!(
            game.teams
                .iter()
                .find(|t| t.id == "team2")
                .unwrap()
                .manager_id,
            Some("mgr1".to_string())
        );
    }

    #[test]
    fn hire_manager_creates_career_entry() {
        let mut game = make_game(10, false);
        hire_manager(&mut game, "team2", "2026-11-01").unwrap();
        let entry = game.manager.career_history.last().unwrap();
        assert_eq!(entry.team_id, "team2");
        assert_eq!(entry.team_name, "New FC");
        assert_eq!(entry.start_date, "2026-11-01");
        assert!(entry.end_date.is_none());
        assert_eq!(entry.matches, 0);
        assert_eq!(entry.wins, 0);
    }

    #[test]
    fn hire_manager_resets_satisfaction_to_50() {
        let mut game = make_game(10, false);
        hire_manager(&mut game, "team2", "2026-11-01").unwrap();
        assert_eq!(game.manager.satisfaction, 50);
    }

    #[test]
    fn hire_manager_clears_job_offer_timer() {
        let mut game = make_game(10, false);
        game.days_since_last_job_offer = Some(5);
        hire_manager(&mut game, "team2", "2026-11-01").unwrap();
        assert!(game.days_since_last_job_offer.is_none());
    }

    #[test]
    fn hire_manager_sends_welcome_message() {
        let mut game = make_game(10, false);
        hire_manager(&mut game, "team2", "2026-11-01").unwrap();
        assert!(
            game.messages
                .iter()
                .any(|m| m.id.starts_with("job_welcome_"))
        );
    }

    #[test]
    fn hire_manager_invalid_team_returns_error() {
        let mut game = make_game(10, false);
        let result = hire_manager(&mut game, "nonexistent", "2026-11-01");
        assert!(result.is_err());
    }

    #[test]
    fn check_job_offers_when_employed_initializes_timer() {
        let mut game = make_game(50, true);
        check_job_offers(&mut game);
        // Timer is now started for employed managers too — opportunities can
        // arrive while in post (e.g. headhunting from a bigger club).
        assert!(game.days_since_last_job_offer.is_some());
    }

    #[test]
    fn check_job_offers_initializes_timer_when_unemployed() {
        let mut game = make_game(10, false);
        game.days_since_last_job_offer = None;
        check_job_offers(&mut game);
        assert!(game.days_since_last_job_offer.is_some());
    }

    #[test]
    fn get_available_jobs_when_employed_returns_better_clubs_only() {
        // make_game(_, true) employs the manager at team1 (rep 500).
        // team2 has rep 450 (worse) → must not appear.
        // team3 has rep 800 (better) → must appear (within widened gap).
        let game = make_game(50, true);
        let jobs = get_available_jobs(&game);
        assert!(jobs.iter().any(|j| j.team_id == "team3"));
        assert!(!jobs.iter().any(|j| j.team_id == "team1"));
        assert!(!jobs.iter().any(|j| j.team_id == "team2"));
    }

    #[test]
    fn get_available_jobs_filters_by_reputation() {
        let game = make_game(10, false);
        let jobs = get_available_jobs(&game);
        assert!(jobs.iter().any(|j| j.team_id == "team1"));
        assert!(jobs.iter().any(|j| j.team_id == "team2"));
        assert!(!jobs.iter().any(|j| j.team_id == "team3"));
    }

    #[test]
    fn get_available_jobs_capped_at_4() {
        let mut game = make_game(10, false);
        for i in 4..=10 {
            let mut t = Team::new(
                format!("team{}", i),
                format!("Team {}", i),
                format!("T{}", i),
                "England".to_string(),
                format!("City{}", i),
                format!("Ground{}", i),
                10_000,
            );
            t.reputation = 480;
            game.teams.push(t);
        }
        let jobs = get_available_jobs(&game);
        assert!(jobs.len() <= 4);
    }

    #[test]
    fn apply_for_job_when_employed_at_worse_club_returns_not_better() {
        // team2 (rep 450) is worse than current team1 (rep 500) — can't apply.
        let mut game = make_game(50, true);
        let result = apply_for_job(&mut game, "team2");
        assert_eq!(result, JobApplicationResult::NotBetterClub);
        // Manager still at team1; no career change.
        assert_eq!(game.manager.team_id, Some("team1".to_string()));
    }

    #[test]
    fn apply_for_job_when_employed_at_same_club_returns_same_team() {
        let mut game = make_game(50, true);
        let result = apply_for_job(&mut game, "team1");
        assert_eq!(result, JobApplicationResult::SameTeam);
        assert_eq!(game.manager.team_id, Some("team1".to_string()));
    }

    #[test]
    fn apply_for_job_invalid_team_returns_invalid() {
        let mut game = make_game(10, false);
        let result = apply_for_job(&mut game, "nonexistent");
        assert_eq!(result, JobApplicationResult::InvalidTeam);
    }

    #[test]
    fn apply_job_offer_response_accept_hires_manager() {
        let mut game = make_game(10, false);
        let msg = InboxMessage::new(
            "job_offer_team2_2026-11-01".to_string(),
            "Offer".to_string(),
            "Join us".to_string(),
            "Board".to_string(),
            "2026-11-01".to_string(),
        )
        .with_context(MessageContext {
            team_id: Some("team2".to_string()),
            player_id: None,
            fixture_id: None,
            match_result: None,
            scout_report: None,
            delegated_renewal_report: None,
        })
        .with_action(MessageAction {
            id: "respond_team2".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption {
                options: vec![
                    ActionOption {
                        id: "accept".to_string(),
                        label: "Accept".to_string(),
                        description: String::new(),
                        label_key: None,
                        description_key: None,
                    },
                    ActionOption {
                        id: "decline".to_string(),
                        label: "Decline".to_string(),
                        description: String::new(),
                        label_key: None,
                        description_key: None,
                    },
                ],
            },
            resolved: false,
            label_key: None,
        });
        game.messages.push(msg);

        let effect = apply_job_offer_response(
            &mut game,
            "job_offer_team2_2026-11-01",
            "respond_team2",
            "accept",
        );
        assert!(effect.is_some());
        assert!(effect.unwrap().contains("New FC"));
        assert_eq!(game.manager.team_id, Some("team2".to_string()));
        assert_eq!(game.manager.satisfaction, 50);
    }

    #[test]
    fn apply_job_offer_response_decline_no_state_change() {
        let mut game = make_game(10, false);
        let msg = InboxMessage::new(
            "job_offer_team2_2026-11-01".to_string(),
            "Offer".to_string(),
            "Join us".to_string(),
            "Board".to_string(),
            "2026-11-01".to_string(),
        )
        .with_context(MessageContext {
            team_id: Some("team2".to_string()),
            player_id: None,
            fixture_id: None,
            match_result: None,
            scout_report: None,
            delegated_renewal_report: None,
        })
        .with_action(MessageAction {
            id: "respond_team2".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption { options: vec![] },
            resolved: false,
            label_key: None,
        });
        game.messages.push(msg);

        let effect = apply_job_offer_response(
            &mut game,
            "job_offer_team2_2026-11-01",
            "respond_team2",
            "decline",
        );
        assert!(effect.is_some());
        assert!(effect.unwrap().contains("declined"));
        assert!(game.manager.team_id.is_none());
    }

    #[test]
    fn apply_job_offer_response_ignores_non_job_messages() {
        let mut game = make_game(10, false);
        let result = apply_job_offer_response(&mut game, "sponsor_123", "action1", "accept");
        assert!(result.is_none());
    }

    #[test]
    fn apply_job_offer_response_accept_when_employed_switches_teams() {
        // Manager employed at team1; receives offer from team3 (better club).
        // Inbox accept always succeeds (formal invitation).
        let mut game = make_game(50, true);
        // Seed an open career entry at the current team so we can verify it closes.
        game.manager.career_history.push(ManagerCareerEntry {
            team_id: "team1".to_string(),
            team_name: "Old FC".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: None,
            matches: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            best_league_position: None,
        });

        let msg = InboxMessage::new(
            "job_offer_team3_2026-11-01".to_string(),
            "Offer".to_string(),
            "Join us".to_string(),
            "Board".to_string(),
            "2026-11-01".to_string(),
        )
        .with_context(MessageContext {
            team_id: Some("team3".to_string()),
            player_id: None,
            fixture_id: None,
            match_result: None,
            scout_report: None,
            delegated_renewal_report: None,
        })
        .with_action(MessageAction {
            id: "respond_team3".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption { options: vec![] },
            resolved: false,
            label_key: None,
        });
        game.messages.push(msg);

        let effect = apply_job_offer_response(
            &mut game,
            "job_offer_team3_2026-11-01",
            "respond_team3",
            "accept",
        );
        assert!(effect.is_some());
        assert!(effect.unwrap().contains("Elite FC"));
        // Manager moved to the new club.
        assert_eq!(game.manager.team_id, Some("team3".to_string()));
        // Old team's manager_id cleared.
        assert!(
            game.teams
                .iter()
                .find(|t| t.id == "team1")
                .unwrap()
                .manager_id
                .is_none()
        );
        // New team's manager_id set.
        assert_eq!(
            game.teams
                .iter()
                .find(|t| t.id == "team3")
                .unwrap()
                .manager_id,
            Some("mgr1".to_string())
        );
        // Old career entry closed; new career entry opened and still open.
        assert_eq!(game.manager.career_history.len(), 2);
        let old = &game.manager.career_history[0];
        let new = &game.manager.career_history[1];
        assert_eq!(old.team_id, "team1");
        assert_eq!(old.end_date.as_deref(), Some("2026-11-01"));
        assert_eq!(new.team_id, "team3");
        assert!(new.end_date.is_none());
        // Satisfaction reset on rehire.
        assert_eq!(game.manager.satisfaction, 50);
    }

    #[test]
    fn apply_job_offer_response_accept_rejects_offer_for_current_club() {
        // Defensive: a stale offer pointing at the manager's own club must not
        // create a no-op career entry.
        let mut game = make_game(50, true);
        let msg = InboxMessage::new(
            "job_offer_team1_2026-11-01".to_string(),
            "Offer".to_string(),
            "Stay with us".to_string(),
            "Board".to_string(),
            "2026-11-01".to_string(),
        )
        .with_context(MessageContext {
            team_id: Some("team1".to_string()),
            player_id: None,
            fixture_id: None,
            match_result: None,
            scout_report: None,
            delegated_renewal_report: None,
        })
        .with_action(MessageAction {
            id: "respond_team1".to_string(),
            label: "Respond".to_string(),
            action_type: ActionType::ChooseOption { options: vec![] },
            resolved: false,
            label_key: None,
        });
        game.messages.push(msg);

        let effect = apply_job_offer_response(
            &mut game,
            "job_offer_team1_2026-11-01",
            "respond_team1",
            "accept",
        );
        assert!(effect.is_some());
        assert!(effect.unwrap().contains("already the manager"));
        // No state change.
        assert_eq!(game.manager.team_id, Some("team1".to_string()));
        assert_eq!(game.manager.career_history.len(), 0);
    }

    #[test]
    fn switch_manager_team_closes_old_career_entry_and_opens_new_one() {
        let mut game = make_game(50, true);
        game.manager.career_history.push(ManagerCareerEntry {
            team_id: "team1".to_string(),
            team_name: "Old FC".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: None,
            matches: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            best_league_position: None,
        });

        let result = switch_manager_team(&mut game, "team3", "2026-11-01");
        assert!(result.is_ok());
        assert_eq!(game.manager.career_history.len(), 2);
        assert_eq!(
            game.manager.career_history[0].end_date.as_deref(),
            Some("2026-11-01")
        );
        assert!(game.manager.career_history[1].end_date.is_none());
    }

    #[test]
    fn switch_manager_team_clears_old_team_manager_id() {
        let mut game = make_game(50, true);
        switch_manager_team(&mut game, "team3", "2026-11-01").unwrap();
        let old = game.teams.iter().find(|t| t.id == "team1").unwrap();
        assert!(old.manager_id.is_none());
    }

    #[test]
    fn switch_manager_team_sets_new_team_manager_id_and_team_id() {
        let mut game = make_game(50, true);
        switch_manager_team(&mut game, "team3", "2026-11-01").unwrap();
        assert_eq!(game.manager.team_id, Some("team3".to_string()));
        let new = game.teams.iter().find(|t| t.id == "team3").unwrap();
        assert_eq!(new.manager_id, Some("mgr1".to_string()));
    }

    #[test]
    fn switch_manager_team_resets_satisfaction_and_warnings() {
        let mut game = make_game(20, true);
        game.manager.warning_stage = 2;
        switch_manager_team(&mut game, "team3", "2026-11-01").unwrap();
        assert_eq!(game.manager.satisfaction, 50);
        assert_eq!(game.manager.warning_stage, 0);
    }

    #[test]
    fn switch_manager_team_errors_when_unemployed() {
        let mut game = make_game(10, false);
        let result = switch_manager_team(&mut game, "team3", "2026-11-01");
        assert!(result.is_err());
    }

    #[test]
    fn switch_manager_team_errors_for_same_team() {
        let mut game = make_game(50, true);
        let result = switch_manager_team(&mut game, "team1", "2026-11-01");
        assert!(result.is_err());
    }

    #[test]
    fn check_job_offers_when_employed_generates_offer_from_better_club() {
        // make_game(_, true): manager rep 500, employed at team1 (rep 500),
        // team2 (rep 450, worse), team3 (rep 800, better).
        // Set days past the upper threshold to bypass the "not yet" branch.
        let mut game = make_game(50, true);
        game.days_since_last_job_offer = Some(20);

        check_job_offers(&mut game);

        let offers: Vec<_> = game
            .messages
            .iter()
            .filter(|m| m.id.starts_with("job_offer_"))
            .collect();
        assert_eq!(offers.len(), 1);
        // Only "better" club is team3 — never team1 (current) or team2 (worse).
        assert_eq!(offers[0].context.team_id.as_deref(), Some("team3"));
        // Timer reset after sending an offer.
        assert_eq!(game.days_since_last_job_offer, Some(0));
    }

    #[test]
    fn check_job_offers_when_employed_no_better_clubs_does_not_offer() {
        // Make every club worse than the current one.
        let mut game = make_game(50, true);
        game.days_since_last_job_offer = Some(20);
        for team in &mut game.teams {
            if team.id != "team1" {
                team.reputation = 100;
            }
        }

        check_job_offers(&mut game);

        let offers: Vec<_> = game
            .messages
            .iter()
            .filter(|m| m.id.starts_with("job_offer_"))
            .collect();
        assert!(offers.is_empty());
    }

    #[test]
    fn appoint_manager_dispatches_to_hire_when_unemployed() {
        let mut game = make_game(10, false);
        appoint_manager(&mut game, "team2", "2026-11-01").unwrap();
        assert_eq!(game.manager.team_id, Some("team2".to_string()));
        assert_eq!(game.manager.career_history.len(), 1);
        assert!(game.manager.career_history[0].end_date.is_none());
    }

    #[test]
    fn appoint_manager_dispatches_to_switch_when_employed() {
        let mut game = make_game(50, true);
        game.manager.career_history.push(ManagerCareerEntry {
            team_id: "team1".to_string(),
            team_name: "Old FC".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: None,
            matches: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            best_league_position: None,
        });

        appoint_manager(&mut game, "team3", "2026-11-01").unwrap();

        assert_eq!(game.manager.team_id, Some("team3".to_string()));
        // Old entry closed AND new entry opened — proof the switch path ran.
        assert_eq!(game.manager.career_history.len(), 2);
        assert_eq!(
            game.manager.career_history[0].end_date.as_deref(),
            Some("2026-11-01")
        );
        assert!(game.manager.career_history[1].end_date.is_none());
    }

    #[test]
    fn job_application_result_serializes_to_snake_case() {
        // Locks in the serde rename so the frontend wire format stays stable.
        assert_eq!(
            serde_json::to_string(&JobApplicationResult::Hired).unwrap(),
            "\"hired\""
        );
        assert_eq!(
            serde_json::to_string(&JobApplicationResult::SameTeam).unwrap(),
            "\"same_team\""
        );
        assert_eq!(
            serde_json::to_string(&JobApplicationResult::NotBetterClub).unwrap(),
            "\"not_better_club\""
        );
        assert_eq!(
            serde_json::to_string(&JobApplicationResult::AlreadyEmployed).unwrap(),
            "\"already_employed\""
        );
    }
}
