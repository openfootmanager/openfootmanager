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
pub enum JobApplicationResult {
    Hired,
    Rejected,
    InvalidTeam,
    AlreadyEmployed,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobOfferResponseEffect {
    pub message: String,
    pub i18n_key: String,
    pub i18n_params: HashMap<String, String>,
}

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn response_effect(message: String, i18n_key: &str, team_name: &str) -> JobOfferResponseEffect {
    JobOfferResponseEffect {
        message,
        i18n_key: i18n_key.to_string(),
        i18n_params: params(&[("team", team_name)]),
    }
}

pub(crate) fn expire_outstanding_job_offers_for_team(game: &mut Game, team_id: &str) {
    let team_name = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.to_string());

    for message in game.messages.iter_mut().filter(|message| {
        message.id.starts_with("job_offer_") && message.context.team_id.as_deref() == Some(team_id)
    }) {
        message.read = true;
        message.subject = format!("Position Filled — {}", team_name);
        message.body = format!(
            "The vacancy at {} has now been filled, so this offer is no longer available.",
            team_name
        );
        message.subject_key = Some("be.msg.jobOfferExpired.subject".to_string());
        message.body_key = Some("be.msg.jobOfferExpired.body".to_string());
        message.i18n_params = params(&[("team", &team_name)]);
        for action in &mut message.actions {
            action.resolved = true;
        }
    }
}

/// Shared hiring flow used by both offer-accept and application-accept paths.
pub fn hire_manager(game: &mut Game, team_id: &str, date: &str) -> Result<String, String> {
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or_else(|| format!("Team {} not found", team_id))?;
    if team.manager_id.is_some() {
        return Err(format!("Team {} is not vacant", team_id));
    }
    let team_name = team.name.clone();
    let manager_id = game.manager.id.clone();
    let manager_name = game.manager.full_name();

    // Assign manager to team
    game.manager.hire(team_id.to_string());
    if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
        team.manager_id = Some(manager_id.clone());
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
    game.sync_user_manager_record();
    game.vacant_team_days.remove(team_id);
    expire_outstanding_job_offers_for_team(game, team_id);

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
    game.news.push(crate::news::managerial_appointment_article(
        &manager_id,
        &manager_name,
        team_id,
        &team_name,
        date,
    ));

    info!(
        "[job_offers] Manager {} hired at {} (satisfaction reset to 50)",
        game.manager.full_name(),
        team_name
    );

    Ok(team_name)
}

