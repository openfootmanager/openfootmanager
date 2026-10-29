use crate::contracts::{iso, record_movement};
use crate::game::Game;
use chrono::NaiveDate;
use domain::player::{Player, PlayerMovementEntry, PlayerMovementKind};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn seeded_value(player_id: &str, season: u32, salt: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    player_id.hash(&mut hasher);
    season.hash(&mut hasher);
    salt.hash(&mut hasher);
    (hasher.finish() % u32::MAX as u64) as u32
}

fn has_expired_contract(player: &Player, current_date: NaiveDate) -> bool {
    player
        .contract_end()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .is_some_and(|contract_end| contract_end < current_date)
}

fn retirement_chance(player: &Player, age: i32, current_date: NaiveDate) -> u32 {
    if age < 33 {
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
    let world_seed = game.seed;
    for player in game.players.iter_mut() {
        let age = domain::age::age_on_or_unknown(current_date, &player.date_of_birth);
        let mut stream = crate::seed::rng_for_seed(
            world_seed,
            &format!("development/{}", player.id),
            &iso(current_date),
        );
        crate::development::develop_one_season(player, age, world_seed, &mut stream);
        crate::player_rating::refresh_player_derived(player, current_date);

        if should_retire(player, age, current_date, season) {
            if let Some(team_id) = player.team_id.clone()
                && let Some(team) = game.teams.iter_mut().find(|team| team.id == team_id)
            {
                team.remove_player_references(&player.id);
            }
            retire_player(player, current_date);
        }
    }
    crate::retirees::archive_retired_players(game, &iso(current_date), season);
}

#[cfg(test)]
mod tests {
    use super::{apply_seasonal_aging, retirement_chance, should_retire};
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

        assert!(
            game.players.is_empty(),
            "he leaves the roster for the archive"
        );
        assert_eq!(game.retired_players.len(), 1);
        assert_eq!(game.retired_players[0].id, "older-pro");
        assert_eq!(game.retired_players[0].retired_on, current_date.to_string());
        let veteran = game.retired_players[0].to_player();
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

        assert!(game.players.is_empty());
        let veteran = game.retired_players[0].to_player();
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

    /// Given a contracted veteran at a club, with enough appearances, an unrated season and an
    /// ordinary overall,
    /// when his retirement chance is read at each age from 32 to 40,
    /// then it is 0 before 33, then 12, 24, 42, 60 and 78, and certain from 38.
    #[test]
    fn the_retirement_chance_rises_with_each_year_from_33() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
        let mut player = make_player("ageing-pro", "1990-01-01");
        player.stage_contract_end(Some("2027-06-30".to_string()));
        player.stats.appearances = 20;
        assert!(player.ovr < 80, "an 80+ overall lowers the chance");

        let by_age: Vec<(i32, u32)> = (32..=40)
            .map(|age| (age, retirement_chance(&player, age, date)))
            .collect();

        assert_eq!(
            by_age,
            vec![
                (32, 0),
                (33, 12),
                (34, 24),
                (35, 42),
                (36, 60),
                (37, 78),
                (38, 100),
                (39, 100),
                (40, 100),
            ]
        );
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

    /// A cohort of `count` players of `age` rated like `make_player`, with the given ceiling, in a
    /// game that has played one season of ageing: the mean change in rating of the players still on
    /// the roster afterwards (the retirees leave it, and are not part of the average).
    fn mean_change_after_a_season(age: i32, potential: u8, count: usize) -> f64 {
        let born = format!("{}-01-01", 2026 - age);
        let players: Vec<Player> = (0..count)
            .map(|n| {
                let mut player = make_player(&format!("cohort-{age}-{n}"), &born);
                player.stage_contract_end(Some("2030-06-30".to_string()));
                crate::player_rating::refresh_player_derived(
                    &mut player,
                    chrono::NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
                );
                player.potential = potential;
                player
            })
            .collect();
        let before: std::collections::HashMap<String, f64> = players
            .iter()
            .map(|p| (p.id.clone(), f64::from(p.ovr)))
            .collect();
        let mut game = make_game(players);
        let date = game.clock.current_date.date_naive();

        apply_seasonal_aging(&mut game, date, 1);

        let changes: Vec<f64> = game
            .players
            .iter()
            .map(|after| f64::from(after.ovr) - before[&after.id])
            .collect();
        assert!(!changes.is_empty(), "the whole cohort retired");
        changes.iter().sum::<f64>() / changes.len() as f64
    }

    /// Given a cohort of 17-year-olds with room to grow and a cohort of 35-year-olds,
    /// When a season of ageing is applied,
    /// Then the young gain rating on average and the old lose it.
    #[test]
    fn a_season_develops_the_young_and_wears_the_old() {
        assert!(mean_change_after_a_season(17, 90, 80) > 2.0);
        // Flat-70 attributes, a ceiling at the rating: the mean change of the players who stay.
        assert!(mean_change_after_a_season(35, 70, 200) < -1.0);
    }

    /// Given a save written before ceilings followed decline, in which a 32-year-old rated 70
    /// still carries a ceiling of 90,
    /// When his next summer's ageing runs (no rewrite on load),
    /// Then his ceiling has come down to his rating, and stays there the summer after.
    #[test]
    fn an_old_saves_veteran_loses_his_stale_ceiling_at_the_next_summer() {
        let mut veteran = make_player("old-save", "1994-01-01");
        veteran.stage_contract_end(Some("2031-06-30".to_string()));
        veteran.potential = 90;
        let mut game = make_game(vec![veteran]);
        let date = game.clock.current_date.date_naive();
        assert_eq!(
            game.players[0].potential, 90,
            "the save is loaded as it was"
        );

        apply_seasonal_aging(&mut game, date, 1);

        let after = &game.players[0];
        assert_eq!(
            after.potential, after.ovr,
            "ceiling came down with the rating"
        );
        let next = date + chrono::Months::new(12);
        apply_seasonal_aging(&mut game, next, 2);
        // He may retire in his second summer; if he plays on, his ceiling is still his rating.
        if let Some(after) = game.players.first() {
            assert_eq!(after.potential, after.ovr);
        }
    }

    /// Given a cohort of 26-year-olds rated 70 with a ceiling of 85,
    /// When a season of ageing is applied,
    /// Then they are still growing: development tapers, it does not stop at 26.
    #[test]
    fn development_tapers_through_the_late_twenties_and_does_not_stop_dead() {
        let at_26 = mean_change_after_a_season(26, 85, 120);
        let at_30 = mean_change_after_a_season(30, 85, 120);
        assert!(at_26 > 0.0, "26-year-olds changed by {at_26}");
        assert!(at_26 > at_30, "26: {at_26}, 30: {at_30}");
    }
}
