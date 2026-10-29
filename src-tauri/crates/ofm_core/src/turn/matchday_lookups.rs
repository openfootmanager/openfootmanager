//! What a matchday asks of the whole world, found once instead of once per match.
//!
//! A world holds tens of thousands of players and fixtures; the questions a match asks of them —
//! who are this club's players, does it play again soon — were each a pass over all of them,
//! several times per match. A competition-day reads the answers off this instead.

use std::collections::{HashMap, HashSet};

use crate::game::Game;

pub(crate) struct MatchdayLookups {
    clubs_with_a_match_soon: HashSet<String>,
    club_positions: HashMap<String, Vec<usize>>,
    fixtures_counted: usize,
}

impl MatchdayLookups {
    pub(crate) fn of(game: &Game) -> Self {
        Self {
            clubs_with_a_match_soon: super::squad::clubs_with_a_match_soon(game),
            club_positions: crate::match_day_eligibility::positions_by_club(&game.players),
            fixtures_counted: fixtures_in_play(game),
        }
    }

    /// Whether the club plays again within the congestion window.
    pub(crate) fn is_congested(&self, team_id: &str) -> bool {
        self.clubs_with_a_match_soon.contains(team_id)
    }

    /// Where the club's players stand in `game.players`, ascending; empty for a club with none.
    /// In a debug build, a group left stale by a player changing club without [`Self::players_moved`]
    /// is refused here.
    pub(crate) fn club(&self, game: &Game, team_id: &str) -> &[usize] {
        let group = self
            .club_positions
            .get(team_id)
            .map_or(&[][..], Vec::as_slice);
        crate::match_day_eligibility::debug_assert_group_is_current(&game.players, team_id, group);
        group
    }

    /// Call after a match is applied: an applied match can add fixtures (a knockout round's
    /// successor), and then who plays soon is asked of the fixtures again.
    pub(crate) fn after_match_applied(&mut self, game: &Game) {
        let fixtures_now = fixtures_in_play(game);
        if fixtures_now != self.fixtures_counted {
            self.clubs_with_a_match_soon = super::squad::clubs_with_a_match_soon(game);
            self.fixtures_counted = fixtures_now;
        }
    }

    /// Call after anything that moved a player between clubs (a kick-off top-up signing a free
    /// agent): the clubs' players are found again.
    pub(crate) fn players_moved(&mut self, game: &Game) {
        self.club_positions = crate::match_day_eligibility::positions_by_club(&game.players);
    }
}

/// How many fixtures the world holds, which a match that adds some changes.
fn fixtures_in_play(game: &Game) -> usize {
    game.competitions
        .iter()
        .chain(game.league.iter())
        .map(|competition| competition.fixtures.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_of_two_clubs() -> Game {
        let mut game = crate::turn::squad::match_day_pool_tests::match_day_game(false, 3, 0);
        let mut visitor = game.players[0].clone();
        visitor.id = "visitor".to_string();
        visitor.team_id = Some("away".to_string());
        game.players.push(visitor);
        game
    }

    /// Given lookups found before a player changed club and told so afterwards,
    /// When the clubs are read, Then each club's group is where its players stand now.
    #[test]
    fn lookups_told_a_player_moved_read_the_current_clubs() {
        let mut game = world_of_two_clubs();
        let mut lookups = MatchdayLookups::of(&game);
        game.players[0].team_id = Some("away".to_string());

        lookups.players_moved(&game);

        assert_eq!(lookups.club(&game, "club"), [1, 2]);
        assert_eq!(lookups.club(&game, "away"), [0, 3]);
    }

    /// Given lookups found before a player changed club and never told,
    /// When a club is read, Then a debug build refuses the stale group.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "stale")]
    fn lookups_not_told_a_player_moved_are_refused() {
        let mut game = world_of_two_clubs();
        let lookups = MatchdayLookups::of(&game);
        game.players[0].team_id = Some("away".to_string());

        lookups.club(&game, "club");
    }
}
