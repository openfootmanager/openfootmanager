//! The retired legends of the world, read from the archive and, until retirees leave the
//! roster, from players still flagged retired.

use std::collections::{HashMap, HashSet};

use domain::player::{CareerEntry, Position};
use serde::Serialize;

use crate::game::Game;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HallOfFameLegend {
    pub player_id: String,
    pub full_name: String,
    pub nationality: String,
    pub football_nation: String,
    pub position: Position,
    pub appearances: u32,
    pub goals: u32,
    pub assists: u32,
    /// Seasons he spent at a club that finished first that season.
    pub titles: u32,
    pub last_club_name: Option<String>,
    pub final_season: Option<u32>,
}

/// Who a legend is, whichever record he is read from.
struct Retiree<'a> {
    id: &'a str,
    full_name: &'a str,
    nationality: &'a str,
    football_nation: &'a str,
    position: Position,
    career: &'a [CareerEntry],
}

pub fn hall_of_fame_legends(game: &Game) -> Vec<HallOfFameLegend> {
    let championships = championship_seasons_by_club(game);
    let archived: HashSet<&str> = game.retired_players.iter().map(|p| p.id.as_str()).collect();

    let from_archive = game.retired_players.iter().map(|player| Retiree {
        id: &player.id,
        full_name: &player.full_name,
        nationality: &player.nationality,
        football_nation: &player.football_nation,
        position: player.natural_position.clone(),
        career: &player.career,
    });
    let from_roster = game
        .players
        .iter()
        .filter(|player| player.retired && !archived.contains(player.id.as_str()))
        .map(|player| Retiree {
            id: &player.id,
            full_name: &player.full_name,
            nationality: &player.nationality,
            football_nation: &player.football_nation,
            position: player.position.clone(),
            career: &player.career,
        });

    let mut legends: Vec<HallOfFameLegend> = from_archive
        .chain(from_roster)
        .filter(|retiree| !retiree.career.is_empty())
        .map(|retiree| legend_of(&retiree, &championships))
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

fn championship_seasons_by_club(game: &Game) -> HashMap<&str, HashSet<u32>> {
    game.teams
        .iter()
        .map(|team| {
            let seasons = team
                .history
                .iter()
                .filter(|record| record.league_position == 1)
                .map(|record| record.season)
                .collect();
            (team.id.as_str(), seasons)
        })
        .collect()
}

fn legend_of(
    retiree: &Retiree<'_>,
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
        player_id: retiree.id.to_string(),
        full_name: retiree.full_name.to_string(),
        nationality: retiree.nationality.to_string(),
        football_nation: retiree.football_nation.to_string(),
        position: retiree.position.clone(),
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
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes};
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
        Game::new(clock, manager, teams, players, vec![], vec![])
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

    /// Given a retiree still flagged retired in the roster (before the move), when legends are
    /// listed, then he is one, so the list is the same before and after the move.
    #[test]
    fn a_retiree_still_on_the_roster_is_a_legend() {
        let vale = veteran(
            "p-vale",
            "Victor Vale",
            vec![career(1, "t-a", "Alpha FC", 10, 2)],
        );
        let game = game_with(vec![vale], vec![]);

        let legends = hall_of_fame_legends(&game);

        assert_eq!(legends.len(), 1);
        assert_eq!(legends[0].player_id, "p-vale");
    }

    /// Given a person who is both archived and flagged retired in the roster, when legends are
    /// listed, then he appears once, from the archive.
    #[test]
    fn a_person_in_both_places_is_listed_once_from_the_archive() {
        let mut vale = veteran(
            "p-vale",
            "Victor Vale",
            vec![career(1, "t-a", "Alpha FC", 10, 2)],
        );
        let mut game = game_with(vec![], vec![]);
        game.retired_players.push(archive(&vale));
        vale.full_name = "Roster Copy".into();
        game.players.push(vale);

        let legends = hall_of_fame_legends(&game);

        assert_eq!(legends.len(), 1);
        assert_eq!(legends[0].full_name, "Victor Vale");
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
}
