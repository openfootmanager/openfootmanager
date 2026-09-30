//! A club never runs out of players.
//!
//! The floor is how many players of each position group a club must have
//! registered — injured or not — to put a side out at all, with a keeper to
//! spare. It is one rule with one home: the generator builds clubs to it, and
//! every path that can take players away from a club answers to it.

use crate::game::Game;
use domain::player::Position;

/// Players a club must keep registered in each position group, in
/// `[GK, DEF, MID, FWD]` order. Twelve in all: an eleven and a second keeper,
/// because a club whose only keeper is injured still has to put one in goal.
pub const MIN_PLAYERS_PER_GROUP: [(Position, usize); 4] = [
    (Position::Goalkeeper, 2),
    (Position::Defender, 4),
    (Position::Midfielder, 4),
    (Position::Forward, 2),
];

/// The floor for one position group, or 0 for a group with none.
pub fn group_floor(group: &Position) -> usize {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .find(|(position, _)| position == group)
        .map(|(_, floor)| *floor)
        .unwrap_or(0)
}

/// Every position group this club is short in, with how many players it is
/// short by. Empty when the club is at or above the floor everywhere.
///
/// Counts every player registered to the club — injured players included,
/// since they are still the club's to field in a crisis, and players out on
/// loan excluded, since they are registered to their borrower.
pub fn squad_shortfall(game: &Game, team_id: &str) -> Vec<(Position, usize)> {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .filter_map(|(group, floor)| {
            let have = game
                .players
                .iter()
                .filter(|player| player.team_id.as_deref() == Some(team_id))
                .filter(|player| player.position.to_group_position() == *group)
                .count();
            (have < *floor).then(|| (group.clone(), floor - have))
        })
        .collect()
}

/// Bring a club back up to the floor by signing free agents, and return the
/// ids of the players it signed (empty when the club was not short).
///
/// For each group the club is short in, the best free agent of that group
/// (highest rating, not retired, fit before injured) is signed on the terms
/// the contracts module expects him to want. When nobody suitable is on the
/// market a free agent is generated for the club's country and signed the same
/// way, so the club is never left short. The club's wage policy is not
/// consulted: this is the signing a club makes because it cannot otherwise put
/// a side out, and "we cannot afford to field a team" is not an outcome the game
/// allows.
pub fn restore_minimum_squad(game: &mut Game, team_id: &str) -> Vec<String> {
    use chrono::Datelike;

    let shortfall = squad_shortfall(game, team_id);
    if shortfall.is_empty() {
        return Vec::new();
    }
    let Some(team) = game.teams.iter().find(|team| team.id == team_id).cloned() else {
        log::error!("[squad_floor] cannot restore the squad of unknown club {team_id}");
        return Vec::new();
    };
    let current_date = game.clock.current_date.date_naive();

    let mut signed = Vec::new();
    for (group, missing) in shortfall {
        for _ in 0..missing {
            let index = best_free_agent(game, &group).unwrap_or_else(|| {
                game.players.push(crate::generator::generate_free_agent(
                    &group,
                    &team.country,
                    current_date.year() as u32,
                ));
                game.players.len() - 1
            });
            let wage = crate::contracts::expected_wage(&game.players[index], &team, current_date);
            let years =
                crate::contracts::expected_contract_years(&game.players[index], current_date);
            match crate::contracts::sign_free_agent(game, index, &team, wage, years, current_date) {
                Ok(()) => signed.push(game.players[index].id.clone()),
                Err(error) => log::error!(
                    "[squad_floor] could not sign {} for {team_id}: {error}",
                    game.players[index].id
                ),
            }
        }
    }
    signed
}

