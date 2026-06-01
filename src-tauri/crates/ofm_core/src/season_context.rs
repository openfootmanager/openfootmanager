use crate::end_of_season::is_league_complete;
use crate::game::Game;
use chrono::{Duration, NaiveDate};
use domain::league::League;
use domain::season::{SeasonContext, SeasonPhase, TransferWindowContext, TransferWindowStatus};

const TRANSFER_WINDOW_PRESEASON_DAYS: i64 = 30;
const TRANSFER_WINDOW_POST_START_DAYS: i64 = 30;

pub fn refresh_game_context(game: &mut Game) {
    game.season_context = derive_season_context(game);
}

pub fn derive_season_context(game: &Game) -> SeasonContext {
    let Some(league) = &game.league else {
        return SeasonContext::default();
    };

    let season_start = league_boundary_date(league, Boundary::Start);
    let season_end = league_boundary_date(league, Boundary::End);
    let current_date = game.clock.current_date.date_naive();

    let phase = if is_league_complete(league) {
        SeasonPhase::PostSeason
    } else if league_has_started(league) {
        SeasonPhase::InSeason
    } else {
        SeasonPhase::Preseason
    };

    let days_until_season_start = season_start.and_then(|start| {
        let days = (start - current_date).num_days();
        (days >= 0).then_some(days)
    });

    let transfer_window = derive_transfer_window_context(current_date, season_start);

    SeasonContext {
        phase,
        season_start: season_start.map(format_date),
        season_end: season_end.map(format_date),
        days_until_season_start,
        transfer_window,
    }
}

#[derive(Copy, Clone)]
enum Boundary {
    Start,
    End,
}

fn league_boundary_date(league: &League, boundary: Boundary) -> Option<NaiveDate> {
    league
        .fixtures
        .iter()
        .filter(|fixture| fixture.counts_for_league_standings())
        .filter_map(|fixture| NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
        .reduce(|left, right| match boundary {
            Boundary::Start => left.min(right),
            Boundary::End => left.max(right),
        })
}

fn league_has_started(league: &League) -> bool {
    league.standings.iter().any(|entry| entry.played > 0)
        || league.fixtures.iter().any(|fixture| {
            fixture.counts_for_league_standings()
                && fixture.status == domain::league::FixtureStatus::Completed
        })
}

fn derive_transfer_window_context(
    current_date: NaiveDate,
    season_start: Option<NaiveDate>,
) -> TransferWindowContext {
    let Some(season_start) = season_start else {
        return TransferWindowContext::default();
    };

    let opens_on = season_start - Duration::days(TRANSFER_WINDOW_PRESEASON_DAYS);
    let closes_on = season_start + Duration::days(TRANSFER_WINDOW_POST_START_DAYS);

    let (status, days_until_opens, days_remaining) = if current_date < opens_on {
        (
            TransferWindowStatus::Closed,
            Some((opens_on - current_date).num_days()),
            None,
        )
    } else if current_date > closes_on {
        (TransferWindowStatus::Closed, None, None)
    } else {
        let remaining = (closes_on - current_date).num_days();
        let status = if remaining == 0 {
            TransferWindowStatus::DeadlineDay
        } else {
            TransferWindowStatus::Open
        };
        (status, None, Some(remaining))
    };

    TransferWindowContext {
        status,
        opens_on: Some(format_date(opens_on)),
        closes_on: Some(format_date(closes_on)),
        days_until_opens,
        days_remaining,
    }
}

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::derive_season_context;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
    };
    use domain::manager::Manager;
    use domain::season::{SeasonPhase, TransferWindowStatus};
    use domain::team::Team;

    fn make_team(id: &str, name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "England".to_string(),
            "Test City".to_string(),
            format!("{} Ground", name),
            20_000,
        )
    }

    fn make_fixture(id: &str, date: &str, status: FixtureStatus, matchday: u32) -> Fixture {
        Fixture {
            id: id.to_string(),
            matchday,
            date: date.to_string(),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: status.clone(),
            result: (status == FixtureStatus::Completed).then_some(MatchResult {
                home_goals: 1,
                away_goals: 0,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
            }),
        }
    }

    fn make_game(current_date: (i32, u32, u32), league: Option<League>) -> Game {
        let clock = GameClock::new(
            Utc.with_ymd_and_hms(current_date.0, current_date.1, current_date.2, 12, 0, 0)
                .unwrap(),
        );
        let manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(
            clock,
            manager,
            vec![
                make_team("team1", "Alpha FC"),
                make_team("team2", "Beta FC"),
            ],
            vec![],
            vec![],
            vec![],
        );
        game.league = league;
        game
    }

    #[test]
    fn derives_preseason_context_before_first_fixture() {
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![make_fixture(
                "fx1",
                "2026-08-01",
                FixtureStatus::Scheduled,
                1,
            )],
            standings: vec![
                StandingEntry::new("team1".to_string()),
                StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
        };
        let game = make_game((2026, 7, 10), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(context.phase, SeasonPhase::Preseason);
        assert_eq!(context.season_start.as_deref(), Some("2026-08-01"));
        assert_eq!(context.days_until_season_start, Some(22));
        assert_eq!(context.transfer_window.status, TransferWindowStatus::Open);
        assert_eq!(context.transfer_window.days_remaining, Some(52));
    }

    #[test]
    fn derives_deadline_day_window_status() {
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![make_fixture(
                "fx1",
                "2026-08-01",
                FixtureStatus::Scheduled,
                1,
            )],
            standings: vec![
                StandingEntry::new("team1".to_string()),
                StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
        };
        let game = make_game((2026, 8, 31), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(
            context.transfer_window.status,
            TransferWindowStatus::DeadlineDay
        );
        assert_eq!(context.transfer_window.days_remaining, Some(0));
    }

    #[test]
    fn derives_in_season_context_after_matches_begin() {
        let mut alpha = StandingEntry::new("team1".to_string());
        alpha.record_result(2, 1);
        let mut beta = StandingEntry::new("team2".to_string());
        beta.record_result(1, 2);
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![make_fixture(
                "fx1",
                "2026-08-01",
                FixtureStatus::Completed,
                1,
            )],
            standings: vec![alpha, beta],
            transfer_log: vec![],
            transfer_rumours: vec![],
        };
        let game = make_game((2026, 8, 5), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(context.phase, SeasonPhase::InSeason);
        assert_eq!(context.days_until_season_start, None);
        assert_eq!(context.transfer_window.status, TransferWindowStatus::Open);
    }

    #[test]
    fn derives_postseason_context_once_league_is_complete() {
        let mut alpha = StandingEntry::new("team1".to_string());
        alpha.record_result(2, 1);
        let mut beta = StandingEntry::new("team2".to_string());
        beta.record_result(1, 2);
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![
                make_fixture("fx1", "2026-08-01", FixtureStatus::Completed, 1),
                make_fixture("fx2", "2026-08-08", FixtureStatus::Completed, 2),
            ],
            standings: vec![alpha, beta],
            transfer_log: vec![],
            transfer_rumours: vec![],
        };
        let game = make_game((2026, 8, 9), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(context.phase, SeasonPhase::PostSeason);
        assert_eq!(context.season_end.as_deref(), Some("2026-08-08"));
    }
}
