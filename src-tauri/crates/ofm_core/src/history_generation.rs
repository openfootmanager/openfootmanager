use crate::{game::Game, season_awards::SeasonAwards};
use chrono::{TimeZone, Utc};
use domain::{
    league::{League, StandingEntry},
    manager::ManagerCareerEntry,
    player::{CareerEntry, Player, PlayerSeasonStats, Position},
    team::TeamSeasonRecord,
    world_history::{
        HistoricalManagerAwardWinner, HistoricalPlayerAwardWinner, HistoricalSeasonAwardsRecord,
    },
};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const DEFAULT_HISTORY_LEAGUE_ID: &str = "history-league";
const DEFAULT_HISTORY_LEAGUE_NAME: &str = "Historical League";

fn deterministic_value(parts: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    parts.hash(&mut hasher);
    hasher.finish()
}

fn deterministic_u32(parts: impl Hash, modulo: u32) -> u32 {
    if modulo == 0 {
        0
    } else {
        (deterministic_value(parts) % modulo as u64) as u32
    }
}

fn season_start_string(season: u32) -> String {
    format!("{:04}-07-01", season)
}

fn build_historical_standings(game: &Game, season: u32) -> Vec<StandingEntry> {
    let matches_played = ((game.teams.len().saturating_sub(1)) * 2) as u32;
    let mut ranking_scores: Vec<(String, u32)> = game
        .teams
        .iter()
        .map(|team| {
            let score = team.reputation.saturating_mul(10)
                + deterministic_u32((&team.id, season, "ranking"), 250);
            (team.id.clone(), score)
        })
        .collect();
    ranking_scores.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    ranking_scores
        .into_iter()
        .enumerate()
        .map(|(index, (team_id, _))| {
            let strength = (game.teams.len().saturating_sub(index)) as u32;
            let max_draws =
                matches_played.min(4 + deterministic_u32((&team_id, season, "draws"), 5));
            let target_wins = ((matches_played * strength) / (game.teams.len() as u32 + 1))
                .saturating_add(2)
                .min(matches_played.saturating_sub(max_draws));
            let wins = target_wins;
            let draws = max_draws.min(matches_played.saturating_sub(wins));
            let losses = matches_played.saturating_sub(wins + draws);
            let goals_for = 18
                + wins.saturating_mul(2)
                + draws
                + deterministic_u32((&team_id, season, "goals_for"), 12);
            let goals_against = 10
                + losses.saturating_mul(2)
                + deterministic_u32((&team_id, season, "goals_against"), 10)
                + index as u32;

            StandingEntry {
                team_id,
                played: matches_played,
                won: wins,
                drawn: draws,
                lost: losses,
                goals_for,
                goals_against,
                points: wins.saturating_mul(3).saturating_add(draws),
            }
        })
        .collect()
}

fn upsert_team_history(game: &mut Game, season: u32, standings: &[StandingEntry]) {
    let standings_by_team: HashMap<&str, (u32, &StandingEntry)> = standings
        .iter()
        .enumerate()
        .map(|(index, standing)| (standing.team_id.as_str(), ((index + 1) as u32, standing)))
        .collect();

    for team in game.teams.iter_mut() {
        let Some((league_position, standing)) = standings_by_team.get(team.id.as_str()) else {
            continue;
        };

        team.history.retain(|record| record.season != season);
        let record = TeamSeasonRecord {
            season,
            league_position: *league_position,
            played: standing.played,
            won: standing.won,
            drawn: standing.drawn,
            lost: standing.lost,
            goals_for: standing.goals_for,
            goals_against: standing.goals_against,
        };

        if team
            .history
            .last()
            .is_none_or(|existing| existing.season < season)
        {
            team.history.push(record);
        } else {
            team.history.push(record);
            team.history
                .sort_by(|left, right| left.season.cmp(&right.season));
        }
    }
}