/// Called daily. Generates passive job offers for unemployed managers.
pub fn check_job_offers(game: &mut Game) {
    if game.manager.team_id.is_some() {
        return;
    }

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

fn get_offer_candidates(game: &Game, rng: &mut impl rand::Rng) -> Vec<JobOpportunity> {
    let mgr_rep = game.manager.reputation;
    let mut candidates: Vec<JobOpportunity> = game
        .teams
        .iter()
        .filter(|t| {
            let diff = (t.reputation as i32 - mgr_rep as i32).unsigned_abs();
            t.manager_id.is_none() && diff <= 200
        })
        .map(|t| JobOpportunity {
            team_id: t.id.clone(),
            team_name: t.name.clone(),
            city: t.city.clone(),
            reputation: t.reputation,
            last_league_position: t.history.last().map(|h| h.league_position),
        })
        .collect();

    if candidates.len() < 2 {
        candidates = game
            .teams
            .iter()
            .filter(|t| {
                let diff = (t.reputation as i32 - mgr_rep as i32).unsigned_abs();
                t.manager_id.is_none() && diff <= 400
            })
            .map(|t| JobOpportunity {
                team_id: t.id.clone(),
                team_name: t.name.clone(),
                city: t.city.clone(),
                reputation: t.reputation,
                last_league_position: t.history.last().map(|h| h.league_position),
            })
            .collect();
    }

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
        youth_target_position: None,
        youth_search_region: None,
        youth_search_objective: None,
        youth_prospects: None,
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

/// Returns up to 4 job opportunities suitable for the unemployed manager.
pub fn get_available_jobs(game: &Game) -> Vec<JobOpportunity> {
    if game.manager.team_id.is_some() {
        return vec![];
    }

    let mgr_rep = game.manager.reputation;
    let mut jobs: Vec<JobOpportunity> = game
        .teams
        .iter()
        .filter(|t| {
            let diff = (t.reputation as i32 - mgr_rep as i32).unsigned_abs();
            t.manager_id.is_none() && diff <= 200
        })
        .map(|t| JobOpportunity {
            team_id: t.id.clone(),
            team_name: t.name.clone(),
            city: t.city.clone(),
            reputation: t.reputation,
            last_league_position: t.history.last().map(|h| h.league_position),
        })
        .collect();

    if jobs.len() < 2 {
        jobs = game
            .teams
            .iter()
            .filter(|t| {
                let diff = (t.reputation as i32 - mgr_rep as i32).unsigned_abs();
                t.manager_id.is_none() && diff <= 400
            })
            .map(|t| JobOpportunity {
                team_id: t.id.clone(),
                team_name: t.name.clone(),
                city: t.city.clone(),
                reputation: t.reputation,
                last_league_position: t.history.last().map(|h| h.league_position),
            })
            .collect();
    }

    jobs.sort_by(|a, b| b.reputation.cmp(&a.reputation));
    jobs.truncate(4);
    jobs
}

/// Active application by the manager for a specific team's job.
pub fn apply_for_job(game: &mut Game, team_id: &str) -> JobApplicationResult {
    if game.manager.team_id.is_some() {
        return JobApplicationResult::AlreadyEmployed;
    }

    let team = match game.teams.iter().find(|t| t.id == team_id) {
        Some(t) if t.manager_id.is_none() => t,
        None => return JobApplicationResult::InvalidTeam,
        Some(_) => return JobApplicationResult::InvalidTeam,
    };

    let team_rep = team.reputation;
    let mgr_rep = game.manager.reputation;
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
        let team_name = team.name.clone();
        match hire_manager(game, team_id, &today) {
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
        let team_name = team.name.clone();
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
) -> Option<JobOfferResponseEffect> {
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
            // Guard against accepting an old offer after being (re)hired elsewhere.
            // Without this, the stale "accept" would leave the previous club's
            // manager_id set and its career entry open.
            if game.manager.team_id.is_some() {
                return Some(response_effect(
                    format!(
                        "You are already employed and cannot accept the offer from {}",
                        team_name
                    ),
                    "be.msg.jobOffer.effects.alreadyEmployed",
                    &team_name,
                ));
            }
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            match hire_manager(game, &team_id, &today) {
                Ok(name) => Some(response_effect(
                    format!("You have been appointed manager of {}", name),
                    "be.msg.jobOffer.effects.accepted",
                    &name,
                )),
                Err(e) if e.contains("is not vacant") => Some(response_effect(
                    format!(
                        "The offer from {} is no longer available because the position has been filled",
                        team_name
                    ),
                    "be.msg.jobOffer.effects.unavailable",
                    &team_name,
                )),
                Err(_) => Some(response_effect(
                    format!("Could not accept the offer from {}", team_name),
                    "be.msg.jobOffer.effects.failed",
                    &team_name,
                )),
            }
        }
        "decline" => Some(response_effect(
            format!("You declined the offer from {}", team_name),
            "be.msg.jobOffer.effects.declined",
            &team_name,
        )),
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
    fn hire_manager_syncs_user_manager_record() {
        let mut game = make_game(10, false);

        hire_manager(&mut game, "team2", "2026-11-01").unwrap();

        let stored_manager = game
            .managers
            .iter()
            .find(|manager| manager.id == "mgr1")
            .unwrap();
        assert_eq!(stored_manager.team_id.as_deref(), Some("team2"));
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
    fn hire_manager_creates_managerial_appointment_news_article() {
        let mut game = make_game(10, false);

        hire_manager(&mut game, "team2", "2026-11-01").unwrap();

        assert!(game.news.iter().any(|article| {
            article.category == domain::news::NewsCategory::ManagerialChange
                && article.team_ids.contains(&"team2".to_string())
                && article.headline_key.as_deref() == Some("be.news.managerialAppointment.headline")
                && article.body_key.as_deref() == Some("be.news.managerialAppointment.body")
        }));
    }

    #[test]
    fn hire_manager_resolves_outstanding_offer_for_team() {
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
            youth_target_position: None,
            youth_search_region: None,
            youth_search_objective: None,
            youth_prospects: None,
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

        hire_manager(&mut game, "team2", "2026-11-01").unwrap();

        let offer = game
            .messages
            .iter()
            .find(|message| message.id == "job_offer_team2_2026-11-01")
            .unwrap();
        assert!(offer.read);
        assert!(offer.actions.iter().all(|action| action.resolved));
        assert_eq!(
            offer.subject_key.as_deref(),
            Some("be.msg.jobOfferExpired.subject")
        );
        assert_eq!(
            offer.body_key.as_deref(),
            Some("be.msg.jobOfferExpired.body")
        );
        assert!(offer.subject.contains("Position Filled"));
        assert!(offer.body.contains("no longer available"));
    }

    #[test]
    fn hire_manager_invalid_team_returns_error() {
        let mut game = make_game(10, false);
        let result = hire_manager(&mut game, "nonexistent", "2026-11-01");
        assert!(result.is_err());
    }

    #[test]
    fn check_job_offers_no_op_when_employed() {
        let mut game = make_game(50, true);
        check_job_offers(&mut game);
        assert!(game.days_since_last_job_offer.is_none());
        assert!(game.messages.is_empty());
    }

    #[test]
    fn check_job_offers_initializes_timer_when_unemployed() {
        let mut game = make_game(10, false);
        game.days_since_last_job_offer = None;
        check_job_offers(&mut game);
        assert!(game.days_since_last_job_offer.is_some());
    }

    #[test]
    fn get_available_jobs_returns_empty_when_employed() {
        let game = make_game(50, true);
        let jobs = get_available_jobs(&game);
        assert!(jobs.is_empty());
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
    fn get_available_jobs_only_returns_vacant_clubs() {
        let mut game = make_game(10, false);
        game.teams
            .iter_mut()
            .find(|team| team.id == "team2")
            .unwrap()
            .manager_id = Some("mgr-ai".to_string());

        let jobs = get_available_jobs(&game);

        assert!(jobs.iter().any(|job| job.team_id == "team1"));
        assert!(!jobs.iter().any(|job| job.team_id == "team2"));
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
    fn apply_for_job_when_employed_returns_already_employed() {
        let mut game = make_game(50, true);
        let result = apply_for_job(&mut game, "team2");
        assert_eq!(result, JobApplicationResult::AlreadyEmployed);
    }

    #[test]
    fn apply_for_job_invalid_team_returns_invalid() {
        let mut game = make_game(10, false);
        let result = apply_for_job(&mut game, "nonexistent");
        assert_eq!(result, JobApplicationResult::InvalidTeam);
    }

    #[test]
    fn apply_for_job_occupied_team_returns_invalid() {
        let mut game = make_game(10, false);
        game.teams
            .iter_mut()
            .find(|team| team.id == "team2")
            .unwrap()
            .manager_id = Some("mgr-ai".to_string());

        let result = apply_for_job(&mut game, "team2");

        assert_eq!(result, JobApplicationResult::InvalidTeam);
        assert!(
            game.messages.is_empty(),
            "occupied clubs should not generate application responses"
        );
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
            youth_target_position: None,
            youth_search_region: None,
            youth_search_objective: None,
            youth_prospects: None,
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
        let effect = effect.expect("effect");
        assert!(effect.message.contains("New FC"));
        assert_eq!(effect.i18n_key, "be.msg.jobOffer.effects.accepted");
        assert_eq!(
            effect.i18n_params.get("team").map(String::as_str),
            Some("New FC")
        );
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
            youth_target_position: None,
            youth_search_region: None,
            youth_search_objective: None,
            youth_prospects: None,
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
        let effect = effect.expect("effect");
        assert!(effect.message.contains("declined"));
        assert_eq!(effect.i18n_key, "be.msg.jobOffer.effects.declined");
        assert_eq!(
            effect.i18n_params.get("team").map(String::as_str),
            Some("New FC")
        );
        assert!(game.manager.team_id.is_none());
    }

    #[test]
    fn apply_job_offer_response_ignores_non_job_messages() {
        let mut game = make_game(10, false);
        let result = apply_job_offer_response(&mut game, "sponsor_123", "action1", "accept");
        assert!(result.is_none());
    }

    #[test]
    fn apply_job_offer_response_accept_rejected_when_already_employed() {
        let mut game = make_game(50, true);
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
            youth_target_position: None,
            youth_search_region: None,
            youth_search_objective: None,
            youth_prospects: None,
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
            "accept",
        );
        let effect = effect.expect("effect");
        assert!(effect.message.contains("already employed"));
        assert_eq!(effect.i18n_key, "be.msg.jobOffer.effects.alreadyEmployed");
        assert_eq!(
            effect.i18n_params.get("team").map(String::as_str),
            Some("New FC")
        );
        // Previous team assignment must be untouched.
        assert_eq!(game.manager.team_id, Some("team1".to_string()));
        assert_eq!(game.teams[0].manager_id, Some("mgr1".to_string()));
        // No new career entry opened.
        assert_eq!(game.manager.career_history.len(), 0);
    }

    #[test]
    fn apply_job_offer_response_accept_returns_unavailable_effect_when_team_filled() {
        let mut game = make_game(10, false);
        game.teams
            .iter_mut()
            .find(|team| team.id == "team2")
            .unwrap()
            .manager_id = Some("mgr-ai".to_string());

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
            youth_target_position: None,
            youth_search_region: None,
            youth_search_objective: None,
            youth_prospects: None,
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
            "accept",
        )
        .expect("effect");

        assert_eq!(effect.i18n_key, "be.msg.jobOffer.effects.unavailable");
        assert_eq!(
            effect.i18n_params.get("team").map(String::as_str),
            Some("New FC")
        );
        assert!(effect.message.contains("no longer available"));
        assert_eq!(game.manager.team_id, None);
    }
}
