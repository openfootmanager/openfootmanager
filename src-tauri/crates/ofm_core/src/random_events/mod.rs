mod builders_reports;
mod message_builders;
mod responses;

pub(crate) use message_builders::sponsor_offer_message;
pub use responses::{RandomEventResponseEffect, apply_event_response};

use crate::contracts::{ContractWarningStage, contract_warning_stage};
use crate::game::Game;
use crate::morale::apply_morale_change;
use chrono::Datelike;
use domain::message::*;
use rand::RngExt;
use std::collections::HashMap;

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn fallback_club_name() -> &'static str {
    concat!("Your", " ", "Club")
}

fn sponsor_offer_name(index: usize) -> String {
    const SPONSORS: [(&str, &str); 8] = [
        ("GreenTech", "Industries"),
        ("Nova", "Sports"),
        ("Titan", "Energy"),
        ("BlueWave", "Solutions"),
        ("Summit", "Capital"),
        ("Apex", "Motors"),
        ("FreshBrew", concat!("Co", ".")),
        ("CityLink", "Telecom"),
    ];
    let (prefix, suffix) = SPONSORS[index % SPONSORS.len()];
    [prefix, suffix].join(" ")
}

fn rival_interest_name(index: usize) -> String {
    const RIVALS: [(&str, &str); 5] = [
        ("FC", "Rival"),
        ("Sporting", "Ambition"),
        ("United", "Prestige"),
        ("Real", "Progress"),
        ("Bayern", "Elite"),
    ];
    let (prefix, suffix) = RIVALS[index % RIVALS.len()];
    [prefix, suffix].join(" ")
}

/// Injury probability multiplier based on fitness. Less fit players are more
/// prone to training ground injuries.
fn injury_probability_from_fitness(fitness: u8) -> f64 {
    if fitness < 30 {
        3.0
    } else if fitness < 50 {
        2.0
    } else if fitness < 70 {
        1.5
    } else if fitness >= 90 {
        0.7
    } else {
        1.0
    }
}

fn action(id: &str, label: &str, label_key: &str, action_type: ActionType) -> MessageAction {
    MessageAction {
        id: id.to_string(),
        label: label.to_string(),
        action_type,
        resolved: false,
        label_key: Some(label_key.to_string()),
    }
}

