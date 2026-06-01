use crate::game::Game;
use chrono::{Datelike, NaiveDate};
use domain::player::Player;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const MIN_ATTRIBUTE: u8 = 1;
const MAX_ATTRIBUTE: u8 = 99;

fn player_age_on(current_date: NaiveDate, date_of_birth: &str) -> i32 {
    let Ok(dob) = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d") else {
        return 30;
    };

    let mut age = current_date.year() - dob.year();
    if current_date.ordinal() < dob.ordinal() {
        age -= 1;
    }
    age
}

fn seeded_value(player_id: &str, season: u32, salt: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    player_id.hash(&mut hasher);
    season.hash(&mut hasher);
    salt.hash(&mut hasher);
    (hasher.finish() % u32::MAX as u64) as u32
}

fn veteran_pace_loss(player_id: &str, age: i32, season: u32) -> u8 {
    if age < 30 {
        return 0;
    }

    1 + (seeded_value(player_id, season, "pace-loss") % 3) as u8
}

fn technical_growth(player_id: &str, age: i32, season: u32) -> u8 {
    if age > 32 {
        return 0;
    }

    (seeded_value(player_id, season, "technical-growth") % 2) as u8
}

fn increase_attribute(value: &mut u8, delta: u8) {
    *value = value.saturating_add(delta).min(MAX_ATTRIBUTE);
}

fn decrease_attribute(value: &mut u8, delta: u8) {
    *value = value.saturating_sub(delta).max(MIN_ATTRIBUTE);
}

fn apply_attribute_curve(player: &mut Player, age: i32, season: u32) {
    let pace_loss = veteran_pace_loss(&player.id, age, season);
    if pace_loss > 0 {
        decrease_attribute(&mut player.attributes.pace, pace_loss);
    }

    let growth = technical_growth(&player.id, age, season);
    if growth > 0 {
        increase_attribute(&mut player.attributes.passing, growth);
        increase_attribute(&mut player.attributes.vision, growth);
        increase_attribute(&mut player.attributes.decisions, growth);
        increase_attribute(&mut player.attributes.composure, growth);
    }
}

fn has_expired_contract(player: &Player, current_date: NaiveDate) -> bool {
    player
        .contract_end
        .as_deref()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .is_some_and(|contract_end| contract_end < current_date)
}

fn retirement_chance(player: &Player, age: i32, current_date: NaiveDate) -> u32 {
    if player.retired || age < 33 {
        return 0;
    }

    let mut chance: u32 = match age {
        33 => 12,
        34 => 24,
        35 => 42,
        36 => 60,
        37 => 78,
        _ => 100,
    };

    if player.contract_end.is_none() || has_expired_contract(player, current_date) {
        chance += 18;
    }
    if player.team_id.is_none() {
        chance += 10;
    }
    if player.stats.appearances < 10 {
        chance += 8;
    }
    if player.stats.avg_rating <= 6.4 {
        chance += 8;
    }
    if player.stats.avg_rating >= 7.4 {
        chance = chance.saturating_sub(10);
    }
    if player.ovr >= 80 {
        chance = chance.saturating_sub(15);
    }

    chance.min(100)
}

fn should_retire(player: &Player, age: i32, current_date: NaiveDate, season: u32) -> bool {
    let chance = retirement_chance(player, age, current_date);
    if chance == 0 {
        return false;
    }

    let roll = seeded_value(&player.id, season, "retirement-roll") % 100;
    roll < chance
}

fn retire_player(player: &mut Player) {
    player.retired = true;
    player.team_id = None;
    player.contract_end = None;
    player.transfer_listed = false;
    player.loan_listed = false;
    player.transfer_offers.clear();
}

pub fn apply_seasonal_aging(game: &mut Game, current_date: NaiveDate, season: u32) {
    for player in game.players.iter_mut() {
        if player.retired {
            continue;
        }

        let age = player_age_on(current_date, &player.date_of_birth);
        apply_attribute_curve(player, age, season);

        if should_retire(player, age, current_date, season) {
            if let Some(team_id) = player.team_id.clone()
                && let Some(team) = game.teams.iter_mut().find(|team| team.id == team_id)
            {
                team.remove_player_references(&player.id);
            }
            retire_player(player);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_seasonal_aging, player_age_on, should_retire, technical_growth, veteran_pace_loss};
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{NaiveDate, TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, PlayerSeasonStats, Position};
    use domain::team::Team;

    fn make_player(id: &str, dob: &str) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            format!("Player {id}"),
            dob.to_string(),
            "England".to_string(),
            Position::Forward,
            PlayerAttributes {
                pace: 70,
                stamina: 70,
                strength: 70,
                agility: 70,
                passing: 70,
                shooting: 70,
                tackling: 70,
                dribbling: 70,
                defending: 70,
                positioning: 70,
                vision: 70,
                decisions: 70,
                composure: 70,
                aggression: 70,
                teamwork: 70,
                leadership: 70,
                handling: 20,
                reflexes: 20,
                aerial: 70,
            },
        );
        player.team_id = Some("team1".to_string());
        player
    }

    fn make_game(players: Vec<Player>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let team = Team::new(
            "team1".to_string(),
            "Test FC".to_string(),
            "TFC".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );

        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    #[test]
    fn veteran_pace_loss_stays_within_supported_range() {
        let loss = veteran_pace_loss("veteran", 34, 5);
        assert!((1..=3).contains(&loss));
        assert_eq!(veteran_pace_loss("prospect", 24, 5), 0);
    }

    #[test]
    fn technical_growth_stops_after_age_32() {
        assert!(technical_growth("player", 31, 3) <= 1);
        assert_eq!(technical_growth("player", 33, 3), 0);
    }

    #[test]
    fn deterministic_retirement_favors_older_out_of_contract_players() {
        let current_date = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
        let mut player = make_player("older-pro", "1988-01-01");
        player.contract_end = Some("2026-05-01".to_string());
        player.stats = PlayerSeasonStats {
            appearances: 6,
            avg_rating: 6.1,
            ..PlayerSeasonStats::default()
        };

        let age = player_age_on(current_date, &player.date_of_birth);
        assert!(should_retire(&player, age, current_date, 1));
    }

    #[test]
    fn apply_seasonal_aging_retires_veteran_and_reduces_pace() {
        let mut veteran = make_player("older-pro", "1988-01-01");
        veteran.contract_end = Some("2026-05-01".to_string());
        veteran.attributes.pace = 20;
        veteran.transfer_listed = true;
        veteran.stats = PlayerSeasonStats {
            appearances: 6,
            avg_rating: 6.1,
            ..PlayerSeasonStats::default()
        };

        let mut game = make_game(vec![veteran]);
        game.teams[0].starting_xi_ids = vec!["older-pro".to_string()];
        game.teams[0].training_groups = vec![domain::team::TrainingGroup {
            id: "group-1".to_string(),
            name: "Core".to_string(),
            focus: domain::team::TrainingFocus::Tactical,
            player_ids: vec!["older-pro".to_string()],
        }];
        game.teams[0].match_roles.captain = Some("older-pro".to_string());
        let current_date = game.clock.current_date.date_naive();

        apply_seasonal_aging(&mut game, current_date, 1);

        let veteran = &game.players[0];
        assert!(veteran.retired);
        assert_eq!(veteran.team_id, None);
        assert!(!veteran.transfer_listed);
        assert!(veteran.attributes.pace < 20);
        assert!(game.teams[0].starting_xi_ids.is_empty());
        assert!(game.teams[0].training_groups[0].player_ids.is_empty());
        assert_eq!(game.teams[0].match_roles.captain, None);
    }
}
