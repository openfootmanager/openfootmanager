//! What the team-selection screen shows of every club's squad before a career begins: each
//! player's rating and the squad's average.
//!
//! The knowledge rule (`player_knowledge`) decides what a manager sees of other clubs' players
//! once he has one; before a career there is no club whose knowledge could apply, and choosing a
//! club blind is no choice (maintainer, wire-knowledge Q1, 2 Nov). So the screen gets its own
//! answer, and only while no career has begun: a manager between jobs later in a career sees
//! players through the knowledge rule, not through this.

use serde::Serialize;

use crate::game::Game;
use crate::slices::teams::{average_rating, players_by_club};
use domain::player::Player;

/// Every club's squad as the team-selection screen rates it; no club at all once a career has
/// begun.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TeamSelectionRatings {
    pub clubs: Vec<ClubSquadRatings>,
}

/// One club's squad: its average rating (the club card's rule) and its players, best rated
/// first, so the first eleven are its likely starting side.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ClubSquadRatings {
    pub team_id: String,
    pub average_rating: u8,
    pub players: Vec<RatedPlayer>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RatedPlayer {
    pub player_id: String,
    pub ovr: u8,
}

/// Before a career has begun: the manager has no club and has never had one.
fn before_a_career(game: &Game) -> bool {
    game.manager.team_id.is_none() && game.manager.career_history.is_empty()
}

/// Every club's squad as the team-selection screen rates it, before a career begins; no club once
/// one has begun (see [`TeamSelectionRatings`]). The players are grouped by club in one pass, since
/// this runs under the game's lock and a world holds thousands of them.
pub fn query_team_selection(game: &Game) -> TeamSelectionRatings {
    if !before_a_career(game) {
        return TeamSelectionRatings { clubs: Vec::new() };
    }
    let by_club = players_by_club(&game.players);
    let clubs = game
        .teams
        .iter()
        .map(|team| {
            let mut squad: Vec<&Player> =
                by_club.get(team.id.as_str()).cloned().unwrap_or_default();
            squad.sort_by(|left, right| right.ovr.cmp(&left.ovr).then(left.id.cmp(&right.id)));
            ClubSquadRatings {
                team_id: team.id.clone(),
                average_rating: average_rating(&squad),
                players: squad
                    .iter()
                    .map(|player| RatedPlayer {
                        player_id: player.id.clone(),
                        ovr: player.ovr,
                    })
                    .collect(),
            }
        })
        .collect();
    TeamSelectionRatings { clubs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::{Manager, ManagerCareerEntry};
    use domain::player::Position;
    use domain::team::Team;

    fn team(id: &str) -> Team {
        Team::new(
            id.into(),
            id.into(),
            id.into(),
            "ENG".into(),
            "Town".into(),
            "Ground".into(),
            20_000,
        )
    }

    fn rated(id: &str, club: &str, ovr: u8) -> Player {
        let mut player = Player::new(
            id.into(),
            id.into(),
            id.into(),
            "2000-01-01".into(),
            "ENG".into(),
            Position::Midfielder,
            crate::test_support::uniform_attributes(60),
        );
        player.team_id = Some(club.into());
        player.ovr = ovr;
        player
    }

    /// A world before any career: two clubs, a fresh manager with no club.
    fn world() -> Game {
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        Game::new(
            clock,
            manager,
            vec![team("north"), team("south")],
            vec![
                rated("n1", "north", 70),
                rated("n2", "north", 81),
                rated("n3", "north", 74),
                rated("s1", "south", 60),
            ],
            vec![],
            vec![],
        )
    }

    /// Given a world before any career
    /// When the team-selection screen asks for the squads
    /// Then every club comes with its players best rated first and its average by the club card's
    ///      rule (rounded down: 70, 81, 74 average 75)
    #[test]
    fn before_a_career_every_club_is_rated() {
        let ratings = query_team_selection(&world());

        let north = &ratings.clubs[0];
        assert_eq!(north.team_id, "north");
        let order: Vec<(&str, u8)> = north
            .players
            .iter()
            .map(|rated| (rated.player_id.as_str(), rated.ovr))
            .collect();
        assert_eq!(order, vec![("n2", 81), ("n3", 74), ("n1", 70)]);
        assert_eq!(north.average_rating, 75);
        assert_eq!(ratings.clubs[1].average_rating, 60);
    }

    /// Given the manager has taken a club
    /// When the squads are asked for
    /// Then none comes: from then on the club's knowledge decides what he sees
    #[test]
    fn once_a_career_has_a_club_nothing_is_rated_here() {
        let mut game = world();
        game.manager.hire("north".into());

        assert!(query_team_selection(&game).clubs.is_empty());
    }

    /// Given a manager between jobs later in his career (no club, but a career behind him)
    /// When the squads are asked for
    /// Then none comes: being out of work is not a new career's team selection
    #[test]
    fn a_manager_between_jobs_is_not_shown_every_rating() {
        let mut game = world();
        game.manager.career_history.push(ManagerCareerEntry {
            team_id: "north".into(),
            team_name: "north".into(),
            start_date: "2024-07-01".into(),
            end_date: Some("2026-05-30".into()),
            matches: 76,
            wins: 30,
            draws: 20,
            losses: 26,
            best_league_position: Some(5),
        });

        assert!(query_team_selection(&game).clubs.is_empty());
    }
}