/// Generate random daily events. Called from process_day().
/// Events have low probability each day so they don't spam the player.
pub fn check_random_events(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let user_team_id = match game.manager.team_id.clone() {
        Some(id) => id,
        None => return,
    };

    // The ledger, not the mailbox: a message the player deleted was still sent.
    // Borrowed, not cloned — the ledger is never pruned, so it only grows.
    let existing_ids = &game.emitted_events;

    let mut rng = game.rng_today("random-events");
    let mut new_messages: Vec<InboxMessage> = Vec::new();

    // --- 1. Sponsor offer (1% chance per day) ---
    {
        let msg_id = format!("sponsor_{}", today);
        if !existing_ids.contains(&msg_id) && rng.random_range(0..100) == 0 {
            let team_name = game
                .teams
                .iter()
                .find(|t| t.id == user_team_id)
                .map(|t| t.name.as_str())
                .unwrap_or(fallback_club_name());
            let amount = rng.random_range(5..=30) * 10_000; // 50k - 300k
            let sponsor = sponsor_offer_name(rng.random_range(0..8));

            new_messages.push(message_builders::sponsor_offer_message(
                &msg_id, team_name, &sponsor, amount, &today,
            ));
        }
    }

    // --- 2. Training ground injury (fitness-weighted probability, non-match days) ---
    {
        let has_match = game.league.as_ref().is_some_and(|l| {
            l.fixtures
                .iter()
                .any(|f| f.date == today && f.status == domain::league::FixtureStatus::Scheduled)
        });
        if !has_match {
            // Pick a random non-injured player, biased toward less fit players.
            let eligible: Vec<&domain::player::Player> = game
                .players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(&user_team_id) && p.injury.is_none())
                .collect();
            if !eligible.is_empty() {
                let player = eligible[rng.random_range(0..eligible.len())];
                // Base 2% chance, scaled by fitness: less fit → higher probability
                let fitness_mult = injury_probability_from_fitness(player.fitness);
                let base_prob = 1.0_f64 / 50.0;
                let injury_prob = (base_prob * fitness_mult).min(1.0);
                if rng.random_bool(injury_prob) {
                    let msg_id = format!("training_injury_{}_{}", player.id, today);
                    if !existing_ids.contains(&msg_id) {
                        let days = rng.random_range(3..=14);
                        let injury_names = [
                            "common.injuries.minorMuscleStrain",
                            "common.injuries.twistedAnkle",
                            "common.injuries.kneeBruise",
                            "common.injuries.hamstringTightness",
                            "common.injuries.calfStrain",
                        ];
                        let injury_name = injury_names[rng.random_range(0..injury_names.len())];

                        new_messages.push(message_builders::training_injury_message(
                            &msg_id,
                            &player.id,
                            &player.match_name,
                            injury_name,
                            days,
                            &today,
                        ));

                        // Apply the injury
                        let pid = player.id.clone();
                        let player_name = player.match_name.clone();
                        let player_team_id = player.team_id.clone();
                        let is_notable = player.market_value >= 500_000
                            || player_team_id.as_deref().is_some_and(|tid| {
                                game.teams
                                    .iter()
                                    .find(|t| t.id == tid)
                                    .is_some_and(|t| t.starting_xi_ids.iter().any(|id| id == &pid))
                            });

                        if let Some(p) = game.players.iter_mut().find(|p| p.id == pid) {
                            p.injury = Some(domain::player::Injury {
                                name: injury_name.to_string(),
                                days_remaining: days,
                            });
                        }

                        // Generate a public news article for notable players
                        if is_notable {
                            let news_id = format!("injury_news_{}_{}", pid, today);
                            let actual_team_id = player_team_id.as_deref().unwrap_or(&user_team_id);
                            let team_name = game
                                .teams
                                .iter()
                                .find(|t| t.id == actual_team_id)
                                .map(|t| t.name.clone())
                                .unwrap_or_else(|| actual_team_id.to_string());
                            let date_str = game.clock.current_date.to_rfc3339();
                            if !game.news.iter().any(|a| a.id == news_id) {
                                game.news.push(crate::news::injury_news_article(
                                    &news_id,
                                    &pid,
                                    &player_name,
                                    actual_team_id,
                                    &team_name,
                                    days,
                                    &date_str,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // --- 3. Media story (2% chance per day) ---
    {
        let msg_id = format!("media_{}", today);
        if !existing_ids.contains(&msg_id) && rng.random_range(0..50) == 0 {
            let team_name = game
                .teams
                .iter()
                .find(|t| t.id == user_team_id)
                .map(|t| t.name.as_str())
                .unwrap_or(fallback_club_name());

            // Pick a random player for the story
            let team_players: Vec<&domain::player::Player> = game
                .players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(&user_team_id))
                .collect();
            if !team_players.is_empty() {
                let player = team_players[rng.random_range(0..team_players.len())];
                let is_positive = rng.random_bool(0.6); // 60% positive stories

                new_messages.push(message_builders::media_story_message(
                    &msg_id,
                    team_name,
                    &player.id,
                    &player.match_name,
                    is_positive,
                    &today,
                ));

                // Apply morale effect
                let pid = player.id.clone();
                if let Some(p) = game.players.iter_mut().find(|p| p.id == pid) {
                    let delta: i16 = if is_positive {
                        rng.random_range(2..=5)
                    } else {
                        rng.random_range(-5..=-1)
                    };
                    p.morale = apply_morale_change(p.morale, delta);
                }
            }
        }
    }

    // --- 4. International call-up (5% chance per day, only if match in next 7 days) ---
    {
        let upcoming_match = game.league.as_ref().and_then(|l| {
            let current = game.clock.current_date;
            l.fixtures.iter().find(|f| {
                f.status == domain::league::FixtureStatus::Scheduled
                    && (f.home_team_id == user_team_id || f.away_team_id == user_team_id)
                    && {
                        if let Ok(d) = chrono::NaiveDate::parse_from_str(&f.date, "%Y-%m-%d") {
                            let diff = (d - current.date_naive()).num_days();
                            (1..=7).contains(&diff)
                        } else {
                            false
                        }
                    }
            })
        });

        if upcoming_match.is_some() && rng.random_range(0..50) == 0 {
            let eligible: Vec<&domain::player::Player> = game
                .players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(&user_team_id) && p.injury.is_none())
                .collect();
            if !eligible.is_empty() {
                let player = eligible[rng.random_range(0..eligible.len())];
                let msg_id = format!("intl_callup_{}_{}", player.id, today);
                if !existing_ids.contains(&msg_id) {
                    new_messages.push(message_builders::international_callup_message(
                        &msg_id,
                        &player.match_name,
                        &player.nationality,
                        &today,
                    ));
                    // Morale boost for being called up
                    let pid = player.id.clone();
                    if let Some(p) = game.players.iter_mut().find(|p| p.id == pid) {
                        p.morale = apply_morale_change(p.morale, rng.random_range(3..=8));
                    }
                }
            }
        }
    }

    // --- 5. Community event / club milestone (1% chance per day) ---
    {
        let msg_id = format!("community_{}", today);
        if !existing_ids.contains(&msg_id) && rng.random_range(0..100) == 0 {
            let team_name = game
                .teams
                .iter()
                .find(|t| t.id == user_team_id)
                .map(|t| t.name.as_str())
                .unwrap_or(fallback_club_name());
            new_messages.push(message_builders::community_event_message(
                &msg_id, team_name, &today,
            ));
        }
    }

    // --- 6. Dressing room mood report (once per week, deterministic on Monday) ---
    {
        let is_monday = game.clock.current_date.weekday() == chrono::Weekday::Mon;
        let msg_id = format!("mood_report_{}", today);
        if !existing_ids.contains(&msg_id) && is_monday {
            let team_players: Vec<&domain::player::Player> = game
                .players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(&user_team_id))
                .collect();
            if !team_players.is_empty() {
                let avg_morale: f64 = team_players.iter().map(|p| p.morale as f64).sum::<f64>()
                    / team_players.len() as f64;
                let low_morale_count = team_players.iter().filter(|p| p.morale < 40).count();
                let high_morale_count = team_players.iter().filter(|p| p.morale >= 80).count();

                new_messages.push(builders_reports::mood_report_message(
                    &msg_id,
                    avg_morale,
                    low_morale_count,
                    high_morale_count,
                    team_players.len(),
                    &today,
                ));
            }
        }
    }

    // --- 7. Board confidence check (after 3+ consecutive losses, 100% trigger once) ---
    {
        if let Some(league) = &game.league {
            let completed: Vec<_> = league
                .fixtures
                .iter()
                .filter(|f| {
                    f.status == domain::league::FixtureStatus::Completed
                        && (f.home_team_id == user_team_id || f.away_team_id == user_team_id)
                })
                .collect();
            if completed.len() >= 3 {
                let last3 = &completed[completed.len() - 3..];
                let losses = last3
                    .iter()
                    .filter(|f| {
                        if let Some(r) = &f.result {
                            let user_goals = if f.home_team_id == user_team_id {
                                r.home_goals
                            } else {
                                r.away_goals
                            };
                            let opp_goals = if f.home_team_id == user_team_id {
                                r.away_goals
                            } else {
                                r.home_goals
                            };
                            user_goals < opp_goals
                        } else {
                            false
                        }
                    })
                    .count();
                let last_loss_date = last3
                    .last()
                    .map(|f| f.date.clone())
                    .unwrap_or(today.clone());
                let msg_id = format!("board_confidence_{}", last_loss_date);
                if losses >= 3 && !existing_ids.contains(&msg_id) {
                    new_messages.push(builders_reports::board_confidence_message(&msg_id, &today));
                }
            }
        }
    }

    // --- 8. Fan petition (2% chance per day) ---
    {
        let msg_id = format!("fan_petition_{}", today);
        if !existing_ids.contains(&msg_id) && rng.random_range(0..50) == 0 {
            let team_name = game
                .teams
                .iter()
                .find(|t| t.id == user_team_id)
                .map(|t| t.name.as_str())
                .unwrap_or(fallback_club_name());
            new_messages.push(builders_reports::fan_petition_message(
                &msg_id, team_name, &today,
            ));
        }
    }

    // --- 9. Rival interest in player (2% chance per day) ---
    {
        let msg_id = format!("rival_interest_{}", today);
        if !existing_ids.contains(&msg_id) && rng.random_range(0..50) == 0 {
            let current_date = game.clock.current_date.date_naive();
            let eligible: Vec<&domain::player::Player> = game
                .players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(&user_team_id) && p.injury.is_none())
                .collect();
            if !eligible.is_empty() {
                let player = choose_rival_interest_target(&eligible, current_date, &mut rng)
                    .unwrap_or(eligible[rng.random_range(0..eligible.len())]);
                let rival = rival_interest_name(rng.random_range(0..5));
                new_messages.push(builders_reports::rival_interest_message(
                    &msg_id,
                    &player.id,
                    &player.match_name,
                    &rival,
                    &today,
                ));
            }
        }
    }

    crate::inbox::emit_all(game, new_messages);
}

pub fn rival_interest_weight(
    player: &domain::player::Player,
    current_date: chrono::NaiveDate,
) -> u32 {
    let contract_weight = match contract_warning_stage(player.contract_end(), current_date) {
        Some(ContractWarningStage::FinalWeeks) => 16,
        Some(ContractWarningStage::ThreeMonths) => 12,
        Some(ContractWarningStage::SixMonths) => 8,
        Some(ContractWarningStage::TwelveMonths) => 5,
        None => 1,
    };

    let value_weight = if player.market_value >= 2_000_000 {
        3
    } else if player.market_value >= 750_000 {
        2
    } else {
        1
    };

    contract_weight + value_weight
}

fn choose_rival_interest_target<'a, R: rand::Rng + ?Sized>(
    eligible: &[&'a domain::player::Player],
    current_date: chrono::NaiveDate,
    rng: &mut R,
) -> Option<&'a domain::player::Player> {
    let total_weight: u32 = eligible
        .iter()
        .map(|player| rival_interest_weight(player, current_date))
        .sum();

    if total_weight == 0 {
        return None;
    }

    let mut roll = rng.random_range(0..total_weight);
    for player in eligible {
        let weight = rival_interest_weight(player, current_date);
        if roll < weight {
            return Some(*player);
        }
        roll -= weight;
    }

    eligible.last().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{Player, PlayerAttributes, Position};
    use rand::SeedableRng;

    /// Given fitness at each side of every step: 29, 30, 40, 49, 50, 60, 69, 70, 80, 89 and 90
    /// When the training-injury multiplier is read
    /// Then it steps 3.0, 2.0, 1.5, 1.0 and 0.7 at 30, 50, 70 and 90
    #[test]
    fn injury_multiplier_steps_at_thirty_fifty_seventy_and_ninety() {
        let multipliers: Vec<f64> = [29, 30, 40, 49, 50, 60, 69, 70, 80, 89, 90]
            .into_iter()
            .map(injury_probability_from_fitness)
            .collect();

        assert_eq!(
            multipliers,
            vec![3.0, 2.0, 2.0, 2.0, 1.5, 1.5, 1.5, 1.0, 1.0, 1.0, 0.7]
        );
    }

    /// Given rival indices 3 and 8
    /// When each is named
    /// Then both are Real Progress: the five rivals repeat in turn
    #[test]
    fn rival_names_cycle_through_all_five_clubs() {
        assert_eq!(
            (rival_interest_name(3), rival_interest_name(8)),
            ("Real Progress".to_string(), "Real Progress".to_string())
        );
    }

    fn squad_player(id: &str, contract_end: Option<&str>, market_value: u64) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1995-01-01".to_string(),
            "England".to_string(),
            Position::Midfielder,
            PlayerAttributes {
                pace: 60,
                stamina: 60,
                strength: 60,
                agility: 60,
                passing: 60,
                shooting: 60,
                tackling: 60,
                dribbling: 60,
                defending: 60,
                positioning: 60,
                vision: 60,
                decisions: 60,
                composure: 60,
                aggression: 60,
                teamwork: 60,
                leadership: 60,
                handling: 30,
                reflexes: 30,
                aerial: 60,
            },
        );
        player.stage_contract_end(contract_end.map(str::to_string));
        player.market_value = market_value;
        player
    }

    fn mid_june() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()
    }

    /// Given a squad of one eligible player
    /// When a rival picks a target
    /// Then it is him: a lone player is always chosen
    #[test]
    fn a_lone_eligible_player_is_the_rivals_target() {
        let only = squad_player("only", None, 0);
        let eligible = [&only];
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        let target = choose_rival_interest_target(&eligible, mid_june(), &mut rng);

        assert_eq!(target.map(|player| player.id.as_str()), Some("only"));
    }

    /// Given ten players on long contracts (weight 2 each) and one in the final weeks of his
    ///       (weight 17)
    /// When a rival picks a target 10,000 times
    /// Then the expiring player is picked at least 30% of the time (17 in 37 is about 46%)
    #[test]
    fn rival_interest_falls_mostly_on_a_player_whose_contract_is_running_out() {
        let secure: Vec<Player> = (0..10)
            .map(|i| squad_player(&format!("secure{i}"), None, 0))
            .collect();
        let expiring = squad_player("expiring", Some("2025-07-05"), 0);
        let mut eligible: Vec<&Player> = secure.iter().collect();
        eligible.push(&expiring);
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        let picks = (0..10_000)
            .filter(|_| {
                choose_rival_interest_target(&eligible, mid_june(), &mut rng)
                    .is_some_and(|player| player.id == "expiring")
            })
            .count();

        assert!(picks >= 3_000, "{picks} of 10,000");
    }
}