/// The free agent of this group a club would sign first: not retired, fit
/// before injured, then the highest rating. Ties fall to the lowest id so the
/// answer does not depend on the order players are stored in.
fn best_free_agent(game: &Game, group: &Position) -> Option<usize> {
    game.players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.team_id.is_none() && !player.retired)
        .filter(|(_, player)| player.position.to_group_position() == *group)
        .max_by(|(_, left), (_, right)| {
            left.injury
                .is_none()
                .cmp(&right.injury.is_none())
                .then(left.ovr.cmp(&right.ovr))
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes};
    use domain::team::Team;

    fn attrs() -> PlayerAttributes {
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

    pub(super) fn player(id: &str, team_id: Option<&str>, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-01-01".to_string(),
            "England".to_string(),
            position,
            attrs(),
        );
        player.team_id = team_id.map(str::to_string);
        player
    }

    /// A club with exactly `per_group` players in each group, `[GK, DEF, MID, FWD]`.
    pub(super) fn club_with(per_group: [usize; 4]) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());
        let team = Team::new(
            "club".to_string(),
            "Club".to_string(),
            "CLB".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );
        let mut players = Vec::new();
        for ((group, _), count) in MIN_PLAYERS_PER_GROUP.iter().zip(per_group) {
            for i in 0..count {
                players.push(player(
                    &format!("{group:?}{i}"),
                    Some("club"),
                    group.clone(),
                ));
            }
        }
        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    #[test]
    fn a_club_at_the_floor_is_short_of_nothing() {
        let game = club_with([2, 4, 4, 2]);
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    #[test]
    fn a_club_is_short_by_group_and_by_how_many() {
        let game = club_with([0, 3, 4, 5]);
        assert_eq!(
            squad_shortfall(&game, "club"),
            vec![(Position::Goalkeeper, 2), (Position::Defender, 1)]
        );
    }

    #[test]
    fn a_club_with_nobody_is_short_everywhere() {
        let game = club_with([0, 0, 0, 0]);
        assert_eq!(
            squad_shortfall(&game, "club"),
            MIN_PLAYERS_PER_GROUP.to_vec()
        );
    }

    /// A player out on loan is registered to his borrower, and an injured one
    /// is still the club's to field. Granular positions count in their group.
    #[test]
    fn injured_players_count_and_loaned_out_players_do_not() {
        let mut game = club_with([2, 4, 4, 1]);
        let mut injured = player("injured_striker", Some("club"), Position::Striker);
        injured.injury = Some(domain::player::Injury {
            name: "common.injuries.calfStrain".to_string(),
            days_remaining: 30,
        });
        game.players.push(injured);
        game.players
            .push(player("loaned_out", Some("borrower"), Position::Goalkeeper));
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    fn free_agent(id: &str, position: Position, ovr: u8) -> Player {
        let mut agent = player(id, None, position);
        agent.ovr = ovr;
        agent.contract_end = None;
        agent.wage = 0;
        agent
    }

    /// Short by one keeper, with two keepers and a forward on the market: the
    /// better keeper is signed, on a real contract, and nobody else.
    #[test]
    fn a_short_club_signs_the_best_free_agent_of_the_group_it_is_short_in() {
        let mut game = club_with([1, 4, 4, 2]);
        game.players
            .push(free_agent("weak_keeper", Position::Goalkeeper, 55));
        game.players
            .push(free_agent("good_keeper", Position::Goalkeeper, 70));
        game.players
            .push(free_agent("forward", Position::Forward, 80));

        let signed = restore_minimum_squad(&mut game, "club");

        assert_eq!(signed, vec!["good_keeper".to_string()]);
        assert!(squad_shortfall(&game, "club").is_empty());
        let keeper = game.players.iter().find(|p| p.id == "good_keeper").unwrap();
        assert_eq!(keeper.team_id.as_deref(), Some("club"));
        assert!(keeper.contract_end.is_some(), "signed without a contract");
        assert!(keeper.wage > 0, "signed on no wage");
        let forward = game.players.iter().find(|p| p.id == "forward").unwrap();
        assert_eq!(
            forward.team_id, None,
            "a club short of keepers signed a forward"
        );
    }

    #[test]
    fn a_retired_player_is_never_signed() {
        let mut game = club_with([1, 4, 4, 2]);
        let mut retired = free_agent("retired_keeper", Position::Goalkeeper, 90);
        retired.retired = true;
        game.players.push(retired);

        let signed = restore_minimum_squad(&mut game, "club");

        assert_eq!(signed.len(), 1);
        assert_ne!(signed[0], "retired_keeper");
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    /// With nobody on the market the club still gets its players: generated
    /// free agents of the right group, signed like any other.
    #[test]
    fn a_club_with_nobody_to_sign_is_given_generated_free_agents() {
        let mut game = club_with([0, 0, 0, 0]);

        let signed = restore_minimum_squad(&mut game, "club");

        assert_eq!(signed.len(), 12);
        assert!(squad_shortfall(&game, "club").is_empty());
        for id in &signed {
            let player = game.players.iter().find(|p| &p.id == id).unwrap();
            assert_eq!(player.team_id.as_deref(), Some("club"));
            assert!(player.contract_end.is_some());
        }
    }

    #[test]
    fn a_club_at_the_floor_signs_nobody() {
        let mut game = club_with([2, 4, 4, 2]);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 90));
        assert!(restore_minimum_squad(&mut game, "club").is_empty());
    }
}
