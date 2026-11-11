use super::division_finished_on;
use crate::game::Game;
use domain::league::League;

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
        crate::world::ladder::is_ladder_table(competition)
            && Game::club_takes_part_in(competition, club_id)
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

/// The club whose league the user follows: his own, or, for a manager out of work, the last club
/// he managed (maintainer, 5 Nov 2026), so a sacked manager reads the awards, summary, preview and
/// news of the league he left. None for a manager who has never had a club.
pub fn followed_club(game: &Game) -> Option<&str> {
    game.manager
        .team_id
        .as_deref()
        .or_else(|| game.manager.last_club())
}

/// The league-table competition the screens show as the user's: [`followed_club`]'s. It used to take
/// the user's club, so for a manager out of work it fell to the `game.league` mirror, which with no
/// club is the world's first competition: a sacked manager from England read Argentina's awards
/// (#1270).
///
/// A manager who has never had a club follows none (maintainer, 8 Nov 2026), so the preview, the
/// awards and the summary show nothing for him. A followed club in no league table shows nothing
/// too, except in a legacy save with no `competitions`, whose only league is the mirror: in a
/// modern world the mirror is the world's first competition, a league the club has nothing to do
/// with. The season's rollover is a separate question, `rollover_division`.
pub fn user_division(game: &Game) -> Option<&League> {
    let club = followed_club(game)?;
    club_league_table(game, club).or_else(|| {
        game.competitions
            .is_empty()
            .then_some(game.league.as_ref())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::league::{CompetitionScope, CompetitionType, Fixture, FixtureStatus};

    /// Given a domestic league and later-finishing cup and international tables for the same
    /// club, when its league is selected, then only the domestic league is eligible.
    #[test]
    fn a_later_cup_or_international_table_cannot_replace_a_clubs_domestic_league() {
        let mut domestic = League::new("domestic".into(), "League".into(), 2025, &["club".into()]);
        domestic.fixtures.push(Fixture {
            date: "2025-05-01".into(),
            status: FixtureStatus::Completed,
            ..Default::default()
        });
        for (kind, scope) in [
            (CompetitionType::Cup, CompetitionScope::Domestic),
            (CompetitionType::League, CompetitionScope::International),
        ] {
            let mut later = domestic.clone();
            later.id = "later".into();
            later.kind = kind;
            later.scope = scope;
            later.fixtures[0].date = "2025-06-01".into();
            let competitions = [domestic.clone(), later];
            assert_eq!(
                club_league_table_in(&competitions, "club").map(|league| league.id.as_str()),
                Some("domestic")
            );
        }
    }
}
