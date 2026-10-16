//! Renewing a cup the day its edition is finished.
//!
//! The rollover renews every competition together, when the user's league ends. A cup is not the
//! user's league: its last tie can be played long before the rollover, or, when the calendar
//! postpones a tie past the league's last round, after it. Waiting for the rollover left the first
//! idle for months and the second a whole rollover late. So a finished cup is archived and
//! renewed by the same rule the rollover uses, but on its own day: the day its next edition is
//! four weeks from starting. Until then the finished bracket stays where the player can see who
//! won; renewing on the day of the final would replace it with the next edition at once.
//!
//! Only domestic and regional cups: their entrants never change at the rollover. A continental
//! competition's field is worked out from the whole season's domestic results, and a league
//! needs its whole pyramid finished before it can renew, so both wait for the rollover.

use chrono::{DateTime, Days, NaiveDate, Utc};
use domain::league::{CompetitionFormat, CompetitionScope, CompetitionType, League};
use log::{debug, warn};

use crate::edition_archive::archive_completed_edition;
use crate::end_of_season::{is_cup_edition_finished, season_calendar::next_edition};
use crate::game::Game;

/// The shared break the rollover keeps between editions, in days.
const BREAK_BETWEEN_EDITIONS_DAYS: u64 = 28;

fn is_renewable_cup(competition: &League) -> bool {
    competition.kind == CompetitionType::Cup
        && matches!(
            competition.scope,
            CompetitionScope::Domestic | CompetitionScope::Regional
        )
        && matches!(
            competition.rules.format,
            CompetitionFormat::Knockout | CompetitionFormat::GroupAndKnockout
        )
        && !competition.fixtures.is_empty()
}

/// Whether the cup's next edition is due to start within the shared break. A cup with no configured
/// start (month 0) is left to the rollover.
fn next_edition_is_near(
    cup: &League,
    clock: &crate::clock::GameClock,
    today_start: DateTime<Utc>,
    earliest_start: DateTime<Utc>,
) -> bool {
    cup.season_start_month != 0 && next_edition(cup, clock, today_start).start <= earliest_start
}

