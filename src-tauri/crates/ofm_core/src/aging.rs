use crate::contracts::{iso, record_movement};
use crate::game::Game;
use chrono::NaiveDate;
use domain::player::{Player, PlayerMovementEntry, PlayerMovementKind};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const MIN_ATTRIBUTE: u8 = 1;
const MAX_ATTRIBUTE: u8 = 99;

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

/// Stamina and agility go from 31, a point or two a summer.
fn veteran_endurance_loss(player_id: &str, age: i32, season: u32) -> u8 {
    if age < 31 {
        return 0;
    }

    1 + (seeded_value(player_id, season, "endurance-loss") % 2) as u8
}

/// Strength goes from 33, a point or two a summer.
fn veteran_strength_loss(player_id: &str, age: i32, season: u32) -> u8 {
    if age < 33 {
        return 0;
    }

    1 + (seeded_value(player_id, season, "strength-loss") % 2) as u8
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
    let endurance_loss = veteran_endurance_loss(&player.id, age, season);
    if endurance_loss > 0 {
        decrease_attribute(&mut player.attributes.stamina, endurance_loss);
        decrease_attribute(&mut player.attributes.agility, endurance_loss);
    }
    let strength_loss = veteran_strength_loss(&player.id, age, season);
    if strength_loss > 0 {
        decrease_attribute(&mut player.attributes.strength, strength_loss);
    }

    // A player who has reached his ceiling has nothing left to learn; without this the
    // ceiling was lifted by the growth below every summer (it is floored at the rating).
    let growth = if player.potential > player.ovr {
        technical_growth(&player.id, age, season)
    } else {
        0
    };
    if growth > 0 {
        increase_attribute(&mut player.attributes.passing, growth);
        increase_attribute(&mut player.attributes.vision, growth);
        increase_attribute(&mut player.attributes.decisions, growth);
        increase_attribute(&mut player.attributes.composure, growth);
    }
}

