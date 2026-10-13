//! The club player pool every match-day consumer reads.
use crate::player_rating::natural_ovr;
use domain::player::{Player, Position, SquadRole};
use std::collections::HashMap;

// A healthy eleven plus seven reserves; healthy seniors are never capped.
const MATCH_DAY_POOL_TARGET: usize = 18;

/// Every player on the books of `team_id`, in the order of `players`.
pub(crate) fn club_players<'a>(players: &'a [Player], team_id: &str) -> Vec<&'a Player> {
    players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect()
}

/// Every club's players, found in one pass, each club's in the order of `players`. Looked up by
/// club, never iterated, so the map's hashing decides nothing.
pub(crate) fn players_by_club(players: &[Player]) -> HashMap<&str, Vec<&Player>> {
    let mut by_club: HashMap<&str, Vec<&Player>> = HashMap::new();
    for player in players {
        if let Some(team_id) = player.team_id.as_deref() {
            by_club.entry(team_id).or_default().push(player);
        }
    }
    by_club
}

/// Where each club's players stand in `players`, found in one pass: owned positions, for work that
/// changes players while it goes (a signing moves a player to a club) and so cannot hold borrows of
/// them. A club's list is in the order of `players`. Looked up by club, never iterated.
pub(crate) fn positions_by_club(players: &[Player]) -> HashMap<String, Vec<usize>> {
    let mut by_club: HashMap<String, Vec<usize>> = HashMap::new();
    for (position, player) in players.iter().enumerate() {
        if let Some(team_id) = player.team_id.as_deref() {
            by_club
                .entry(team_id.to_string())
                .or_default()
                .push(position);
        }
    }
    by_club
}

/// Where the club `team_id`'s players stand in `players`.
pub(crate) fn club_positions(players: &[Player], team_id: &str) -> Vec<usize> {
    players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.team_id.as_deref() == Some(team_id))
        .map(|(position, _)| position)
        .collect()
}

/// [`eligible_among`] for the club `team_id`, found by scanning `players`.
pub(crate) fn match_day_eligible_players<'a>(
    players: &'a [Player],
    team_id: &str,
) -> Vec<&'a Player> {
    eligible_among(&club_players(players, team_id))
}

/// Healthy seniors, with youth covering a missing keeper before the numeric
/// shortage. In a crisis the treatment room remains available to complete an XI.
/// Call-ups never change the player's squad role. `club` is one club's players.
pub(crate) fn eligible_among<'a>(club: &[&'a Player]) -> Vec<&'a Player> {
    let (mut eligible, mut youth): (Vec<_>, Vec<_>) = club
        .iter()
        .copied()
        .filter(|p| p.injury.is_none())
        .partition(|p| p.squad_role == SquadRole::Senior);
    youth.sort_by(|left, right| {
        natural_ovr(right)
            .total_cmp(&natural_ovr(left))
            .then_with(|| left.id.cmp(&right.id))
    });
    if !eligible
        .iter()
        .any(|p| p.natural_position == Position::Goalkeeper)
        && let Some(index) = youth
            .iter()
            .position(|p| p.natural_position == Position::Goalkeeper)
    {
        eligible.push(youth.remove(index));
    }
    let youth_needed = MATCH_DAY_POOL_TARGET.saturating_sub(eligible.len());
    eligible.extend(youth.into_iter().take(youth_needed));
    // The builder can field injured youth only when a healthy eleven is impossible.
    // Projecting them too ensures that no emergency call-up is invisible in training.
    if eligible.len() < 11 {
        eligible.extend(club.iter().copied().filter(|p| p.injury.is_some()));
    }
    eligible
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::test_fixtures::player_at;
    use domain::player::Injury;

    fn ids(players: &[&Player]) -> Vec<String> {
        players.iter().map(|player| player.id.clone()).collect()
    }

    /// Players of three clubs and some free agents, interleaved the way a world stores them.
    fn interleaved_world() -> Vec<Player> {
        let mut players = Vec::new();
        for round in 0..14 {
            for club in ["red", "green", "blue"] {
                players.push(player_at(&format!("{club}-{round}"), club, "2035-06-30"));
            }
            let mut free_agent = player_at(&format!("free-{round}"), "none", "2035-06-30");
            free_agent.team_id = None;
            players.push(free_agent);
        }
        players
    }

    /// Given a world whose clubs' players are interleaved,
    /// When the players are grouped by club in one pass,
    /// Then each club's group is the players a scan for that club finds, in the same order, and
    ///      free agents belong to no group.
    #[test]
    fn grouping_finds_each_clubs_players_in_the_order_scanning_finds_them() {
        let players = interleaved_world();
        let by_club = players_by_club(&players);

        for club in ["red", "green", "blue"] {
            assert_eq!(ids(&by_club[club]), ids(&club_players(&players, club)));
        }
        assert_eq!(by_club.len(), 3);
    }

    /// Given a club short of fit players, with youth and injured players among them,
    /// When its match-day pool is read from the club's group and by scanning the world,
    /// Then the pool is the same players in the same order.
    #[test]
    fn a_pool_built_from_a_clubs_group_is_the_pool_built_by_scanning() {
        let mut players = interleaved_world();
        for (index, player) in players
            .iter_mut()
            .filter(|player| player.team_id.as_deref() == Some("red"))
            .enumerate()
        {
            if index < 9 {
                player.injury = Some(Injury {
                    name: "knock".to_string(),
                    days_remaining: 5,
                });
            } else if index % 2 == 0 {
                player.squad_role = SquadRole::Youth;
            }
        }
        let by_club = players_by_club(&players);

        for club in ["red", "green", "blue"] {
            assert_eq!(
                ids(&eligible_among(&by_club[club])),
                ids(&match_day_eligible_players(&players, club)),
                "{club}"
            );
        }
        assert!(
            match_day_eligible_players(&players, "red")
                .iter()
                .any(|player| player.injury.is_some()),
            "red is short of fit players, so the pool includes injured ones"
        );
    }
}
