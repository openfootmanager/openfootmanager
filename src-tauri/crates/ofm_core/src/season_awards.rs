use crate::game::Game;
use chrono::{Datelike, NaiveDate};
use domain::league::League;
use domain::manager::Manager;
use domain::player::{Player, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    pub player_of_year: Vec<AwardEntry>,   // Best avg rating over half the club's season
    pub clean_sheet_king: Vec<AwardEntry>, // Most clean sheets (GKs only)
    pub most_appearances: Vec<AwardEntry>,
    pub young_player: Vec<AwardEntry>, // Best avg rating, age <= 21
    pub manager_of_season: Vec<ManagerAwardEntry>,
}

/// A rating-based award goes to someone who played a real share of his club's
/// season: a handful of brilliant matches is not a season.
const AWARD_MIN_SHARE_OF_MATCHES: f64 = 0.5;
/// The least a rating-based award asks, however short the season.
const PLAYER_OF_YEAR_MIN_RATED_MATCHES: u32 = 5;
const YOUNG_PLAYER_MIN_RATED_MATCHES: u32 = 3;

struct PlayerAwardContext<'a> {
    player: &'a Player,
    team_id: String,
    team_name: String,
    age: i32,
    club_matches_played: u32,
}

impl PlayerAwardContext<'_> {
    fn has_rated_share_of_season(&self, floor: u32) -> bool {
        let share = (AWARD_MIN_SHARE_OF_MATCHES * self.club_matches_played as f64).ceil() as u32;
        crate::match_rating::is_rated(self.player.stats.avg_rating)
            && self.player.stats.rated_appearances >= share.max(floor)
    }
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

/// Team ids contesting a division, falling back to the standings for legacy
/// leagues that never recorded a participant list.
fn division_team_ids(division: &League) -> HashSet<String> {
    if !division.participant_ids.is_empty() {
        division.participant_ids.iter().cloned().collect()
    } else {
        division
            .standings
            .iter()
            .map(|standing| standing.team_id.clone())
            .collect()
    }
}

/// Matches each club has played. Inside a division that is its table; across
/// the world a club may sit in a league and several cups, and the league is
/// the competition it plays most matches in.
fn matches_played_by_club<'a>(
    game: &'a Game,
    division: Option<&'a League>,
) -> std::collections::HashMap<&'a str, u32> {
    let tables: Vec<&League> = match division {
        Some(division) => vec![division],
        None => game.competitions_in_play().iter().collect(),
    };
    let mut played = std::collections::HashMap::new();
    for standing in tables.iter().flat_map(|table| &table.standings) {
        let most = played.entry(standing.team_id.as_str()).or_insert(0);
        *most = (*most).max(standing.played);
    }
    played
}

