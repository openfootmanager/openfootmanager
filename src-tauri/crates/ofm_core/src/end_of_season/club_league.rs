use super::division_finished_on;
use crate::game::Game;
use domain::league::{CompetitionFormat, League};

/// The league table a club contests, for any club. `None` when no league table lists it (a cup
/// has no table).
///
/// A split-season country runs a club through more than one table. Take the one the year
/// ended on — the same one the payout and the club's record are taken from — or the summary
/// crowns them from one half while they are paid for the other.
pub fn club_league_table<'a>(game: &'a Game, club_id: &str) -> Option<&'a League> {
    club_league_table_in(&game.competitions, club_id)
}

/// The same league selection for save upgrades, before a whole game has been loaded.
pub fn club_league_table_in<'a>(competitions: &'a [League], club_id: &str) -> Option<&'a League> {
    let contains_club = |competition: &League| {
        competition.rules.format == CompetitionFormat::LeagueTable
            && (competition.participant_ids.iter().any(|id| id == club_id)
                || competition
                    .standings
                    .iter()
                    .any(|standing| standing.team_id == club_id))
    };

    competitions
        .iter()
        .filter(|competition| contains_club(competition))
        .max_by(|left, right| {
            division_finished_on(left)
                .cmp(&division_finished_on(right))
                .then_with(|| right.priority.cmp(&left.priority))
        })
}