fn prepare_seeded_managers(game: &mut Game, first_season: u32) {
    crate::ai_hiring::seed_ai_managers(game);
    let start_date = season_start_string(first_season);

    for manager in game.managers.iter_mut() {
        let Some(team_id) = manager.team_id.clone() else {
            continue;
        };

        if let Some(entry) = manager
            .career_history
            .iter_mut()
            .find(|entry| entry.team_id == team_id && entry.end_date.is_none())
        {
            if entry.matches == 0 && entry.wins == 0 && entry.draws == 0 && entry.losses == 0 {
                entry.start_date = start_date.clone();
            }
            manager
                .career_history
                .sort_by(|left, right| left.start_date.cmp(&right.start_date));
            continue;
        }

        let team_name = game
            .teams
            .iter()
            .find(|team| team.id == team_id)
            .map(|team| team.name.clone())
            .unwrap_or_default();
        manager.career_history.push(ManagerCareerEntry {
            team_id,
            team_name,
            start_date: start_date.clone(),
            end_date: None,
            matches: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            best_league_position: None,
        });
        manager
            .career_history
            .sort_by(|left, right| left.start_date.cmp(&right.start_date));
    }
}

fn upsert_manager_history(game: &mut Game, standings: &[StandingEntry]) {
    let standings_by_team: HashMap<&str, (u32, &StandingEntry)> = standings
        .iter()
        .enumerate()
        .map(|(index, standing)| (standing.team_id.as_str(), ((index + 1) as u32, standing)))
        .collect();

    for manager in game.managers.iter_mut() {
        let Some(team_id) = manager.team_id.clone() else {
            continue;
        };
        let Some((position, standing)) = standings_by_team.get(team_id.as_str()) else {
            continue;
        };
        let total_matches = standing.won + standing.drawn + standing.lost;

        manager.career_stats.matches_managed += total_matches;
        manager.career_stats.wins += standing.won;
        manager.career_stats.draws += standing.drawn;
        manager.career_stats.losses += standing.lost;
        if *position == 1 {
            manager.career_stats.trophies += 1;
        }
        if manager
            .career_stats
            .best_finish
            .is_none_or(|best_finish| *position < best_finish)
        {
            manager.career_stats.best_finish = Some(*position);
        }

        if let Some(entry) = manager
            .career_history
            .iter_mut()
            .find(|entry| entry.team_id == team_id && entry.end_date.is_none())
        {
            entry.matches += total_matches;
            entry.wins += standing.won;
            entry.draws += standing.drawn;
            entry.losses += standing.lost;
            if entry
                .best_league_position
                .is_none_or(|best_finish| *position < best_finish)
            {
                entry.best_league_position = Some(*position);
            }
        }
    }
}

fn base_rating(player: &Player, position_bonus: f32, season: u32) -> f32 {
    let ovr_bonus = (player.ovr.saturating_sub(55) as f32) / 18.0;
    let variation = deterministic_u32((&player.id, season, "rating"), 30) as f32 / 100.0;
    (6.1 + position_bonus + ovr_bonus + variation).clamp(6.0, 9.6)
}

