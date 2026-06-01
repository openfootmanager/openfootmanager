use crate::game::Game;
use chrono::{Datelike, NaiveDate};
use domain::manager::Manager;
use domain::player::{Player, Position};
use serde::{Deserialize, Serialize};

/// A single award entry (player + stat value).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AwardEntry {
    pub player_id: String,
    pub player_name: String,
    pub team_id: String,
    pub team_name: String,
    pub value: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManagerAwardEntry {
    pub manager_id: String,
    pub manager_name: String,
    pub team_id: String,
    pub team_name: String,
    pub value: f64,
    pub win_rate: f64,
}

/// Season award standings — top 5 in each category.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonAwards {
    pub golden_boot: Vec<AwardEntry>,      // Top scorers
    pub assist_king: Vec<AwardEntry>,      // Top assists
    pub player_of_year: Vec<AwardEntry>,   // Best avg rating (min 5 apps)
    pub clean_sheet_king: Vec<AwardEntry>, // Most clean sheets (GKs only)
    pub most_appearances: Vec<AwardEntry>,
    pub young_player: Vec<AwardEntry>, // Best avg rating, age <= 21
    pub manager_of_season: Vec<ManagerAwardEntry>,
}

struct PlayerAwardContext<'a> {
    player: &'a Player,
    team_id: String,
    team_name: String,
    age: i32,
}

struct ManagerAwardContext<'a> {
    manager: &'a Manager,
    team_id: String,
    team_name: String,
    league_position: u32,
    points: u32,
    win_rate: f64,
}

fn free_agent_team_name() -> String {
    ["Free", "Agent"].join(" ")
}

fn player_age_on(today: &NaiveDate, date_of_birth: &str) -> i32 {
    if let Ok(dob) = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d") {
        let mut age = today.year() - dob.year();
        if today.ordinal() < dob.ordinal() {
            age -= 1;
        }
        age
    } else {
        30
    }
}

fn resolve_team_info(game: &Game, player: &Player) -> (String, String) {
    let team_name = player
        .team_id
        .as_ref()
        .and_then(|team_id| game.teams.iter().find(|team| &team.id == team_id))
        .map(|team| team.name.clone())
        .unwrap_or_else(free_agent_team_name);
    let team_id = player.team_id.clone().unwrap_or_default();

    (team_id, team_name)
}

fn build_player_award_contexts(game: &Game) -> Vec<PlayerAwardContext<'_>> {
    let today = game.clock.current_date.date_naive();

    game.players
        .iter()
        .filter(|player| !player.retired && player.stats.appearances > 0)
        .map(|player| {
            let (team_id, team_name) = resolve_team_info(game, player);

            PlayerAwardContext {
                player,
                team_id,
                team_name,
                age: player_age_on(&today, &player.date_of_birth),
            }
        })
        .collect()
}

fn award_entry<'a>(context: &PlayerAwardContext<'a>, value: f64) -> AwardEntry {
    AwardEntry {
        player_id: context.player.id.clone(),
        player_name: context.player.match_name.clone(),
        team_id: context.team_id.clone(),
        team_name: context.team_name.clone(),
        value,
    }
}

fn manager_award_entry<'a>(context: &ManagerAwardContext<'a>) -> ManagerAwardEntry {
    ManagerAwardEntry {
        manager_id: context.manager.id.clone(),
        manager_name: context.manager.full_name(),
        team_id: context.team_id.clone(),
        team_name: context.team_name.clone(),
        value: context.points as f64,
        win_rate: context.win_rate,
    }
}

fn build_manager_award_contexts(game: &Game) -> Vec<ManagerAwardContext<'_>> {
    let mut standings_by_team = std::collections::HashMap::new();

    if let Some(league) = &game.league {
        for (index, standing) in league.sorted_standings().into_iter().enumerate() {
            standings_by_team.insert(
                standing.team_id.clone(),
                (
                    (index + 1) as u32,
                    standing.points,
                    standing.played,
                    standing.won,
                ),
            );
        }
    }

    game.managers
        .iter()
        .filter_map(|manager| {
            let team_id = manager.team_id.clone()?;
            let team = game.teams.iter().find(|team| team.id == team_id)?;
            let (league_position, points, matches_played, wins) =
                if let Some((league_position, points, matches_played, wins)) =
                    standings_by_team.get(&team_id)
                {
                    (*league_position, *points, *matches_played, *wins)
                } else {
                    let latest_record = team.history.iter().max_by_key(|record| record.season)?;
                    let points = latest_record.won * 3 + latest_record.drawn;
                    (
                        latest_record.league_position,
                        points,
                        latest_record.played,
                        latest_record.won,
                    )
                };
            let win_rate = if matches_played == 0 {
                0.0
            } else {
                (wins as f64 / matches_played as f64) * 100.0
            };

            Some(ManagerAwardContext {
                manager,
                team_id,
                team_name: team.name.clone(),
                league_position,
                points,
                win_rate,
            })
        })
        .collect()
}

