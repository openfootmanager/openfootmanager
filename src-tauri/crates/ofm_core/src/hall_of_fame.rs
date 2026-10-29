//! The retired legends of the world, read from the archive of players who have stopped playing, and
//! the champions of its past seasons.

use std::collections::{HashMap, HashSet};

use domain::player::Position;
use domain::retired_player::RetiredPlayer;
use domain::team::TeamSeasonRecord;
use serde::Serialize;

use crate::end_of_season::ladder_tiers;
use crate::game::Game;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HallOfFameLegend {
    pub player_id: String,
    pub full_name: String,
    pub nationality: String,
    pub football_nation: String,
    /// His natural position, the one the squad screens name him by.
    pub position: Position,
    pub appearances: u32,
    pub goals: u32,
    pub assists: u32,
    /// Seasons he spent at a club that finished first that season.
    pub titles: u32,
    pub last_club_name: Option<String>,
    pub final_season: Option<u32>,
}

pub fn hall_of_fame_legends(game: &Game) -> Vec<HallOfFameLegend> {
    let championships = championship_seasons_by_club(game);
    let mut legends: Vec<HallOfFameLegend> = game
        .retired_players
        .iter()
        .filter(|retiree| !retiree.career.is_empty())
        .map(|retiree| legend_of(retiree, &championships))
        .collect();
    legends.sort_by(|left, right| {
        right
            .titles
            .cmp(&left.titles)
            .then(right.appearances.cmp(&left.appearances))
            .then(right.goals.cmp(&left.goals))
            .then_with(|| left.full_name.cmp(&right.full_name))
    });
    legends
}

/// Whether a club's season record is a league title: first place in the top flight of its country.
///
/// A second division's winner is promoted, not crowned. The tier is read off the record's
/// competition, so it follows the ladder as it is today. A record that cannot be placed on the
/// ladder (a blank id, from a save made before records named their competition, or a competition
/// that has since left the ladder) keeps the old reading, first place, so a past title is never
/// erased; the cost is that such a record cannot tell a second-division win from a title.
fn is_league_title(tiers: &HashMap<&str, u32>, record: &TeamSeasonRecord) -> bool {
    record.league_position == 1
        && tiers
            .get(record.competition_id.as_str())
            .is_none_or(|tier| *tier == 0)
}

fn championship_seasons_by_club(game: &Game) -> HashMap<&str, HashSet<u32>> {
    let tiers = ladder_tiers(game);
    game.teams
        .iter()
        .map(|team| {
            let seasons = team
                .history
                .iter()
                .filter(|record| is_league_title(&tiers, record))
                .map(|record| record.season)
                .collect();
            (team.id.as_str(), seasons)
        })
        .collect()
}

/// A club that won a league title, and the season record that says so.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PastChampion {
    pub season: u32,
    pub team_id: String,
    pub record: TeamSeasonRecord,
}

/// Every league title in the world's past, the latest season first, then the more wins, then the
/// club's name.
pub fn hall_of_fame_champions(game: &Game) -> Vec<PastChampion> {
    let tiers = ladder_tiers(game);
    let mut champions: Vec<(&str, PastChampion)> = game
        .teams
        .iter()
        .flat_map(|team| {
            let tiers = &tiers;
            team.history
                .iter()
                .filter(move |record| is_league_title(tiers, record))
                .map(move |record| {
                    (
                        team.name.as_str(),
                        PastChampion {
                            season: record.season,
                            team_id: team.id.clone(),
                            record: record.clone(),
                        },
                    )
                })
        })
        .collect();
    champions.sort_by(|(left_name, left), (right_name, right)| {
        right
            .season
            .cmp(&left.season)
            .then(right.record.won.cmp(&left.record.won))
            .then_with(|| left_name.cmp(right_name))
    });
    champions
        .into_iter()
        .map(|(_, champion)| champion)
        .collect()
}