fn build_player_award_contexts<'a>(
    game: &'a Game,
    scope: Option<&HashSet<String>>,
    division: Option<&League>,
) -> Vec<PlayerAwardContext<'a>> {
    let today = game.clock.current_date.date_naive();
    let matches_played_by_club = matches_played_by_club(game, division);

    game.players
        .iter()
        .filter(|player| player.stats.appearances > 0)
        .filter(|player| match scope {
            Some(team_ids) => player
                .team_id
                .as_ref()
                .is_some_and(|team_id| team_ids.contains(team_id)),
            None => true,
        })
        .map(|player| {
            let (team_id, team_name) = resolve_team_info(game, player);
            let club_matches_played = matches_played_by_club
                .get(team_id.as_str())
                .copied()
                .unwrap_or(0);

            PlayerAwardContext {
                player,
                team_id,
                team_name,
                age: player_age_on(&today, &player.date_of_birth),
                club_matches_played,
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

fn build_manager_award_contexts<'a>(
    game: &'a Game,
    division: Option<&League>,
) -> Vec<ManagerAwardContext<'a>> {
    let mut standings_by_team = std::collections::HashMap::new();
    let scope = division.map(division_team_ids);

    if let Some(league) = division.or(game.league.as_ref()) {
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
            if let Some(team_ids) = &scope
                && !team_ids.contains(&team_id)
            {
                return None;
            }
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
        .filter(|context| include(context))
        .map(|context| award_entry(context, value(context)))
        .collect();

    awards.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    awards.truncate(5);
    awards
}

/// Compute current season award standings from player stats, across the whole
/// game world (legacy behaviour, used for transfers and generated history).
pub fn compute_season_awards(game: &Game) -> SeasonAwards {
    compute_awards(game, None)
}

/// Compute award standings within a single division: only its participating
/// clubs' players and managers are eligible, and the manager award ranks by
/// that division's table.
pub fn compute_division_season_awards(game: &Game, division: &League) -> SeasonAwards {
    compute_awards(game, Some(division))
}

fn compute_awards(game: &Game, division: Option<&League>) -> SeasonAwards {
    let player_scope = division.map(division_team_ids);
    let contexts = build_player_award_contexts(game, player_scope.as_ref(), division);
    let manager_contexts = build_manager_award_contexts(game, division);

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

    // Player of the Year — best avg rating over a real share of the season
    let player_of_year = top_awards(
        &contexts,
        |context| context.has_rated_share_of_season(PLAYER_OF_YEAR_MIN_RATED_MATCHES),
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

    // Young Player of the Year — age <= 21, best avg rating over the same share
    let young_player = top_awards(
        &contexts,
        |context| {
            context.age <= 21 && context.has_rated_share_of_season(YOUNG_PLAYER_MIN_RATED_MATCHES)
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
    use super::{compute_division_season_awards, compute_season_awards};
    use chrono::{TimeZone, Utc};
    use domain::league::League;
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
    fn division_awards_only_consider_participating_clubs() {
        let team1 = make_team("team1", "Alpha FC");
        let team2 = make_team("team2", "Beta FC");
        let players = vec![
            make_player(
                "p1",
                "Division Scorer",
                Some("team1"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 10,
                    ..PlayerSeasonStats::default()
                },
            ),
            make_player(
                "p2",
                "Foreign Scorer",
                Some("team2"),
                Position::Forward,
                "2000-01-01",
                PlayerSeasonStats {
                    appearances: 8,
                    goals: 25,
                    ..PlayerSeasonStats::default()
                },
            ),
        ];
        let mut game = make_game(players, vec![team1, team2]);
        game.managers
            .push(make_manager("m1", "Division", "Boss", Some("team1")));
        game.managers
            .push(make_manager("m2", "Foreign", "Boss", Some("team2")));

        let division = League::new(
            "div1".to_string(),
            "Division One".to_string(),
            2025,
            &["team1".to_string()],
        );

        let awards = compute_division_season_awards(&game, &division);

        assert_eq!(
            awards.golden_boot.first().map(|e| e.player_id.as_str()),
            Some("p1"),
            "only the division's own scorer is eligible"
        );
        assert!(awards.golden_boot.iter().all(|e| e.team_id == "team1"));
        assert!(
            awards
                .manager_of_season
                .iter()
                .all(|e| e.team_id == "team1"),
            "manager award is scoped to the division"
        );

        // The unscoped computation still sees the whole world.
        let global = compute_season_awards(&game);
        assert_eq!(
            global.golden_boot.first().map(|e| e.player_id.as_str()),
            Some("p2")
        );
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
                    rated_appearances: 6,
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
                    rated_appearances: 5,
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
                    rated_appearances: 4,
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
                    rated_appearances: 2,
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
                    rated_appearances: 6,
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

    fn standing_after(team_id: &str, played: u32) -> domain::league::StandingEntry {
        let mut standing = domain::league::StandingEntry::new(team_id.to_string());
        standing.played = played;
        standing
    }

    fn rated_player(id: &str, dob: &str, rated_appearances: u32, avg_rating: f32) -> Player {
        make_player(
            id,
            id,
            Some("team1"),
            Position::Forward,
            dob,
            PlayerSeasonStats {
                appearances: rated_appearances,
                rated_appearances,
                avg_rating,
                ..PlayerSeasonStats::default()
            },
        )
    }

    fn division_awards(players: Vec<Player>, club_played: u32) -> super::SeasonAwards {
        let game = make_game(players, vec![make_team("team1", "Test FC")]);
        let division = League {
            standings: vec![standing_after("team1", club_played)],
            ..Default::default()
        };
        compute_division_season_awards(&game, &division)
    }

    /// Given a 38-match season and a forward who was brilliant in six matches,
    /// when awards are computed, then the regular starter wins Player of the Year.
    #[test]
    fn a_six_match_forward_cannot_win_player_of_the_year() {
        let cameo_star = rated_player("cameo-star", "1998-01-01", 6, 8.7);
        let regular = rated_player("regular", "1998-01-01", 35, 7.2);

        let awards = division_awards(vec![cameo_star, regular], 38);

        let ids: Vec<_> = awards
            .player_of_year
            .iter()
            .map(|e| e.player_id.as_str())
            .collect();
        assert_eq!(ids, vec!["regular"]);
    }

    /// Given a 38-match season, when a player has exactly half the club's matches rated
    /// and another one fewer, then only the first is eligible.
    #[test]
    fn half_of_the_clubs_matches_is_the_player_of_the_year_line() {
        let on_the_line = rated_player("on-the-line", "1998-01-01", 19, 7.0);
        let just_short = rated_player("just-short", "1998-01-01", 18, 9.0);

        let awards = division_awards(vec![on_the_line, just_short], 38);

        let ids: Vec<_> = awards
            .player_of_year
            .iter()
            .map(|e| e.player_id.as_str())
            .collect();
        assert_eq!(ids, vec!["on-the-line"]);
    }

    /// Given a world-wide ranking and a club that sits in a league and a short cup table, when
    /// awards are computed without a division, then its league matches set the threshold.
    #[test]
    fn world_awards_measure_a_club_by_its_league_matches() {
        let mut game = make_game(
            vec![
                rated_player("cameo-star", "1998-01-01", 6, 8.7),
                rated_player("regular", "1998-01-01", 35, 7.2),
            ],
            vec![make_team("team1", "Test FC")],
        );
        game.competitions = vec![
            League {
                standings: vec![standing_after("team1", 38)],
                ..Default::default()
            },
            League {
                standings: vec![standing_after("team1", 6)],
                ..Default::default()
            },
        ];

        let awards = compute_season_awards(&game);

        let ids: Vec<_> = awards
            .player_of_year
            .iter()
            .map(|e| e.player_id.as_str())
            .collect();
        assert_eq!(ids, vec!["regular"]);
    }

    /// Given a stale legacy league mirror that disagrees with the competitions, when world awards
    /// are computed, then the competitions are the source of the club's matches played.
    #[test]
    fn world_awards_ignore_a_stale_legacy_league_mirror() {
        let mut game = make_game(
            vec![rated_player("regular", "1998-01-01", 19, 7.2)],
            vec![make_team("team1", "Test FC")],
        );
        game.competitions = vec![League {
            standings: vec![standing_after("team1", 38)],
            ..Default::default()
        }];
        game.league = Some(League {
            standings: vec![standing_after("team1", 60)],
            ..Default::default()
        });

        let awards = compute_season_awards(&game);

        assert_eq!(
            awards.player_of_year.len(),
            1,
            "19 of 38 is half the season"
        );
    }

    /// Given a club with no table anywhere, when world awards are computed, then the
    /// absolute floor is all that is asked of its players.
    #[test]
    fn a_club_without_a_table_falls_back_to_the_floor() {
        let game = make_game(
            vec![rated_player("tableless", "1998-01-01", 5, 7.0)],
            vec![make_team("team1", "Test FC")],
        );

        let awards = compute_season_awards(&game);

        assert_eq!(awards.player_of_year.len(), 1);
    }

    /// Given a season so short that half the matches is under the absolute floor,
    /// when awards are computed, then the floor decides.
    #[test]
    fn a_short_season_keeps_the_absolute_floor() {
        let at_floor = rated_player("at-floor", "1998-01-01", 5, 7.0);
        let under_floor = rated_player("under-floor", "1998-01-01", 4, 9.0);

        let awards = division_awards(vec![at_floor, under_floor], 6);

        let ids: Vec<_> = awards
            .player_of_year
            .iter()
            .map(|e| e.player_id.as_str())
            .collect();
        assert_eq!(ids, vec!["at-floor"]);
    }

    /// Given a young forward with a few brilliant matches, when awards are computed in a
    /// full season, then Young Player of the Year needs the same share of matches.
    #[test]
    fn young_player_of_the_year_needs_the_same_share_of_matches() {
        let cameo_prospect = rated_player("cameo-prospect", "2006-01-01", 3, 9.0);
        let regular_prospect = rated_player("regular-prospect", "2006-01-01", 25, 6.8);

        let awards = division_awards(vec![cameo_prospect, regular_prospect], 38);

        let ids: Vec<_> = awards
            .young_player
            .iter()
            .map(|e| e.player_id.as_str())
            .collect();
        assert_eq!(ids, vec!["regular-prospect"]);
    }

    /// Given a player with many appearances but few of them rated, when awards are computed,
    /// then only the rated ones count towards the threshold.
    #[test]
    fn unrated_appearances_do_not_count_towards_the_threshold() {
        let mut mostly_unrated = rated_player("mostly-unrated", "1998-01-01", 3, 8.9);
        mostly_unrated.stats.appearances = 30;

        let awards = division_awards(vec![mostly_unrated], 38);

        assert!(awards.player_of_year.is_empty());
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
                rated_appearances: 30,
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
                rated_appearances: 28,
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
    /// Given unrated player seasons, when world and division awards are computed,
    /// then both rating awards are absent and counting awards survive.
    #[test]
    fn unrated_seasons_keep_counting_awards_without_rating_awards() {
        let players = vec![
            make_player(
                "scorer",
                "Scorer",
                Some("team1"),
                Position::Forward,
                "2006-01-01",
                PlayerSeasonStats {
                    appearances: 10,
                    goals: 4,
                    assists: 3,
                    ..Default::default()
                },
            ),
            make_player(
                "keeper",
                "Keeper",
                Some("team1"),
                Position::Goalkeeper,
                "1995-01-01",
                PlayerSeasonStats {
                    appearances: 10,
                    clean_sheets: 5,
                    ..Default::default()
                },
            ),
        ];
        let game = make_game(players, vec![make_team("team1", "One FC")]);
        let division = League {
            standings: vec![domain::league::StandingEntry::new("team1".to_string())],
            ..Default::default()
        };
        for awards in [
            compute_season_awards(&game),
            compute_division_season_awards(&game, &division),
        ] {
            assert!(awards.player_of_year.is_empty());
            assert!(awards.young_player.is_empty());
            assert_eq!(awards.golden_boot[0].player_id, "scorer");
            assert_eq!(awards.assist_king[0].player_id, "scorer");
            assert_eq!(awards.clean_sheet_king[0].player_id, "keeper");
            assert_eq!(awards.most_appearances.len(), 2);
        }
    }

    /// Given a non-finite imported average, when world or division awards are computed,
    /// then the unavailable rating cannot win either rating award.
    #[test]
    fn non_finite_unrated_averages_cannot_win_rating_awards() {
        let player = make_player(
            "invalid",
            "Invalid",
            Some("team1"),
            Position::Forward,
            "2006-01-01",
            PlayerSeasonStats {
                appearances: 10,
                avg_rating: f32::INFINITY,
                ..Default::default()
            },
        );
        let game = make_game(vec![player], vec![make_team("team1", "One FC")]);
        let division = League {
            standings: vec![domain::league::StandingEntry::new("team1".to_string())],
            ..Default::default()
        };
        for awards in [
            compute_season_awards(&game),
            compute_division_season_awards(&game, &division),
        ] {
            assert!(awards.player_of_year.is_empty());
            assert!(awards.young_player.is_empty());
        }
    }
}