fn synthesize_player_season(
    player: &Player,
    matches_played: u32,
    season: u32,
) -> PlayerSeasonStats {
    let missed_matches = deterministic_u32((&player.id, season, "missed"), 8);
    let appearances = matches_played.saturating_sub(missed_matches).max(8);
    let minutes_played = appearances.saturating_mul(90);
    let group_position = player.position.to_group_position();

    let (goals, assists, clean_sheets, shots, passes_completed, tackles_won, interceptions, rating) =
        match group_position {
            Position::Goalkeeper => {
                let clean_sheets =
                    appearances / 3 + deterministic_u32((&player.id, season, "clean_sheets"), 4);
                (
                    0,
                    0,
                    clean_sheets,
                    0,
                    appearances.saturating_mul(18),
                    appearances / 2,
                    appearances,
                    base_rating(player, 0.2, season),
                )
            }
            Position::Defender => {
                let clean_sheets =
                    appearances / 4 + deterministic_u32((&player.id, season, "clean_sheets"), 3);
                (
                    appearances / 12 + deterministic_u32((&player.id, season, "goals"), 3),
                    appearances / 10 + deterministic_u32((&player.id, season, "assists"), 3),
                    clean_sheets,
                    appearances.saturating_mul(1),
                    appearances.saturating_mul(26),
                    appearances.saturating_mul(3),
                    appearances.saturating_mul(2),
                    base_rating(player, 0.1, season),
                )
            }
            Position::Midfielder => (
                appearances / 6 + deterministic_u32((&player.id, season, "goals"), 5),
                appearances / 5 + deterministic_u32((&player.id, season, "assists"), 4),
                0,
                appearances.saturating_mul(2),
                appearances.saturating_mul(32),
                appearances.saturating_mul(2),
                appearances.saturating_mul(2),
                base_rating(player, 0.25, season),
            ),
            Position::Forward => (
                appearances / 3 + deterministic_u32((&player.id, season, "goals"), 8),
                appearances / 7 + deterministic_u32((&player.id, season, "assists"), 3),
                0,
                appearances.saturating_mul(3),
                appearances.saturating_mul(18),
                appearances,
                appearances / 2,
                base_rating(player, 0.35, season),
            ),
            _ => unreachable!(),
        };

    let shots_on_target = shots.saturating_sub(shots / 3);
    let passes_attempted = passes_completed.saturating_add(appearances.saturating_mul(4));
    let fouls_committed = deterministic_u32((&player.id, season, "fouls"), appearances.max(1));

    PlayerSeasonStats {
        appearances,
        goals,
        assists,
        clean_sheets,
        yellow_cards: deterministic_u32((&player.id, season, "yellow"), 5),
        red_cards: deterministic_u32((&player.id, season, "red"), 2),
        avg_rating: rating,
        minutes_played,
        shots,
        shots_on_target,
        passes_completed,
        passes_attempted,
        tackles_won,
        interceptions,
        fouls_committed,
    }
}

fn upsert_player_career(game: &mut Game, season: u32, standings: &[StandingEntry]) {
    let played_by_team: HashMap<&str, u32> = standings
        .iter()
        .map(|standing| (standing.team_id.as_str(), standing.played))
        .collect();
    let team_names: HashMap<&str, &str> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), team.name.as_str()))
        .collect();

    for player in game.players.iter_mut() {
        let Some(team_id) = player.team_id.as_deref() else {
            continue;
        };
        let Some(matches_played) = played_by_team.get(team_id).copied() else {
            continue;
        };

        player.stats = synthesize_player_season(player, matches_played, season);
        player.career.retain(|entry| entry.season != season);
        let entry = CareerEntry {
            season,
            team_id: team_id.to_string(),
            team_name: team_names
                .get(team_id)
                .copied()
                .unwrap_or_default()
                .to_string(),
            appearances: player.stats.appearances,
            goals: player.stats.goals,
            assists: player.stats.assists,
        };

        if player
            .career
            .last()
            .is_none_or(|existing| existing.season < season)
        {
            player.career.push(entry);
        } else {
            player.career.push(entry);
            player
                .career
                .sort_by(|left, right| left.season.cmp(&right.season));
        }
    }
}

fn map_player_award_entry(
    entry: Option<&crate::season_awards::AwardEntry>,
) -> Option<HistoricalPlayerAwardWinner> {
    entry.map(|entry| HistoricalPlayerAwardWinner {
        player_id: entry.player_id.clone(),
        player_name: entry.player_name.clone(),
        team_id: entry.team_id.clone(),
        team_name: entry.team_name.clone(),
        value: entry.value,
    })
}

