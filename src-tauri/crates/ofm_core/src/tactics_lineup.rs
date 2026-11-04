//! The side the manager's club will field, as the Tactics screen shows it: the eleven slot by slot
//! with the role each plays and how he stands there, the bench, and everyone else. Read from the
//! match's own selection (`turn::squad::match_day_selection`), so the screen never picks a second
//! eleven of its own. The players' own details (number, morale, condition, rating) are the squad
//! projection's (`get_squad`); this only says who goes where.

use std::collections::HashSet;

use domain::player::Position;
use domain::team::PlayerRole;
use serde::Serialize;
use ts_rs::TS;

use crate::game::Game;
use crate::match_day_eligibility::club_players;
use crate::player_rating::{Familiarity, familiarity, formation_slots};
use crate::turn::squad::{effective_role, match_day_selection};

const NO_TEAM_ERROR: &str = "be.error.noTeamAssigned";
const MANAGED_TEAM_NOT_FOUND_ERROR: &str = "be.error.managedTeamNotFound";

/// One slot of the eleven.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct LineupSlot {
    /// The position the slot is, from the formation.
    pub position: Position,
    pub player_id: String,
    /// The role he plays there: the one he was given if the slot admits it, else the standard one.
    pub role: PlayerRole,
    /// How he stands at the slot: natural, can play, or out of position.
    pub familiarity: Familiarity,
}

/// The manager's club's side for its next match.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct TacticsLineup {
    pub formation: String,
    /// The eleven the match would field, slot by slot in the formation's order.
    pub eleven: Vec<LineupSlot>,
    /// The bench as the match builds it: every other player available, best first.
    pub substitutes: Vec<String>,
    /// Everyone else at the club (injured, or a youth not called up), in squad order.
    pub reserves: Vec<String>,
}