fn has_expired_contract(player: &Player, current_date: NaiveDate) -> bool {
    player
        .contract_end()
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

    if player.contract_end().is_none() || has_expired_contract(player, current_date) {
        chance += 18;
    }
    if player.team_id.is_none() {
        chance += 10;
    }
    if player.stats.appearances < 10 {
        chance += 8;
    }
    if crate::match_rating::is_rated(player.stats.avg_rating) {
        if player.stats.avg_rating <= 6.4 {
            chance += 8;
        }
        if player.stats.avg_rating >= 7.4 {
            chance = chance.saturating_sub(10);
        }
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

fn retire_player(player: &mut Player, current_date: NaiveDate) {
    // Recorded first, while his club is still known. It ends the contract: the dates
    // and the wage follow it.
    record_movement(
        player,
        PlayerMovementEntry {
            from_team_id: player.contract_club_id().map(str::to_string),
            ..PlayerMovementEntry::new(iso(current_date), PlayerMovementKind::Retired)
        },
    );
    player.retired = true;
    player.team_id = None;
    player.transfer_listed = false;
    player.loan_listed = false;
    player.transfer_offers.clear();
}

pub fn apply_seasonal_aging(game: &mut Game, current_date: NaiveDate, season: u32) {
    for player in game.players.iter_mut() {
        if player.retired {
            continue;
        }

        let age = domain::age::age_on_or_unknown(current_date, &player.date_of_birth);
        apply_attribute_curve(player, age, season);
        crate::player_rating::refresh_player_derived(player, current_date);
        crate::player_rating::settle_ceiling(player, age);

        if should_retire(player, age, current_date, season) {
            if let Some(team_id) = player.team_id.clone()
                && let Some(team) = game.teams.iter_mut().find(|team| team.id == team_id)
            {
                team.remove_player_references(&player.id);
            }
            retire_player(player, current_date);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_attribute_curve, apply_seasonal_aging, retirement_chance, should_retire,
        technical_growth, veteran_endurance_loss, veteran_pace_loss, veteran_strength_loss,
    };
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

    /// Given two players of 36 in the same situation, one rated 85 and one rated 60,
    /// When their chance of retiring this summer is read,
    /// Then it is the same: how good a player is does not keep him playing.
    #[test]
    fn a_star_retires_no_later_than_anyone_else_of_his_age() {
        let current_date = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
        let mut star = make_player("star", "1990-01-01");
        star.ovr = 85;
        let mut journeyman = make_player("journeyman", "1990-01-01");
        journeyman.ovr = 60;

        assert_eq!(
            retirement_chance(&star, 36, current_date),
            retirement_chance(&journeyman, 36, current_date)
        );
    }

    /// Given players of every age around the ages the physical decline starts,
    /// When the stamina and agility loss is rolled,
    /// Then there is none at 30 and a point or two from 31 on.
    #[test]
    fn endurance_loss_starts_at_thirty_one() {
        for season in 1..=20 {
            assert_eq!(veteran_endurance_loss("player", 30, season), 0);
        }
        for age in [31, 35, 40] {
            let losses: Vec<u8> = (1..=20)
                .map(|season| veteran_endurance_loss("player", age, season))
                .collect();
            assert!(
                losses.iter().all(|loss| (1..=2).contains(loss)),
                "age {age}: {losses:?}"
            );
            assert!(
                losses.contains(&1) && losses.contains(&2),
                "age {age}: {losses:?}"
            );
        }
    }

    /// Given players of every age around the age strength starts to go,
    /// When the strength loss is rolled,
    /// Then there is none at 32 and a point or two from 33 on.
    #[test]
    fn strength_loss_starts_at_thirty_three() {
        for season in 1..=20 {
            assert_eq!(veteran_strength_loss("player", 32, season), 0);
        }
        for age in [33, 36, 40] {
            let losses: Vec<u8> = (1..=20)
                .map(|season| veteran_strength_loss("player", age, season))
                .collect();
            assert!(
                losses.iter().all(|loss| (1..=2).contains(loss)),
                "age {age}: {losses:?}"
            );
            assert!(
                losses.contains(&1) && losses.contains(&2),
                "age {age}: {losses:?}"
            );
        }
    }

    /// Given a 34-year-old who has played every season of the run,
    /// When twenty summers of ageing are applied,
    /// Then his stamina, agility and strength only ever fall, and they do fall.
    #[test]
    fn a_veterans_stamina_agility_and_strength_fall() {
        let mut veteran = make_player("veteran", "1992-01-01");
        let before = veteran.attributes.clone();
        for season in 1..=20 {
            let (stamina, agility, strength) = (
                veteran.attributes.stamina,
                veteran.attributes.agility,
                veteran.attributes.strength,
            );

            apply_attribute_curve(&mut veteran, 34, season);

            assert!(veteran.attributes.stamina <= stamina);
            assert!(veteran.attributes.agility <= agility);
            assert!(veteran.attributes.strength <= strength);
        }
        assert!(veteran.attributes.stamina < before.stamina);
        assert!(veteran.attributes.agility < before.agility);
        assert!(veteran.attributes.strength < before.strength);
    }

    /// Given a 24-year-old at his ceiling and one with room to grow,
    /// When twenty summers of ageing are applied to each,
    /// Then the peaked one gains no technical attributes and the other does.
    #[test]
    fn technical_growth_needs_room_under_the_ceiling() {
        let mut peaked = make_player("peaked", "2002-01-01");
        peaked.ovr = 70;
        peaked.potential = 70;
        let mut promising = make_player("promising", "2002-01-01");
        promising.ovr = 70;
        promising.potential = 85;
        for season in 1..=20 {
            apply_attribute_curve(&mut peaked, 24, season);
            apply_attribute_curve(&mut promising, 24, season);
        }

        assert_eq!(peaked.attributes.passing, 70);
        assert!(promising.attributes.passing > 70);
    }

    /// Given a 34-year-old whose stored ceiling is 90 and a 19-year-old whose ceiling is 30 above his rating,
    /// When a summer's ageing is applied,
    /// Then the veteran's ceiling is no more than his rating and the youth's no more than the headroom for 19.
    #[test]
    fn a_ceiling_cannot_sit_above_the_headroom_for_the_age() {
        let mut veteran = make_player("veteran", "1992-01-01");
        veteran.potential = 90;
        let mut youth = make_player("youth", "2007-01-01");
        youth.potential = 99;
        let mut game = make_game(vec![veteran, youth]);

        apply_seasonal_aging(&mut game, NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(), 1);

        let veteran = &game.players[0];
        assert!(veteran.retired || veteran.potential <= veteran.ovr);
        let youth = &game.players[1];
        let room = crate::player_rating::growth_headroom_range(19)
            .end()
            .to_owned();
        assert!(
            youth.potential <= youth.ovr + room,
            "{} vs {}",
            youth.potential,
            youth.ovr
        );
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
        player.stage_contract_end(Some("2026-05-01".to_string()));
        player.stats = PlayerSeasonStats {
            appearances: 6,
            avg_rating: 6.1,
            ..PlayerSeasonStats::default()
        };

        let age = domain::age::age_on_or_unknown(current_date, &player.date_of_birth);
        assert!(should_retire(&player, age, current_date, 1));
    }

    #[test]
    fn apply_seasonal_aging_retires_veteran_and_reduces_pace() {
        let mut veteran = make_player("older-pro", "1988-01-01");
        veteran.stage_contract_start(Some("2023-07-01".to_string()));
        veteran.stage_contract_end(Some("2026-05-01".to_string()));
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
        assert_eq!(
            veteran.contract_start(),
            None,
            "a retired player has no agreement, so neither date survives"
        );
        assert_eq!(veteran.contract_end(), None);
        assert!(!veteran.transfer_listed);
        assert!(veteran.attributes.pace < 20);
        assert!(game.teams[0].starting_xi_ids.is_empty());
        assert!(game.teams[0].training_groups[0].player_ids.is_empty());
        assert_eq!(game.teams[0].match_roles.captain, None);
    }

    /// Retirement ends the contract like any other exit: it is in the history, and
    /// the wage goes with it instead of being left on a player who will never play.
    #[test]
    fn retirement_appends_a_retired_entry_and_zeroes_the_wage() {
        use domain::player::PlayerMovementKind;
        let mut veteran = make_player("older-pro", "1988-01-01");
        veteran.stage_contract_start(Some("2023-07-01".to_string()));
        veteran.stage_contract_end(Some("2026-05-01".to_string()));
        veteran.stage_wage(9_000);
        veteran.stats = PlayerSeasonStats {
            appearances: 6,
            avg_rating: 6.1,
            ..PlayerSeasonStats::default()
        };
        let mut game = make_game(vec![veteran]);
        let current_date = game.clock.current_date.date_naive();

        apply_seasonal_aging(&mut game, current_date, 1);

        let veteran = &game.players[0];
        assert!(veteran.retired);
        let entry = veteran
            .movement_history
            .last()
            .expect("retirement is recorded");
        assert_eq!(entry.kind, PlayerMovementKind::Retired);
        assert_eq!(entry.from_team_id.as_deref(), Some("team1"));
        assert_eq!(veteran.wage(), 0, "no wage left on a retired player");
    }
    /// Given a contracted 34-year-old with enough appearances and an unrated season,
    /// when retirement is considered, then only the age term applies.
    #[test]
    fn an_unrated_veteran_has_no_retirement_rating_modifier() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
        let mut player = make_player("unrated-veteran", "1992-01-01");
        player.stage_contract_end(Some("2027-06-30".to_string()));
        player.stats.appearances = 20;
        player.stats.avg_rating = 0.0;
        assert_eq!(retirement_chance(&player, 34, date), 24);
        player.stats.avg_rating = 6.8;
        assert_eq!(retirement_chance(&player, 34, date), 24);
    }

    /// Given an old save storing the numeric zero sentinel, when the player is loaded,
    /// then his unrated season has no retirement penalty.
    #[test]
    fn a_loaded_unrated_veteran_keeps_the_neutral_retirement_chance() {
        let mut player = make_player("loaded-unrated", "1992-01-01");
        player.stage_contract_end(Some("2027-06-30".to_string()));
        player.stats.appearances = 20;
        let json = serde_json::to_string(&player).unwrap();
        let loaded: Player = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.stats.avg_rating, 0.0);
        assert_eq!(
            retirement_chance(&loaded, 34, NaiveDate::from_ymd_opt(2026, 5, 20).unwrap()),
            24
        );
    }

    /// Given a rated veteran, when retirement is considered, then established poor/good modifiers survive.
    #[test]
    fn a_rated_veteran_keeps_the_existing_retirement_modifiers() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
        let mut player = make_player("rated-veteran", "1992-01-01");
        player.stage_contract_end(Some("2027-06-30".to_string()));
        player.stats.appearances = 20;
        player.stats.avg_rating = 6.4;
        assert_eq!(retirement_chance(&player, 34, date), 32);
        player.stats.avg_rating = 7.4;
        assert_eq!(retirement_chance(&player, 34, date), 14);
    }
}