fn map_manager_award_entry(
    entry: Option<&crate::season_awards::ManagerAwardEntry>,
) -> Option<HistoricalManagerAwardWinner> {
    entry.map(|entry| HistoricalManagerAwardWinner {
        manager_id: entry.manager_id.clone(),
        manager_name: entry.manager_name.clone(),
        team_id: entry.team_id.clone(),
        team_name: entry.team_name.clone(),
        value: entry.value,
        win_rate: entry.win_rate,
    })
}

fn record_historical_awards(game: &mut Game, season: u32, awards: &SeasonAwards) {
    game.world_history
        .record_season_awards(HistoricalSeasonAwardsRecord {
            season,
            golden_boot: map_player_award_entry(awards.golden_boot.first()),
            assist_king: map_player_award_entry(awards.assist_king.first()),
            player_of_year: map_player_award_entry(awards.player_of_year.first()),
            clean_sheet_king: map_player_award_entry(awards.clean_sheet_king.first()),
            most_appearances: map_player_award_entry(awards.most_appearances.first()),
            young_player: map_player_award_entry(awards.young_player.first()),
            manager_of_season: map_manager_award_entry(awards.manager_of_season.first()),
        });
}

fn update_historical_rivalries(game: &mut Game, season: u32, standings: &[StandingEntry]) {
    let top_two = standings.iter().take(2).collect::<Vec<_>>();
    if top_two.len() == 2 {
        let team_a = &top_two[0].team_id;
        let team_b = &top_two[1].team_id;
        let existing = game.world_history.rivalries.iter().find(|rivalry| {
            (rivalry.team_a_id == *team_a && rivalry.team_b_id == *team_b)
                || (rivalry.team_a_id == *team_b && rivalry.team_b_id == *team_a)
        });
        let started_season = existing
            .and_then(|rivalry| rivalry.started_season)
            .map(|existing_start| existing_start.min(season))
            .or(Some(season));
        let intensity = existing
            .map(|rivalry| rivalry.intensity)
            .unwrap_or(55)
            .max(55 + deterministic_u32((team_a, team_b, season, "rivalry"), 36) as u8);
        game.world_history.upsert_rivalry(
            team_a.clone(),
            team_b.clone(),
            intensity,
            started_season,
        );
    }
}

fn reset_player_stats(game: &mut Game) {
    for player in &mut game.players {
        player.stats = PlayerSeasonStats::default();
    }
}

pub fn generate_past_world_history(game: &mut Game, start_year: i32, history_depth_years: u32) {
    if history_depth_years == 0 || game.teams.len() < 2 {
        return;
    }

    let first_season = start_year.saturating_sub(history_depth_years as i32);
    if first_season >= start_year {
        return;
    }

    prepare_seeded_managers(game, first_season as u32);

    let original_date = game.clock.current_date;
    let original_league = game.league.clone();

    for season_year in first_season..start_year {
        let season = season_year as u32;
        let standings = build_historical_standings(game, season);
        upsert_team_history(game, season, &standings);
        upsert_manager_history(game, &standings);
        upsert_player_career(game, season, &standings);

        game.clock.current_date = Utc
            .with_ymd_and_hms(season_year + 1, 5, 31, 0, 0, 0)
            .unwrap();
        game.league = Some(League {
            id: DEFAULT_HISTORY_LEAGUE_ID.to_string(),
            name: DEFAULT_HISTORY_LEAGUE_NAME.to_string(),
            season,
            fixtures: Vec::new(),
            standings: standings.clone(),
            transfer_log: Vec::new(),
            transfer_rumours: Vec::new(),
        });

        let awards = crate::season_awards::compute_season_awards(game);
        record_historical_awards(game, season, &awards);
        update_historical_rivalries(game, season, &standings);
        reset_player_stats(game);
    }

    game.clock.current_date = original_date;
    game.league = original_league;
}

