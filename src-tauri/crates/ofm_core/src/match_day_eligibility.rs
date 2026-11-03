//! The club player pool every match-day consumer reads.
use crate::player_rating::natural_ovr;
use domain::player::{Player, Position, SquadRole};
use std::collections::{HashMap, HashSet};

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

/// In a debug build, panics unless `group` is where the club's players stand in `players` now.
///
/// A group found once and reused (a whole pass's worth of clubs, a matchday's) goes stale the
/// moment a player changes club and nobody finds it again. Every place that reuses one asks this
/// first, so a missed regroup fails the tests that exercise it instead of quietly planning a squad
/// the club no longer has. A release build checks nothing.
pub(crate) fn debug_assert_group_is_current(players: &[Player], team_id: &str, group: &[usize]) {
    debug_assert_eq!(
        group,
        club_positions(players, team_id).as_slice(),
        "the group kept for {team_id} is stale: a player changed club and it was not found again"
    );
}

/// A starting XI holds eleven players.
const STARTING_XI_SIZE: usize = 11;

/// A starting XI that names a player who is not in the club's match-day pool.
pub const STARTING_XI_PLAYER_NOT_ELIGIBLE: &str = "be.error.startingXiPlayerNotEligible";
/// A starting XI that names the same player twice.
pub const STARTING_XI_DUPLICATE_PLAYER: &str = "be.error.startingXiDuplicatePlayer";
/// A starting XI with the wrong number of players for the club's pool.
pub const STARTING_XI_WRONG_SIZE: &str = "be.error.startingXiWrongSize";