/// The side the manager's club would field now. Fails with a translation key when the manager
/// has no club.
pub fn tactics_lineup(game: &Game) -> Result<TacticsLineup, String> {
    let team_id = game.manager.team_id.as_deref().ok_or(NO_TEAM_ERROR)?;
    let team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or(MANAGED_TEAM_NOT_FOUND_ERROR)?;
    let club = club_players(&game.players, team_id);
    let selection = match_day_selection(game, team_id, &club, None);
    let positions = formation_slots(&selection.formation);
    let eleven: Vec<LineupSlot> = selection
        .eleven
        .iter()
        .zip(&positions)
        .map(|(player, position)| LineupSlot {
            position: position.clone(),
            player_id: player.id.clone(),
            role: effective_role(Some(&team.player_roles), player, position),
            familiarity: familiarity(player, position),
        })
        .collect();
    let substitutes: Vec<String> = selection.bench.iter().map(|p| p.id.clone()).collect();
    let placed: HashSet<&str> = eleven
        .iter()
        .map(|slot| slot.player_id.as_str())
        .chain(substitutes.iter().map(String::as_str))
        .collect();
    let reserves = club
        .iter()
        .filter(|player| !placed.contains(player.id.as_str()))
        .map(|player| player.id.clone())
        .collect();
    Ok(TacticsLineup {
        formation: selection.formation.clone(),
        eleven,
        substitutes,
        reserves,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::test_support::uniform_attributes;
    use crate::world::test_fixtures::{manager_for, nation_team};
    use chrono::{TimeZone, Utc};
    use domain::player::{Injury, Player};

    const CLUB: &str = "alpha";

    fn player(id: &str, natural: Position, level: u8) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-01-01".to_string(),
            "England".to_string(),
            natural.clone(),
            uniform_attributes(level),
        );
        player.natural_position = natural;
        player.team_id = Some(CLUB.to_string());
        player
    }

    /// The eleven "s0".."s10" in 4-4-2's own positions, a bench of five weaker players, and the
    /// injured "hurt"; the club plays 4-4-2 with "s0".."s10" saved as its eleven.
    fn game() -> Game {
        let positions = formation_slots("4-4-2");
        let mut players: Vec<Player> = positions
            .iter()
            .enumerate()
            .map(|(slot, position)| player(&format!("s{slot}"), position.clone(), 70))
            .collect();
        for (n, position) in [
            Position::CentralMidfielder,
            Position::CenterBack,
            Position::Striker,
            Position::Goalkeeper,
            Position::LeftBack,
        ]
        .into_iter()
        .enumerate()
        {
            players.push(player(&format!("b{n}"), position, 50 + n as u8));
        }
        let mut hurt = player("hurt", Position::Striker, 80);
        hurt.injury = Some(Injury {
            name: "Hamstring".into(),
            days_remaining: 10,
        });
        players.push(hurt);
        let mut team = nation_team(CLUB, "England", 50);
        team.formation = "4-4-2".into();
        team.starting_xi_ids = (0..11).map(|n| format!("s{n}")).collect();
        let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 8, 1, 12, 0, 0).unwrap());
        Game::new(
            clock,
            manager_for(CLUB),
            vec![team],
            players,
            vec![],
            vec![],
        )
    }

    fn ids(lineup: &TacticsLineup) -> Vec<&str> {
        lineup
            .eleven
            .iter()
            .map(|slot| slot.player_id.as_str())
            .collect()
    }

    /// Given a club whose saved eleven fills 4-4-2's slots
    /// When its lineup is read
    /// Then the eleven is the saved one, slot by slot, each at the formation's position and
    ///      natural there.
    #[test]
    fn the_eleven_is_the_saved_one_slot_by_slot() {
        let game = game();

        let lineup = tactics_lineup(&game).unwrap();

        let expected: Vec<String> = (0..11).map(|n| format!("s{n}")).collect();
        assert_eq!(
            ids(&lineup),
            expected.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(
            lineup
                .eleven
                .iter()
                .map(|slot| slot.position.clone())
                .collect::<Vec<_>>(),
            formation_slots("4-4-2")
        );
        assert!(
            lineup
                .eleven
                .iter()
                .all(|slot| slot.familiarity == Familiarity::Natural)
        );
        assert_eq!(lineup.formation, "4-4-2");
    }

    /// Given the striker "s9" saved in the left-back slot, the centre-back "s2" given a stopper's
    ///       role, and "s3" given a striker's role his centre-back slot doesn't admit
    /// When the lineup is read
    /// Then "s9" is out of position at left-back, "s2" plays as a stopper, and "s3" plays the
    ///      standard role: the role the match will play, not the one stored.
    #[test]
    fn each_slot_carries_the_role_played_and_how_its_player_stands_there() {
        let mut game = game();
        let team = &mut game.teams[0];
        team.starting_xi_ids.swap(1, 9);
        team.player_roles.insert("s2".into(), PlayerRole::Stopper);
        team.player_roles.insert("s3".into(), PlayerRole::Poacher);

        let lineup = tactics_lineup(&game).unwrap();

        assert_eq!(lineup.eleven[1].player_id, "s9");
        assert_eq!(lineup.eleven[1].familiarity, Familiarity::OutOfPosition);
        assert_eq!(lineup.eleven[2].role, PlayerRole::Stopper);
        assert_eq!(lineup.eleven[3].role, PlayerRole::Standard);
    }

    /// Given a club with no saved eleven (a new career, or the page harness's world)
    /// When its lineup is read
    /// Then it holds the eleven the match picks for it: eleven different players.
    #[test]
    fn with_no_saved_eleven_the_lineup_is_the_one_the_match_picks() {
        let mut game = game();
        game.teams[0].starting_xi_ids.clear();

        let lineup = tactics_lineup(&game).unwrap();

        let picked: HashSet<&str> = ids(&lineup).into_iter().collect();
        assert_eq!(lineup.eleven.len(), 11);
        assert_eq!(picked.len(), 11);
    }

    fn injure(game: &mut Game, id: &str) {
        let player = game.players.iter_mut().find(|p| p.id == id).unwrap();
        player.injury = Some(Injury {
            name: "Knock".into(),
            days_remaining: 7,
        });
    }

    /// The club's side in each of the cases the match's own selection handles differently.
    fn side_cases() -> Vec<(&'static str, Game)> {
        let mut no_saved = game();
        no_saved.teams[0].starting_xi_ids.clear();
        let mut injured = game();
        injure(&mut injured, "s4");
        // Fewer than eleven fit players: the match tops the side up from the treatment room.
        let mut short = game();
        for id in ["s1", "s2", "s3", "s5", "s6", "s7", "b0", "b1", "b2", "b4"] {
            injure(&mut short, id);
        }
        // Both keepers out: someone else has to go in goal.
        let mut no_keeper = game();
        injure(&mut no_keeper, "s0");
        injure(&mut no_keeper, "b3");
        // Too few of the saved eleven left: the match rebuilds a fresh, slot-aligned side.
        let mut few_saved = game();
        few_saved.teams[0].starting_xi_ids.truncate(7);
        vec![
            ("saved eleven", game()),
            ("no saved eleven", no_saved),
            ("an injured starter", injured),
            ("a squad too short to field eleven fit players", short),
            ("no fit keeper", no_keeper),
            ("fewer than eight saved", few_saved),
        ]
    }

    /// Given a club's side in each case the match's selection handles differently (its eleven
    ///       saved or not, an injured starter, too few fit players, no fit keeper, too few saved)
    /// When its lineup is read, and its next match's side is built
    /// Then the eleven and the bench are the same players in the same order: the screen shows
    ///      the side that will play.
    #[test]
    fn the_lineup_is_the_side_the_match_fields() {
        for (case, game) in side_cases() {
            let lineup = tactics_lineup(&game).unwrap();
            let (side, bench) = crate::turn::squad::build_team_with_bench(&game, CLUB);

            let fielded: Vec<&str> = side.players.iter().map(|p| p.id.as_str()).collect();
            let benched: Vec<&str> = bench.iter().map(|p| p.id.as_str()).collect();
            assert_eq!(ids(&lineup), fielded, "{case}");
            assert_eq!(
                lineup
                    .substitutes
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                benched,
                "{case}"
            );
        }
    }

    /// Given an injured striker saved in the eleven, and five fit players on the bench
    /// When the lineup is read
    /// Then he is not in it (the match puts a fit player in his slot) and he waits among the
    ///      reserves; the eleven and the bench hold every fit player.
    #[test]
    fn an_injured_starter_waits_among_the_reserves() {
        let mut game = game();
        game.teams[0].starting_xi_ids[10] = "hurt".into();

        let lineup = tactics_lineup(&game).unwrap();

        assert!(!ids(&lineup).contains(&"hurt"));
        assert_eq!(lineup.reserves, vec!["hurt".to_string()]);
        assert_eq!(lineup.eleven.len() + lineup.substitutes.len(), 16);
    }

    /// Given a manager without a club
    /// When the lineup is read
    /// Then it is refused with the no-team key.
    #[test]
    fn a_manager_without_a_club_has_no_lineup() {
        let mut game = game();
        game.manager.team_id = None;

        assert_eq!(tactics_lineup(&game), Err(NO_TEAM_ERROR.to_string()));
    }
}