#[cfg(test)]
mod tests {
    use super::generate_past_world_history;
    use crate::{clock::GameClock, game::Game};
    use chrono::{TimeZone, Utc};
    use domain::{
        manager::{Manager, ManagerCareerEntry},
        player::{CareerEntry, Player, PlayerAttributes, Position},
        staff::{Staff, StaffAttributes, StaffRole},
        team::{Team, TeamSeasonRecord},
    };

    fn sample_player_attributes(position: &Position) -> PlayerAttributes {
        let mut attributes = PlayerAttributes {
            pace: 68,
            stamina: 70,
            strength: 67,
            agility: 69,
            passing: 66,
            shooting: 64,
            tackling: 58,
            dribbling: 65,
            defending: 57,
            positioning: 64,
            vision: 65,
            decisions: 66,
            composure: 64,
            aggression: 55,
            teamwork: 68,
            leadership: 54,
            handling: 18,
            reflexes: 18,
            aerial: 55,
        };

        match position {
            Position::Goalkeeper => {
                attributes.handling = 72;
                attributes.reflexes = 74;
                attributes.aerial = 70;
                attributes.positioning = 69;
                attributes.defending = 32;
                attributes.tackling = 24;
                attributes.shooting = 16;
            }
            Position::Striker => {
                attributes.shooting = 78;
                attributes.positioning = 74;
                attributes.dribbling = 72;
            }
            Position::CentralMidfielder => {
                attributes.passing = 74;
                attributes.vision = 73;
                attributes.stamina = 73;
            }
            _ => {}
        }

        attributes
    }

    fn make_team(index: usize) -> Team {
        let city = format!("City {}", index + 1);
        let mut team = Team::new(
            format!("team-{}", index + 1),
            format!("Club {}", index + 1),
            format!("C{}", index + 1),
            if index % 2 == 0 {
                "England".to_string()
            } else {
                "Spain".to_string()
            },
            city.clone(),
            format!("{} Arena", city),
            25_000 + (index as u32 * 1_000),
        );
        team.reputation = 520 + (index as u32 * 55);
        team.finance = 1_500_000 + (index as i64 * 250_000);
        team
    }

    fn make_staff(team_id: &str, index: usize) -> Staff {
        let mut staff = Staff::new(
            format!("staff-{}", index + 1),
            format!("Coach{}", index + 1),
            "Seed".to_string(),
            format!("197{}-02-01", index),
            StaffRole::AssistantManager,
            StaffAttributes {
                coaching: 70,
                judging_ability: 68,
                judging_potential: 67,
                physiotherapy: 42,
            },
        );
        staff.nationality = "England".to_string();
        staff.team_id = Some(team_id.to_string());
        staff
    }

    fn make_player(team_id: &str, index: usize, position: Position, ovr: u8) -> Player {
        let year = 1990 + index as i32;
        let mut player = Player::new(
            format!("player-{}", index + 1),
            format!("P{}", index + 1),
            format!("Player {}", index + 1),
            format!("{}-03-15", year),
            "England".to_string(),
            position.clone(),
            sample_player_attributes(&position),
        );
        player.team_id = Some(team_id.to_string());
        player.ovr = ovr;
        player.potential = (ovr + 6).min(99);
        player
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2032, 7, 1, 0, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = (0..4).map(make_team).collect::<Vec<_>>();
        let staff = teams
            .iter()
            .enumerate()
            .map(|(index, team)| make_staff(&team.id, index))
            .collect::<Vec<_>>();
        let mut players = Vec::new();
        for (index, team) in teams.iter().enumerate() {
            players.push(make_player(
                &team.id,
                index * 3,
                Position::Goalkeeper,
                64 + index as u8,
            ));
            players.push(make_player(
                &team.id,
                index * 3 + 1,
                Position::CentralMidfielder,
                68 + index as u8,
            ));
            players.push(make_player(
                &team.id,
                index * 3 + 2,
                Position::Striker,
                72 + index as u8,
            ));
        }

        Game::new(clock, manager, teams, players, staff, vec![])
    }

