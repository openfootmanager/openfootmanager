//! Renewing a cup the day its edition is finished.
//!
//! The rollover renews every competition together, when the user's league ends. A cup is not the
//! user's league: its last tie can be played long before the rollover, or, when the calendar
//! postpones a tie past the league's last round, after it. Waiting for the rollover left the first
//! idle for months and the second a whole rollover late. So a finished cup is archived and
//! renewed on the day it finishes, by the same rule the rollover uses.
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

/// Archives and renews every cup whose edition finished, and returns how many it renewed. A cup
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

    let mut renewed = 0;
    for index in 0..game.competitions.len() {
        let cup = &game.competitions[index];
        if !is_renewable_cup(cup) || !is_cup_edition_finished(cup) {
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

    /// Given a domestic cup whose last tie has been played,
    /// When the day ends,
    /// Then its edition is archived with its champion and the cup starts its next edition.
    #[test]
    fn a_cup_renews_the_day_its_last_tie_is_played() {
        let finished = cup(8);
        let season = finished.season;
        let mut game = game_with(vec![finished]);

        let renewed = renew_finished_cups(&mut game, "2033-05-20");

        assert_eq!(renewed, 1);
        let next = &game.competitions[0];
        assert!(next.season > season, "a new edition");
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
            (next.id.as_str(), season)
        );
    }

    /// Given a cup that has just been renewed, When the next day ends, Then it is not renewed (or
    /// archived) again.
    #[test]
    fn a_renewed_cup_is_not_renewed_twice() {
        let mut game = game_with(vec![cup(8)]);
        renew_finished_cups(&mut game, "2033-05-20");
        let once = serde_json::to_string(&game.competitions).unwrap();

        let again = renew_finished_cups(&mut game, "2033-05-21");

        assert_eq!(again, 0);
        assert_eq!(serde_json::to_string(&game.competitions).unwrap(), once);
        assert_eq!(game.edition_archive.len(), 1);
    }

    /// Given a cup with a tie still to play, When the day ends, Then it is left alone.
    #[test]
    fn a_cup_with_a_tie_to_play_is_not_renewed() {
        let mut unfinished = cup(8);
        unfinished.fixtures[0].status = FixtureStatus::Scheduled;
        unfinished.fixtures[0].result = None;
        let mut game = game_with(vec![unfinished]);

        assert_eq!(renew_finished_cups(&mut game, "2033-05-20"), 0);
        assert!(game.edition_archive.is_empty());
    }

    /// Given a finished group-and-knockout cup, When the day ends, Then it renews with fresh groups.
    #[test]
    fn a_finished_group_cup_renews_with_fresh_groups() {
        let mut game = game_with(vec![group_cup()]);

        assert_eq!(renew_finished_cups(&mut game, "2033-05-20"), 1);
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
        let mut continental = cup(8);
        continental.kind = CompetitionType::ContinentalClub;
        continental.scope = CompetitionScope::Continental;
        let mut world_cup = cup(8);
        world_cup.kind = CompetitionType::InternationalNation;
        world_cup.scope = CompetitionScope::International;
        let mut game = game_with(vec![continental, world_cup]);

        assert_eq!(renew_finished_cups(&mut game, "2033-05-20"), 0);
        assert!(game.edition_archive.is_empty());
    }
}