/// Archives and renews every cup whose edition finished and whose next edition is near, and returns how many it renewed. A cup
/// whose edition cannot be archived is left as it is: renewing it would lose the edition.
pub(crate) fn renew_finished_cups(game: &mut Game, today: &str) -> usize {
    let Some(earliest_start) = NaiveDate::parse_from_str(today, "%Y-%m-%d")
        .ok()
        .and_then(|day| day.checked_add_days(Days::new(BREAK_BETWEEN_EDITIONS_DAYS)))
        .and_then(|day| day.and_hms_opt(0, 0, 0))
        .map(|day| DateTime::<Utc>::from_naive_utc_and_offset(day, Utc))
    else {
        return 0;
    };

    let today_start = earliest_start - Days::new(BREAK_BETWEEN_EDITIONS_DAYS);
    let mut renewed = 0;
    for index in 0..game.competitions.len() {
        let cup = &game.competitions[index];
        if !is_renewable_cup(cup)
            || !is_cup_edition_finished(cup)
            || !next_edition_is_near(cup, &game.clock, today_start, earliest_start)
        {
            continue;
        }
        if let Err(failure) = archive_completed_edition(&mut game.edition_archive, cup) {
            warn!(
                "[cup-renewal] {} season {} not renewed, its edition cannot be archived: {failure:?}",
                cup.id, cup.season
            );
            continue;
        }
        let edition = next_edition(cup, &game.clock, earliest_start);
        let cup = &mut game.competitions[index];
        debug!(
            "[cup-renewal] {} season {} finished on {today}; renewing as season {}",
            cup.id, cup.season, edition.season
        );
        match cup.rules.format {
            CompetitionFormat::Knockout => {
                crate::schedule::regenerate_knockout_for_season(cup, edition.season, edition.start)
            }
            CompetitionFormat::GroupAndKnockout => {
                crate::group_stage::regenerate_for_season(cup, edition.season, edition.start)
            }
            CompetitionFormat::LeagueTable => continue,
        }
        renewed += 1;
    }
    renewed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::competition_test_support::{cup, group_cup};
    use chrono::TimeZone;
    use domain::league::FixtureStatus;
    use domain::manager::Manager;

    fn game_with(competitions: Vec<League>) -> Game {
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        let clock = GameClock::new(Utc.with_ymd_and_hms(2033, 5, 20, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.competitions = competitions;
        game
    }

    /// A finished cup whose next edition starts on 1 September 2034.
    fn september_cup() -> League {
        let mut finished = cup(8);
        finished.season = 2033;
        finished.season_start_month = 9;
        finished.season_start_day = 1;
        finished
    }

    /// Given a finished domestic cup whose next edition is four weeks from starting,
    /// When the day ends,
    /// Then its edition is archived with its champion and the cup starts its next edition.
    #[test]
    fn a_finished_cup_renews_when_its_next_edition_is_near() {
        let mut game = game_with(vec![september_cup()]);

        let renewed = renew_finished_cups(&mut game, "2034-08-10");

        assert_eq!(renewed, 1);
        let next = &game.competitions[0];
        assert!(next.season > 2033, "a new edition");
        assert!(!next.fixtures.is_empty());
        assert!(
            next.fixtures
                .iter()
                .all(|f| f.status == FixtureStatus::Scheduled)
        );
        let [edition] = game.edition_archive.as_slice() else {
            panic!("the finished edition is archived once");
        };
        assert_eq!(
            (edition.competition_id.as_str(), edition.season),
            (next.id.as_str(), 2033)
        );
    }

    /// Given a finished cup whose next edition is months away, When the day ends, Then the
    /// finished bracket stays: the player can still see who won it.
    #[test]
    fn a_finished_cup_keeps_its_bracket_until_the_next_edition_is_near() {
        let mut game = game_with(vec![september_cup()]);

        assert_eq!(renew_finished_cups(&mut game, "2034-05-20"), 0);

        assert_eq!(game.competitions[0].season, 2033);
        assert!(
            game.competitions[0]
                .fixtures
                .iter()
                .all(|f| f.status == FixtureStatus::Completed)
        );
        assert!(game.edition_archive.is_empty());
    }

    /// Given a cup with no configured start, When the day ends, Then it is left to the rollover.
    #[test]
    fn a_cup_with_no_configured_start_waits_for_the_rollover() {
        let mut unscheduled = cup(8);
        unscheduled.season_start_month = 0;
        let mut game = game_with(vec![unscheduled]);

        assert_eq!(renew_finished_cups(&mut game, "2034-08-10"), 0);
    }

    /// Given a cup that has just been renewed, When the next day ends, Then it is not renewed (or
    /// archived) again.
    #[test]
    fn a_renewed_cup_is_not_renewed_twice() {
        let mut game = game_with(vec![september_cup()]);
        renew_finished_cups(&mut game, "2034-08-10");
        let once = serde_json::to_string(&game.competitions).unwrap();

        let again = renew_finished_cups(&mut game, "2034-08-11");

        assert_eq!(again, 0);
        assert_eq!(serde_json::to_string(&game.competitions).unwrap(), once);
        assert_eq!(game.edition_archive.len(), 1);
    }

    /// Given a cup with a tie still to play, When the day ends, Then it is left alone.
    #[test]
    fn a_cup_with_a_tie_to_play_is_not_renewed() {
        let mut unfinished = september_cup();
        unfinished.fixtures[0].status = FixtureStatus::Scheduled;
        unfinished.fixtures[0].result = None;
        let mut game = game_with(vec![unfinished]);

        assert_eq!(renew_finished_cups(&mut game, "2034-08-10"), 0);
        assert!(game.edition_archive.is_empty());
    }

    /// Given a finished group-and-knockout cup whose next edition is near, When the day ends,
    /// Then it renews with fresh groups.
    #[test]
    fn a_finished_group_cup_renews_with_fresh_groups() {
        let mut group = group_cup();
        group.season = 2033;
        group.season_start_month = 9;
        group.season_start_day = 1;
        let mut game = game_with(vec![group]);

        assert_eq!(renew_finished_cups(&mut game, "2034-08-10"), 1);
        assert!(
            game.competitions[0]
                .groups
                .iter()
                .flat_map(|group| &group.standings)
                .all(|row| row.played == 0)
        );
    }

    /// Given a finished continental competition and a finished World Cup, When the day ends,
    /// Then neither is renewed: their fields come from the rollover.
    #[test]
    fn continental_and_international_competitions_are_left_to_the_rollover() {
        let mut continental = september_cup();
        continental.kind = CompetitionType::ContinentalClub;
        continental.scope = CompetitionScope::Continental;
        let mut world_cup = september_cup();
        world_cup.kind = CompetitionType::InternationalNation;
        world_cup.scope = CompetitionScope::International;
        let mut game = game_with(vec![continental, world_cup]);

        assert_eq!(renew_finished_cups(&mut game, "2034-08-10"), 0);
        assert!(game.edition_archive.is_empty());
    }

    /// Given a finished domestic knockout that is not a cup (a friendly tournament), When the day
    /// ends, Then it is not renewed: only cups are.
    #[test]
    fn a_friendly_tournament_is_not_a_cup_and_is_not_renewed() {
        let mut tournament = september_cup();
        tournament.kind = CompetitionType::FriendlyCup;
        tournament.scope = CompetitionScope::Domestic;
        let mut game = game_with(vec![tournament]);

        assert_eq!(renew_finished_cups(&mut game, "2034-08-10"), 0);
    }

    /// Given a finished cup whose next edition starts on 1 September, When the day ends on 4 August
    /// (28 days before) and on 3 August (29 days before), Then it renews on the first and not the
    /// second: the four-week lead time is exact.
    #[test]
    fn a_cup_renews_on_the_exact_day_its_four_week_lead_begins() {
        let mut too_early = game_with(vec![september_cup()]);
        assert_eq!(renew_finished_cups(&mut too_early, "2034-08-03"), 0);

        let mut on_time = game_with(vec![september_cup()]);
        assert_eq!(renew_finished_cups(&mut on_time, "2034-08-04"), 1);
    }

    /// Given a finished cup whose next edition is near, When the world's day is processed,
    /// Then the cup has started its next edition: the hook is wired into the day.
    #[test]
    fn processing_a_day_renews_a_finished_cup_whose_next_edition_is_near() {
        let mut game = game_with(vec![september_cup()]);
        game.clock = GameClock::new(Utc.with_ymd_and_hms(2034, 8, 10, 12, 0, 0).unwrap());

        crate::turn::process_day(&mut game);

        assert!(game.competitions[0].season > 2033, "the cup was renewed");
        assert_eq!(game.edition_archive.len(), 1);
    }
}