fn legend_of(
    retiree: &RetiredPlayer,
    championships: &HashMap<&str, HashSet<u32>>,
) -> HallOfFameLegend {
    let latest = retiree.career.iter().max_by_key(|entry| entry.season);
    let titles = retiree
        .career
        .iter()
        .filter(|entry| {
            championships
                .get(entry.team_id.as_str())
                .is_some_and(|seasons| seasons.contains(&entry.season))
        })
        .count() as u32;
    HallOfFameLegend {
        player_id: retiree.id.clone(),
        full_name: retiree.full_name.clone(),
        nationality: retiree.nationality.clone(),
        football_nation: retiree.football_nation.clone(),
        position: retiree.natural_position.clone(),
        appearances: retiree.career.iter().map(|entry| entry.appearances).sum(),
        goals: retiree.career.iter().map(|entry| entry.goals).sum(),
        assists: retiree.career.iter().map(|entry| entry.assists).sum(),
        titles,
        last_club_name: latest.map(|entry| entry.team_name.clone()),
        final_season: latest.map(|entry| entry.season),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::player::{CareerEntry, Player, PlayerAttributes};
    use domain::retired_player::RetiredPlayer;
    use domain::team::{Team, TeamSeasonRecord};

    use crate::clock::GameClock;

    fn attributes() -> PlayerAttributes {
        let value = 60;
        PlayerAttributes {
            pace: value,
            stamina: value,
            strength: value,
            agility: value,
            passing: value,
            shooting: value,
            tackling: value,
            dribbling: value,
            defending: value,
            positioning: value,
            vision: value,
            decisions: value,
            composure: value,
            aggression: value,
            teamwork: value,
            leadership: value,
            handling: value,
            reflexes: value,
            aerial: value,
        }
    }

    fn champion_team(id: &str, name: &str, title_seasons: &[u32]) -> Team {
        let mut team = Team::new(
            id.into(),
            name.into(),
            name.into(),
            "ENG".into(),
            "Town".into(),
            "Ground".into(),
            20_000,
        );
        for season in title_seasons {
            team.history.push(TeamSeasonRecord {
                season: *season,
                league_position: 1,
                played: 38,
                won: 30,
                drawn: 4,
                lost: 4,
                goals_for: 80,
                goals_against: 30,
                competition_id: String::new(),
            });
        }
        team
    }

    fn career(season: u32, team_id: &str, team_name: &str, apps: u32, goals: u32) -> CareerEntry {
        CareerEntry {
            season,
            team_id: team_id.into(),
            team_name: team_name.into(),
            appearances: apps,
            goals,
            assists: goals / 2,
        }
    }

    fn veteran(id: &str, name: &str, career_entries: Vec<CareerEntry>) -> Player {
        let mut player = Player::new(
            id.into(),
            name.into(),
            name.into(),
            "1988-01-01".into(),
            "England".into(),
            Position::Forward,
            attributes(),
        );
        player.career = career_entries;
        player.retired = true;
        player
    }

    fn game_with(players: Vec<Player>, teams: Vec<Team>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2040, 6, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".into(),
            "Alex".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        let (retired, active): (Vec<Player>, Vec<Player>) =
            players.into_iter().partition(|player| player.retired);
        let mut game = Game::new(clock, manager, teams, active, vec![], vec![]);
        game.retired_players = retired.iter().map(archive).collect();
        game
    }

    fn archive(player: &Player) -> RetiredPlayer {
        RetiredPlayer::from_player(player, "2039-06-30", 2039)
    }

    /// Given a retiree who has left the roster for the archive, when legends are listed,
    /// then he is one, with his career totals and last club.
    #[test]
    fn an_archived_retiree_is_a_legend() {
        let vale = veteran(
            "p-vale",
            "Victor Vale",
            vec![
                career(1, "t-a", "Alpha FC", 200, 100),
                career(2, "t-b", "Beta United", 90, 20),
            ],
        );
        let mut game = game_with(vec![], vec![]);
        game.retired_players.push(archive(&vale));

        let legends = hall_of_fame_legends(&game);

        assert_eq!(legends.len(), 1);
        let legend = &legends[0];
        assert_eq!(legend.player_id, "p-vale");
        assert_eq!(legend.full_name, "Victor Vale");
        assert_eq!(
            (legend.appearances, legend.goals, legend.assists),
            (290, 120, 60)
        );
        assert_eq!(legend.last_club_name.as_deref(), Some("Beta United"));
        assert_eq!(legend.final_season, Some(2));
    }

    /// Given active players and a retiree with no career, when legends are listed, then
    /// neither is a legend.
    #[test]
    fn active_players_and_careerless_retirees_are_not_legends() {
        let mut active = veteran(
            "p-active",
            "Active",
            vec![career(1, "t-a", "Alpha FC", 10, 2)],
        );
        active.retired = false;
        let careerless = veteran("p-none", "No Career", vec![]);
        let game = game_with(vec![active, careerless], vec![]);

        assert!(hall_of_fame_legends(&game).is_empty());
    }

    /// Given a club that won the league in 2 of the 3 seasons he played there, when legends are
    /// listed, then he has two titles.
    #[test]
    fn titles_count_the_seasons_his_club_finished_first() {
        let vale = veteran(
            "p-vale",
            "Victor Vale",
            vec![
                career(1, "t-a", "Alpha FC", 30, 10),
                career(2, "t-a", "Alpha FC", 30, 10),
                career(3, "t-a", "Alpha FC", 30, 10),
                career(4, "t-b", "Beta United", 30, 10),
            ],
        );
        let teams = vec![
            champion_team("t-a", "Alpha FC", &[1, 3]),
            champion_team("t-b", "Beta United", &[9]),
        ];
        let game = game_with(vec![vale], teams);

        assert_eq!(hall_of_fame_legends(&game)[0].titles, 2);
    }

    /// Given three legends, when listed, then more titles come first, then appearances, goals,
    /// and name.
    #[test]
    fn legends_are_ranked_by_titles_then_appearances_then_goals_then_name() {
        let champion = veteran(
            "p-1",
            "Zed Champion",
            vec![career(1, "t-a", "Alpha FC", 10, 1)],
        );
        let busy = veteran(
            "p-2",
            "Bea Busy",
            vec![career(5, "t-b", "Beta United", 300, 1)],
        );
        let scorer = veteran(
            "p-3",
            "Ann Scorer",
            vec![career(5, "t-b", "Beta United", 100, 50)],
        );
        let twin = veteran(
            "p-4",
            "Abe Twin",
            vec![career(5, "t-b", "Beta United", 100, 50)],
        );
        let teams = vec![
            champion_team("t-a", "Alpha FC", &[1]),
            champion_team("t-b", "Beta United", &[]),
        ];
        let game = game_with(vec![busy, scorer, champion, twin], teams);

        let order: Vec<_> = hall_of_fame_legends(&game)
            .into_iter()
            .map(|legend| legend.player_id)
            .collect();

        assert_eq!(order, vec!["p-1", "p-2", "p-4", "p-3"]);
    }

    /// Given retirees whose coarse position differs from their natural one, whether archived or
    /// still on the roster, when legends are listed, then both carry the natural position.
    #[test]
    fn a_legend_carries_his_natural_position() {
        let mut archived = veteran(
            "p-archived",
            "Ada Archived",
            vec![career(1, "t-a", "Alpha FC", 10, 1)],
        );
        archived.natural_position = Position::LeftBack;
        archived.position = Position::Defender;
        let mut rostered = veteran(
            "p-roster",
            "Bo Roster",
            vec![career(1, "t-a", "Alpha FC", 10, 1)],
        );
        rostered.natural_position = Position::Striker;
        rostered.position = Position::Forward;
        let mut game = game_with(vec![rostered], vec![]);
        game.retired_players.push(archive(&archived));

        let positions: HashMap<String, Position> = hall_of_fame_legends(&game)
            .into_iter()
            .map(|legend| (legend.player_id, legend.position))
            .collect();

        assert_eq!(positions["p-archived"], Position::LeftBack);
        assert_eq!(positions["p-roster"], Position::Striker);
    }

    /// A country's two divisions, "div-1" the top flight (priority 1) and "div-2" under it.
    fn english_ladder(game: &mut Game) {
        for (id, priority) in [("div-1", 1), ("div-2", 2)] {
            let mut league = League::new(id.into(), id.into(), 2039, &[]);
            league.country_id = Some("ENG".into());
            league.priority = priority;
            game.competitions.push(league);
        }
    }

    fn won_in(team: &mut Team, season: u32, competition_id: &str, won: u32) {
        team.history.push(TeamSeasonRecord {
            season,
            league_position: 1,
            played: 38,
            won,
            drawn: 4,
            lost: 4,
            goals_for: 80,
            goals_against: 30,
            competition_id: competition_id.into(),
        });
    }

    fn champions_of(game: &Game) -> Vec<(u32, String)> {
        hall_of_fame_champions(game)
            .into_iter()
            .map(|champion| (champion.season, champion.team_id))
            .collect()
    }

    /// Given a club that finished first in the second division and another in the top flight,
    /// When the world's champions are listed,
    /// Then only the top-flight club is one: promotion is not a title.
    #[test]
    fn a_second_division_winner_is_not_a_champion() {
        let mut top = champion_team("top", "Top FC", &[]);
        won_in(&mut top, 2038, "div-1", 30);
        let mut second = champion_team("second", "Second FC", &[]);
        won_in(&mut second, 2038, "div-2", 31);
        let mut game = game_with(vec![], vec![top, second]);
        english_ladder(&mut game);

        assert_eq!(champions_of(&game), vec![(2038, "top".to_string())]);
    }

    /// Given titles in different seasons, with wins that differ and a tie on both,
    /// When the champions are listed,
    /// Then the latest season is first, then the more wins, then the club's name.
    #[test]
    fn champions_are_ordered_by_season_then_wins_then_name() {
        let mut older = champion_team("older", "Older FC", &[]);
        won_in(&mut older, 2036, "div-1", 35);
        let mut rich = champion_team("rich", "Zeta FC", &[]);
        won_in(&mut rich, 2038, "div-1", 28);
        let mut tied = champion_team("tied", "Alpha FC", &[]);
        won_in(&mut tied, 2038, "div-1", 28);
        let mut strong = champion_team("strong", "Mid FC", &[]);
        won_in(&mut strong, 2038, "div-1", 33);
        let mut game = game_with(vec![], vec![older, rich, tied, strong]);
        english_ladder(&mut game);

        let order: Vec<String> = champions_of(&game).into_iter().map(|(_, id)| id).collect();

        assert_eq!(order, vec!["strong", "tied", "rich", "older"]);
    }

    /// Given a record saved before records named their competition (a blank id), and one whose
    /// competition has since left the world,
    /// When the champions are listed,
    /// Then both keep the old reading, first place, so a past title is never erased.
    #[test]
    fn a_record_that_cannot_be_placed_on_the_ladder_keeps_the_old_reading() {
        let mut old_save = champion_team("old", "Old FC", &[]);
        won_in(&mut old_save, 2030, "", 30);
        let mut gone = champion_team("gone", "Gone FC", &[]);
        won_in(&mut gone, 2031, "removed-league", 30);
        let mut game = game_with(vec![], vec![old_save, gone]);
        english_ladder(&mut game);

        assert_eq!(
            champions_of(&game),
            vec![(2031, "gone".to_string()), (2030, "old".to_string())]
        );
    }

    /// Given a retiree who spent a season at a club that won the second division and another at a
    /// club that won the top flight,
    /// When legends are listed,
    /// Then only the top-flight season counts as one of his titles.
    #[test]
    fn a_legends_titles_count_top_flight_titles_only() {
        let mut top = champion_team("top", "Top FC", &[]);
        won_in(&mut top, 2037, "div-1", 30);
        let mut second = champion_team("second", "Second FC", &[]);
        won_in(&mut second, 2038, "div-2", 30);
        let both = veteran(
            "both",
            "Both Ways",
            vec![
                career(2037, "top", "Top FC", 30, 5),
                career(2038, "second", "Second FC", 30, 5),
            ],
        );
        let mut game = game_with(vec![both], vec![top, second]);
        english_ladder(&mut game);

        assert_eq!(hall_of_fame_legends(&game)[0].titles, 1);
    }

    /// Given a second division played twice over the same clubs (an Apertura and its Clausura, the
    ///       second played later, so the ladder keeps it and collapses the first away)
    /// When a club that won the first half is asked about
    /// Then it is no champion: a collapsed half is still a second-division table, not an unknown one.
    #[test]
    fn a_collapsed_halfs_second_division_win_is_not_a_title() {
        let mut first_half_winner = champion_team("ap", "Apertura FC", &[]);
        won_in(&mut first_half_winner, 2038, "div-2-apertura", 30);
        let mut game = game_with(vec![], vec![first_half_winner]);
        english_ladder(&mut game);
        for (id, priority, played_on) in [
            ("div-2-apertura", 3, None),
            ("div-2-clausura", 4, Some("2038-11-01")),
        ] {
            let mut half = League::new(id.into(), id.into(), 2038, &["x".into(), "y".into()]);
            half.country_id = Some("ENG".into());
            half.priority = priority;
            if let Some(date) = played_on {
                half.fixtures.push(domain::league::Fixture {
                    date: date.into(),
                    status: domain::league::FixtureStatus::Completed,
                    ..Default::default()
                });
            }
            game.competitions.push(half);
        }
        game.competitions
            .retain(|competition| competition.id != "div-2");

        assert_eq!(champions_of(&game), Vec::<(u32, String)>::new());
    }
}