/// Whether `player_ids` is a starting XI the club `team_id` may field: every id a different player
/// in the club's match-day pool ([`match_day_eligible_players`]), and exactly eleven of them, or the
/// whole pool when the club has fewer than eleven players to field.
///
/// This is the one place a saved lineup is checked, for the screens and for an MCP agent alike
/// (`set_starting_xi_internal` calls it before it writes). It reads the match-day pool, so any rule
/// that rules a player out of that pool (a suspension, say) is enforced here with no change to this
/// function: that pool is the enforcement point. Errors are translation keys.
pub fn validate_starting_xi(
    players: &[Player],
    team_id: &str,
    player_ids: &[String],
) -> Result<(), String> {
    let pool = match_day_eligible_players(players, team_id);
    let mut named = HashSet::new();
    for id in player_ids {
        if !pool.iter().any(|player| &player.id == id) {
            return Err(STARTING_XI_PLAYER_NOT_ELIGIBLE.to_string());
        }
        if !named.insert(id.as_str()) {
            return Err(STARTING_XI_DUPLICATE_PLAYER.to_string());
        }
    }
    if player_ids.len() != pool.len().min(STARTING_XI_SIZE) {
        return Err(STARTING_XI_WRONG_SIZE.to_string());
    }
    Ok(())
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
        // And the pool is the right one, not merely the same one twice. Red is short of fit
        // seniors: its three come first, then its two youngsters cover the shortage, then the
        // nine in the treatment room, in the order they are stored. Green is not short and
        // fields its fit seniors, as they are stored.
        let in_the_pool = |club: &str, rounds: Vec<usize>| -> Vec<String> {
            rounds
                .into_iter()
                .map(|round| format!("{club}-{round}"))
                .collect()
        };
        assert_eq!(
            ids(&eligible_among(&by_club["red"])),
            in_the_pool("red", vec![9, 11, 13, 10, 12, 0, 1, 2, 3, 4, 5, 6, 7, 8])
        );
        assert_eq!(
            ids(&eligible_among(&by_club["green"])),
            in_the_pool("green", (0..14).collect())
        );
        assert!(
            match_day_eligible_players(&players, "red")
                .iter()
                .any(|player| player.injury.is_some()),
            "red is short of fit players, so the pool includes injured ones"
        );
    }

    /// Given a group found before a player left the club and a group found after,
    /// When each is checked against the world,
    /// Then the current group passes.
    #[test]
    fn a_current_group_is_accepted() {
        let mut players = interleaved_world();
        let before = club_positions(&players, "red");
        debug_assert_group_is_current(&players, "red", &before);

        players[before[0]].team_id = Some("blue".to_string());

        debug_assert_group_is_current(&players, "red", &club_positions(&players, "red"));
    }

    /// Given a group found before a player left the club,
    /// When it is reused without being found again,
    /// Then a debug build refuses it.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "the group kept for red is stale")]
    fn a_stale_group_is_refused() {
        let mut players = interleaved_world();
        let before = club_positions(&players, "red");
        players[before[0]].team_id = Some("blue".to_string());

        debug_assert_group_is_current(&players, "red", &before);
    }

    /// A club of `size` players, `red-0` .. `red-{size-1}`, all healthy seniors, and a rival's player.
    fn club_of(size: usize) -> Vec<Player> {
        let mut players: Vec<Player> = (0..size)
            .map(|index| player_at(&format!("red-{index}"), "red", "2035-06-30"))
            .collect();
        players.push(player_at("rival-0", "blue", "2035-06-30"));
        players
    }

    fn xi(prefix: &str, range: std::ops::Range<usize>) -> Vec<String> {
        range.map(|index| format!("{prefix}-{index}")).collect()
    }

    /// Given a club of fourteen healthy seniors,
    /// When a lineup of eleven distinct players of the club is checked,
    /// Then it is accepted.
    #[test]
    fn a_full_xi_of_the_clubs_eligible_players_is_accepted() {
        assert_eq!(
            validate_starting_xi(&club_of(14), "red", &xi("red", 0..11)),
            Ok(())
        );
    }

    /// Given a club with plenty of players,
    /// When ten ids, or twelve, are checked,
    /// Then each is refused as the wrong size.
    #[test]
    fn an_xi_of_ten_or_twelve_is_refused_as_the_wrong_size() {
        let players = club_of(14);

        for ids in [xi("red", 0..10), xi("red", 0..12), vec![]] {
            assert_eq!(
                validate_starting_xi(&players, "red", &ids),
                Err(STARTING_XI_WRONG_SIZE.to_string())
            );
        }
    }

    /// Given a lineup that names one player twice,
    /// When it is checked,
    /// Then it is refused as a duplicate, even though it has eleven entries.
    #[test]
    fn a_player_named_twice_is_refused() {
        let mut ids = xi("red", 0..10);
        ids.push("red-3".to_string());

        assert_eq!(
            validate_starting_xi(&club_of(14), "red", &ids),
            Err(STARTING_XI_DUPLICATE_PLAYER.to_string())
        );
    }

    /// Given a lineup that names a rival's player, one nobody knows, or a free agent,
    /// When it is checked,
    /// Then each is refused as not eligible.
    #[test]
    fn a_player_outside_the_club_is_refused() {
        let mut players = club_of(14);
        let mut free_agent = player_at("free-0", "none", "2035-06-30");
        free_agent.team_id = None;
        players.push(free_agent);

        for stranger in ["rival-0", "nobody", "free-0"] {
            let mut ids = xi("red", 0..10);
            ids.push(stranger.to_string());
            assert_eq!(
                validate_starting_xi(&players, "red", &ids),
                Err(STARTING_XI_PLAYER_NOT_ELIGIBLE.to_string()),
                "{stranger}"
            );
        }
    }

    /// Given a club with healthy seniors to spare and one of the eleven chosen injured,
    /// When the lineup is checked,
    /// Then the injured player is refused: he is not in the match-day pool.
    #[test]
    fn an_injured_player_is_refused_while_fit_players_can_be_fielded() {
        let mut players = club_of(14);
        players[0].injury = Some(Injury {
            name: "knock".to_string(),
            days_remaining: 5,
        });

        assert_eq!(
            validate_starting_xi(&players, "red", &xi("red", 0..11)),
            Err(STARTING_XI_PLAYER_NOT_ELIGIBLE.to_string())
        );
        assert_eq!(
            validate_starting_xi(&players, "red", &xi("red", 1..12)),
            Ok(())
        );
    }

    /// Given a club with only nine fit seniors and three in the treatment room,
    /// When an XI that fields all three injured players is checked,
    /// Then it is accepted: in a crisis the pool includes them, so the club can still field eleven.
    #[test]
    fn a_crisis_club_may_field_its_injured_players() {
        let mut players = club_of(12);
        for player in players
            .iter_mut()
            .filter(|p| p.id.starts_with("red-9") || p.id == "red-10" || p.id == "red-11")
        {
            player.injury = Some(Injury {
                name: "knock".to_string(),
                days_remaining: 5,
            });
        }

        assert_eq!(
            validate_starting_xi(&players, "red", &xi("red", 0..11)),
            Ok(())
        );
    }

    /// Given a club with only eight players on its books,
    /// When all eight, and then seven, are checked,
    /// Then all eight are accepted (the pool is smaller than eleven) and seven are refused.
    #[test]
    fn a_club_with_fewer_than_eleven_players_fields_all_it_has() {
        let players = club_of(8);

        assert_eq!(
            validate_starting_xi(&players, "red", &xi("red", 0..8)),
            Ok(())
        );
        assert_eq!(
            validate_starting_xi(&players, "red", &xi("red", 0..7)),
            Err(STARTING_XI_WRONG_SIZE.to_string())
        );
    }
}