    fn seed_later_history_entries(game: &mut Game) {
        let team_id = game.teams[0].id.clone();
        let team_name = game.teams[0].name.clone();

        game.manager.team_id = Some(team_id.clone());
        game.teams[0].manager_id = Some(game.manager.id.clone());
        game.teams[0].history.push(TeamSeasonRecord {
            season: 2031,
            league_position: 2,
            played: 38,
            won: 22,
            drawn: 8,
            lost: 8,
            goals_for: 64,
            goals_against: 36,
        });

        let player = game
            .players
            .iter_mut()
            .find(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            .expect("seed player for first team");
        player.career.push(CareerEntry {
            season: 2031,
            team_id: team_id.clone(),
            team_name: team_name.clone(),
            appearances: 33,
            goals: 7,
            assists: 4,
        });

        game.manager.career_history.push(ManagerCareerEntry {
            team_id,
            team_name,
            start_date: "2031-07-01".to_string(),
            end_date: Some("2032-06-30".to_string()),
            matches: 38,
            wins: 21,
            draws: 9,
            losses: 8,
            best_league_position: Some(2),
        });
        game.sync_user_manager_record();
    }

    fn serialized_history_snapshot(game: &Game) -> serde_json::Value {
        serde_json::json!({
            "teamHistory": game
                .teams
                .iter()
                .map(|team| serde_json::to_value(&team.history).unwrap())
                .collect::<Vec<_>>(),
            "playerCareerCounts": game
                .players
                .iter()
                .map(|player| player.career.len())
                .collect::<Vec<_>>(),
            "managerHistory": game
                .managers
                .iter()
                .map(|manager| serde_json::to_value(&manager.career_history).unwrap())
                .collect::<Vec<_>>(),
            "worldHistory": serde_json::to_value(&game.world_history).unwrap(),
        })
    }

    #[test]
    fn generate_past_world_history_populates_prior_records() {
        let mut game = make_game();

        generate_past_world_history(&mut game, 2032, 3);

        assert!(game.teams.iter().all(|team| team.history.len() == 3));
        assert!(game.players.iter().any(|player| !player.career.is_empty()));
        assert!(
            game.managers
                .iter()
                .any(|manager| !manager.career_history.is_empty())
        );
        assert_eq!(game.world_history.season_awards.len(), 3);
        assert!(!game.world_history.rivalries.is_empty());
    }

    #[test]
    fn generate_past_world_history_is_deterministic() {
        let mut left = make_game();
        let mut right = make_game();

        generate_past_world_history(&mut left, 2032, 4);
        generate_past_world_history(&mut right, 2032, 4);

        assert_eq!(
            serialized_history_snapshot(&left),
            serialized_history_snapshot(&right)
        );
    }

    #[test]
    fn generate_past_world_history_backfills_seeded_entries_in_chronological_order() {
        let mut left = make_game();
        let mut right = make_game();

        seed_later_history_entries(&mut left);
        seed_later_history_entries(&mut right);

        generate_past_world_history(&mut left, 2032, 4);
        generate_past_world_history(&mut right, 2032, 4);

        let team = &left.teams[0];
        assert!(
            team.history
                .windows(2)
                .all(|window| window[0].season <= window[1].season)
        );

        let player = left
            .players
            .iter()
            .find(|player| player.team_id.as_deref() == Some(team.id.as_str()))
            .expect("player history for seeded team");
        assert!(
            player
                .career
                .windows(2)
                .all(|window| window[0].season <= window[1].season)
        );

        let manager = left
            .managers
            .iter()
            .find(|manager| manager.id == left.manager.id)
            .expect("user manager in manager list");
        assert!(
            manager
                .career_history
                .windows(2)
                .all(|window| window[0].start_date <= window[1].start_date)
        );

        assert_eq!(
            serialized_history_snapshot(&left),
            serialized_history_snapshot(&right)
        );
    }
}
