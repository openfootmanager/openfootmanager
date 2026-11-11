//! Where each club stands in its own league table, for the league-position sponsorship bonus
//! (#1363).

use std::collections::HashMap;

use domain::league::League;

use crate::end_of_season::club_league_table;
use crate::game::Game;

/// The table a club's position is read from: the league table it contests, whichever league the
/// user follows. A legacy save has no `competitions`, and its one league is the `game.league`
/// mirror.
fn own_table<'a>(game: &'a Game, club_id: &str) -> Option<&'a League> {
    if game.competitions.is_empty() {
        return game.league.as_ref();
    }
    club_league_table(game, club_id)
}

/// Each club of `league` by its place in the table, first place is 1.
fn ranking(league: &League) -> HashMap<String, u32> {
    league
        .sorted_standings()
        .into_iter()
        .enumerate()
        .map(|(index, standing)| (standing.team_id, index as u32 + 1))
        .collect()
}

/// A club's place in its own league table, `None` when no league table lists it.
pub(super) fn league_position(game: &Game, club_id: &str) -> Option<u32> {
    own_table(game, club_id).and_then(|league| ranking(league).remove(club_id))
}

/// Every club's place in its own league table, each table ranked once. A club in no league table
/// has no entry.
pub(super) fn league_positions(game: &Game) -> HashMap<String, u32> {
    let mut rankings: HashMap<&str, HashMap<String, u32>> = HashMap::new();
    let mut positions = HashMap::new();
    for team in &game.teams {
        let Some(table) = own_table(game, &team.id) else {
            continue;
        };
        let ranked = rankings
            .entry(table.id.as_str())
            .or_insert_with(|| ranking(table));
        if let Some(position) = ranked.get(&team.id) {
            positions.insert(team.id.clone(), *position);
        }
    }
    positions
}

#[cfg(test)]
mod tests {
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::finance::CashKind;
    use domain::league::League;
    use domain::manager::Manager;
    use domain::team::{Sponsorship, SponsorshipBonusCriterion, Team};

    const BASE: i64 = 10_000;
    const BONUS: i64 = 7_000;

    fn club(id: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            id.to_string(),
            id[..3].to_string(),
            "England".to_string(),
            "Testville".to_string(),
            "Ground".to_string(),
            22_000,
        );
        team.finance = 1_000_000;
        team.sponsorship = Some(Sponsorship {
            sponsor_name: "Acme".to_string(),
            base_value: BASE,
            remaining_weeks: 10,
            bonus_criteria: vec![SponsorshipBonusCriterion::LeaguePosition {
                max_position: 1,
                bonus_amount: BONUS,
            }],
        });
        team
    }

    fn table(id: &str, ranked: [(&str, u32); 2]) -> League {
        let ids: Vec<String> = ranked.iter().map(|(club, _)| club.to_string()).collect();
        let mut league = League::new(id.to_string(), id.to_string(), 2026, &ids);
        for (club, points) in ranked {
            if let Some(entry) = league
                .standings
                .iter_mut()
                .find(|entry| entry.team_id == club)
            {
                entry.points = points;
            }
        }
        league
    }

    /// Monday 16 February 2026; the user manages "alpha", in league A. League B is another
    /// country's, run by the same game.
    fn two_league_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("alpha".to_string());
        let mut game = Game::new(
            clock,
            manager,
            vec![club("alpha"), club("bravo"), club("charlie"), club("delta")],
            vec![],
            vec![],
            vec![],
        );
        let league_a = table("league-a", [("alpha", 30), ("bravo", 10)]);
        let league_b = table("league-b", [("charlie", 25), ("delta", 5)]);
        game.league = Some(league_a.clone());
        game.competitions = vec![league_a, league_b];
        game
    }

    fn sponsorship_paid(game: &Game, club_id: &str) -> i64 {
        game.cash_journal
            .iter()
            .filter(|post| post.club_id == club_id && post.kind == CashKind::Sponsorship)
            .map(|post| post.amount)
            .sum()
    }

    /// Given a world of two leagues, the user following league A, and "charlie" top of league B
    /// When the week's finances are processed
    /// Then "charlie" is paid the position bonus as "alpha" is: a club is judged on its own table.
    #[test]
    fn a_club_topping_the_league_the_user_does_not_follow_gets_the_bonus() {
        let mut game = two_league_game();

        crate::finances::process_weekly_finances(&mut game);

        assert_eq!(sponsorship_paid(&game, "charlie"), BASE + BONUS);
        assert_eq!(sponsorship_paid(&game, "alpha"), BASE + BONUS);
    }

    /// Given the same world
    /// When the week's finances are processed
    /// Then the clubs below the top of either league are paid the base only.
    #[test]
    fn a_club_below_the_top_of_its_own_league_gets_the_base_only() {
        let mut game = two_league_game();

        crate::finances::process_weekly_finances(&mut game);

        assert_eq!(sponsorship_paid(&game, "bravo"), BASE);
        assert_eq!(sponsorship_paid(&game, "delta"), BASE);
    }

    /// Given a legacy save with no competitions, whose only league is the `game.league` mirror
    /// When the week's finances are processed
    /// Then the mirror's leader is still paid the bonus.
    #[test]
    fn a_legacy_save_with_only_the_mirror_still_pays_its_leader() {
        let mut game = two_league_game();
        game.competitions.clear();

        crate::finances::process_weekly_finances(&mut game);

        assert_eq!(sponsorship_paid(&game, "alpha"), BASE + BONUS);
        assert_eq!(sponsorship_paid(&game, "bravo"), BASE);
    }
}