fn top_manager_awards(contexts: &[ManagerAwardContext<'_>]) -> Vec<ManagerAwardEntry> {
    let mut awards: Vec<_> = contexts.iter().map(manager_award_entry).collect();

    awards.sort_by(|left, right| {
        let left_position = contexts
            .iter()
            .find(|context| context.manager.id == left.manager_id)
            .map(|context| context.league_position)
            .unwrap_or(u32::MAX);
        let right_position = contexts
            .iter()
            .find(|context| context.manager.id == right.manager_id)
            .map(|context| context.league_position)
            .unwrap_or(u32::MAX);

        left_position
            .cmp(&right_position)
            .then_with(|| {
                right
                    .value
                    .partial_cmp(&left.value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                right
                    .win_rate
                    .partial_cmp(&left.win_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.manager_name.cmp(&right.manager_name))
    });
    awards.truncate(5);
    awards
}

fn top_awards<'a, F, G>(
    contexts: &[PlayerAwardContext<'a>],
    include: F,
    value: G,
) -> Vec<AwardEntry>
where
    F: Fn(&PlayerAwardContext<'a>) -> bool,
    G: Fn(&PlayerAwardContext<'a>) -> f64,
{
    let mut awards: Vec<_> = contexts
        .iter()
        .filter_map(|context| include(context).then(|| award_entry(context, value(context))))
        .collect();

    awards.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    awards.truncate(5);
    awards
}

/// Compute current season award standings from player stats.
pub fn compute_season_awards(game: &Game) -> SeasonAwards {
    let contexts = build_player_award_contexts(game);
    let manager_contexts = build_manager_award_contexts(game);

    // Golden Boot — top scorers
    let golden_boot = top_awards(
        &contexts,
        |context| context.player.stats.goals > 0,
        |context| context.player.stats.goals as f64,
    );

    // Assist King
    let assist_king = top_awards(
        &contexts,
        |context| context.player.stats.assists > 0,
        |context| context.player.stats.assists as f64,
    );

    // Player of the Year — best avg rating, min 5 appearances
    let player_of_year = top_awards(
        &contexts,
        |context| context.player.stats.appearances >= 5 && context.player.stats.avg_rating > 0.0,
        |context| context.player.stats.avg_rating as f64,
    );

    // Clean Sheet King — GKs only
    let clean_sheet_king = top_awards(
        &contexts,
        |context| {
            context.player.position == Position::Goalkeeper && context.player.stats.clean_sheets > 0
        },
        |context| context.player.stats.clean_sheets as f64,
    );

    // Most Appearances
    let most_appearances = top_awards(
        &contexts,
        |_| true,
        |context| context.player.stats.appearances as f64,
    );

    // Young Player of the Year — age <= 21, best avg rating, min 3 apps
    let young_player = top_awards(
        &contexts,
        |context| {
            context.age <= 21
                && context.player.stats.appearances >= 3
                && context.player.stats.avg_rating > 0.0
        },
        |context| context.player.stats.avg_rating as f64,
    );
    let manager_of_season = top_manager_awards(&manager_contexts);

    SeasonAwards {
        golden_boot,
        assist_king,
        player_of_year,
        clean_sheet_king,
        most_appearances,
        young_player,
        manager_of_season,
    }
}

#[cfg(test)]
mod tests {
    use super::compute_season_awards;
    use chrono::{TimeZone, Utc};
    use domain::manager::{Manager, ManagerCareerStats};
    use domain::player::{Player, PlayerAttributes, PlayerSeasonStats, Position};
    use domain::team::Team;

    use crate::clock::GameClock;
    use crate::game::Game;

    fn default_attrs() -> PlayerAttributes {
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
            handling: 60,
            reflexes: 60,
            aerial: 60,
        }
    }

    fn make_team(id: &str, name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "England".to_string(),
            "Testville".to_string(),
            "Test Ground".to_string(),
            20_000,
        )
    }

    fn make_player(
        id: &str,
        name: &str,
        team_id: Option<&str>,
        position: Position,
        dob: &str,
        stats: PlayerSeasonStats,
    ) -> Player {
        let mut player = Player::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            dob.to_string(),
            "England".to_string(),
            position,
            default_attrs(),
        );
        player.team_id = team_id.map(str::to_string);
        player.stats = stats;
        player
    }

    fn make_game(players: Vec<Player>, teams: Vec<Team>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );

        Game::new(clock, manager, teams, players, vec![], vec![])
    }

    fn make_manager(id: &str, first_name: &str, last_name: &str, team_id: Option<&str>) -> Manager {
        let mut manager = Manager::new(
            id.to_string(),
            first_name.to_string(),
            last_name.to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.team_id = team_id.map(str::to_string);
        manager.career_stats = ManagerCareerStats {
            matches_managed: 38,
            wins: 25,
            draws: 8,
            losses: 5,
            trophies: 1,
            best_finish: Some(1),
        };
        manager
    }

    #[test]
    fn golden_boot_is_sorted_descending_and_limited_to_top_five() {
        let team = make_team("team1", "Test FC");
        let mut players = vec![
            make_player(
                "p1",
                "Player 1",
                Some("team1"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 4,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "p2",
                "Player 2",
                Some("team1"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 6,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "p3",
                "Player 3",
                Some("team1"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 1,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "p4",
                "Player 4",
                Some("team1"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 5,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "p5",
                "Player 5",
                Some("team1"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 2,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "p6",
                "Player 6",
                Some("team1"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 3,
                    ..PlayerSeasonStats::default()
                },
            ),
        ];
        players.push(make_player(
            "p7",
            "Zero Apps",
            Some("team1"),
            Position::Forward,
            "2000-01-01",
            PlayerSeasonStats {
                appearances: 0,
                goals: 99,
                ..PlayerSeasonStats::default()
            },
        ));

        let awards = compute_season_awards(&make_game(players, vec![team]));

        let top_ids: Vec<_> = awards
            .golden_boot
            .iter()
            .map(|entry| entry.player_id.as_str())
            .collect();
        assert_eq!(top_ids, vec!["p2", "p4", "p1", "p6", "p5"]);
        assert_eq!(awards.golden_boot.len(), 5);
        assert!(
            awards
                .golden_boot
                .iter()
                .all(|entry| entry.player_name != "Zero Apps")
        );
    }

    #[test]
    fn player_of_year_and_young_player_apply_their_thresholds_and_age_rules() {
        let team = make_team("team1", "Test FC");
        let players = vec![
            make_player(
                "older-star",
                "Older Star",
                Some("team1"),
                Position::Midfielder,
                "2001-02-10",
                PlayerSeasonStats {
                    appearances: 6,
                    avg_rating: 8.4,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "young-eligible",
                "Young Eligible",
                Some("team1"),
                Position::Forward,
                "2004-06-15",
                PlayerSeasonStats {
                    appearances: 5,
                    avg_rating: 7.8,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "young-four-apps",
                "Young Four Apps",
                Some("team1"),
                Position::Forward,
                "2004-09-10",
                PlayerSeasonStats {
                    appearances: 4,
                    avg_rating: 9.0,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "young-low-apps",
                "Young Low Apps",
                Some("team1"),
                Position::Forward,
                "2005-03-10",
                PlayerSeasonStats {
                    appearances: 2,
                    avg_rating: 9.5,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "invalid-dob",
                "Invalid DOB",
                Some("team1"),
                Position::Midfielder,
                "unknown",
                PlayerSeasonStats {
                    appearances: 6,
                    avg_rating: 8.1,
                    ..PlayerSeasonStats::default()
                },
            ),
        ];

        let awards = compute_season_awards(&make_game(players, vec![team]));

        let player_of_year_ids: Vec<_> = awards
            .player_of_year
            .iter()
            .map(|entry| entry.player_id.as_str())
            .collect();
        assert_eq!(
            player_of_year_ids,
            vec!["older-star", "invalid-dob", "young-eligible"]
        );

        let young_player_ids: Vec<_> = awards
            .young_player
            .iter()
            .map(|entry| entry.player_id.as_str())
            .collect();
        assert_eq!(young_player_ids, vec!["young-four-apps", "young-eligible"]);
    }

    #[test]
    fn retired_players_are_excluded_from_award_tables() {
        let team = make_team("team1", "Test FC");
        let mut retired_star = make_player(
            "retired-star",
            "Retired Star",
            Some("team1"),
            Position::Forward,
            "1990-01-01",
            PlayerSeasonStats {
                appearances: 30,
                goals: 25,
                avg_rating: 8.5,
                ..PlayerSeasonStats::default()
            },
        );
        retired_star.retired = true;

        let active_player = make_player(
            "active-star",
            "Active Star",
            Some("team1"),
            Position::Forward,
            "2000-01-01",
            PlayerSeasonStats {
                appearances: 28,
                goals: 18,
                avg_rating: 7.8,
                ..PlayerSeasonStats::default()
            },
        );

        let awards =
            compute_season_awards(&make_game(vec![retired_star, active_player], vec![team]));

        assert_eq!(awards.golden_boot[0].player_id, "active-star");
        assert_eq!(awards.player_of_year[0].player_id, "active-star");
        assert!(
            awards
                .golden_boot
                .iter()
                .all(|entry| entry.player_id != "retired-star")
        );
    }

    #[test]
    fn manager_of_season_prefers_title_winning_manager() {
        let mut alpha = make_team("team1", "Alpha FC");
        alpha.history.push(domain::team::TeamSeasonRecord {
            season: 2025,
            league_position: 1,
            played: 38,
            won: 25,
            drawn: 8,
            lost: 5,
            goals_for: 72,
            goals_against: 30,
        });
        alpha.manager_id = Some("mgr1".to_string());

        let mut beta = make_team("team2", "Beta FC");
        beta.history.push(domain::team::TeamSeasonRecord {
            season: 2025,
            league_position: 2,
            played: 38,
            won: 22,
            drawn: 9,
            lost: 7,
            goals_for: 65,
            goals_against: 35,
        });
        beta.manager_id = Some("mgr2".to_string());

        let mut game = make_game(vec![], vec![alpha, beta]);
        game.managers = vec![
            make_manager("mgr1", "Alex", "Winner", Some("team1")),
            make_manager("mgr2", "Ben", "Runner", Some("team2")),
        ];
        game.manager = game.managers[0].clone();

        let awards = compute_season_awards(&game);

        assert_eq!(awards.manager_of_season[0].manager_name, "Alex Winner");
        assert_eq!(awards.manager_of_season[0].team_name, "Alpha FC");
    }

    #[test]
    fn manager_of_season_uses_current_season_win_rate() {
        let mut alpha = make_team("team1", "Alpha FC");
        alpha.history.push(domain::team::TeamSeasonRecord {
            season: 2025,
            league_position: 1,
            played: 20,
            won: 10,
            drawn: 5,
            lost: 5,
            goals_for: 30,
            goals_against: 18,
        });
        alpha.manager_id = Some("mgr1".to_string());

        let mut game = make_game(vec![], vec![alpha]);
        let mut manager = make_manager("mgr1", "Alex", "Winner", Some("team1"));
        manager.career_stats = ManagerCareerStats {
            matches_managed: 120,
            wins: 90,
            draws: 15,
            losses: 15,
            trophies: 3,
            best_finish: Some(1),
        };
        game.managers = vec![manager.clone()];
        game.manager = manager;

        let awards = compute_season_awards(&game);

        assert_eq!(awards.manager_of_season[0].win_rate, 50.0);
    }

    #[test]
    fn clean_sheet_king_only_counts_goalkeepers_and_uses_free_agent_fallback() {
        let team = make_team("team1", "Test FC");
        let players = vec![
            make_player(
                "team-gk",
                "Team Keeper",
                Some("team1"),
                Position::Goalkeeper,
                "1998-01-01",
                PlayerSeasonStats {
                    appearances: 10,
                    clean_sheets: 8,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "free-agent-gk",
                "Free Agent Keeper",
                None,
                Position::Goalkeeper,
                "1996-01-01",
                PlayerSeasonStats {
                    appearances: 9,
                    clean_sheets: 9,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "defender",
                "Defender",
                Some("team1"),
                Position::Defender,
                "1999-01-01",
                PlayerSeasonStats {
                    appearances: 12,
                    clean_sheets: 12,
                    ..PlayerSeasonStats::default()
                },
            ),
        ];

        let awards = compute_season_awards(&make_game(players, vec![team]));

        assert_eq!(awards.clean_sheet_king.len(), 2);
        assert_eq!(awards.clean_sheet_king[0].player_id, "free-agent-gk");
        assert_eq!(awards.clean_sheet_king[0].team_id, "");
        assert_eq!(awards.clean_sheet_king[0].team_name, "Free Agent");
        assert!(
            awards
                .clean_sheet_king
                .iter()
                .all(|entry| entry.player_id != "defender")
        );
    }
}
