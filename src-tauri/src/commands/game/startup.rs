//! What the player chooses before a career exists: the start year, the phase
//! they join in, and how much past history to generate behind them.
//!
//! `RawStartupOptions` is the shape the frontend sends; `StartupOptions` is
//! the validated shape everything after this point may rely on, and
//! `normalize_startup_options` is the only bridge between them.

use chrono::{Datelike, Duration, TimeZone, Utc};

use ofm_core::clock::GameClock;
use ofm_core::game::Game;

pub(super) const DEFAULT_GENERATED_HISTORY_DEPTH_YEARS: u32 = 12;
pub(super) const MAX_GENERATED_HISTORY_DEPTH_YEARS: u32 = 24;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStartupOptions {
    #[serde(default)]
    start_year: Option<i32>,
    #[serde(default)]
    start_phase: Option<String>,
    #[serde(default)]
    history_depth_years: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartPhase {
    SeasonStart,
    MidSeason,
}

impl StartPhase {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "seasonStart" => Some(Self::SeasonStart),
            "midSeason" => Some(Self::MidSeason),
            _ => None,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::SeasonStart => "seasonStart",
            Self::MidSeason => "midSeason",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartupOptions {
    pub(super) start_year: i32,
    pub(super) start_phase: StartPhase,
    pub(super) history_depth_years: u32,
}

/// Earliest year a career may start. Historical world packages recreate eras
/// decades before the modern game — a 1962 Santos world is the motivating case —
/// so the floor only needs to keep the clock inside a sane calendar range.
/// Must match `MIN_CAREER_START_YEAR` in `src/pages/MainMenu.tsx`.
pub(super) const MIN_START_YEAR: i32 = 1900;

fn default_start_year() -> i32 {
    chrono::Utc::now().year().max(MIN_START_YEAR)
}

fn default_history_depth_years() -> u32 {
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
}

pub(super) fn start_date_for_year(start_year: i32) -> Result<chrono::DateTime<Utc>, String> {
    // Use June 1 in World Cup years so a fresh career opens just before the
    // tournament, keeping the WC in June rather than scheduling it in July.
    let month = if ofm_core::world_cup::is_world_cup_summer(start_year) {
        6
    } else {
        7
    };
    Utc.with_ymd_and_hms(start_year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "be.error.createManager.invalidStartYear".to_string())
}

pub(super) fn current_date_for_phase(
    start_year: i32,
    start_phase: StartPhase,
) -> Result<chrono::DateTime<Utc>, String> {
    let start_date = start_date_for_year(start_year)?;
    Ok(match start_phase {
        StartPhase::SeasonStart => start_date,
        StartPhase::MidSeason => start_date + Duration::days(120),
    })
}

pub(super) fn age_on_date(birth_date: chrono::NaiveDate, reference_date: chrono::NaiveDate) -> i64 {
    let mut age = i64::from(reference_date.year() - birth_date.year());
    let has_had_birthday =
        (reference_date.month(), reference_date.day()) >= (birth_date.month(), birth_date.day());
    if !has_had_birthday {
        age -= 1;
    }
    age
}

pub(crate) fn start_phase_for_game(game: &Game) -> StartPhase {
    if game.clock.current_date > game.clock.start_date {
        StartPhase::MidSeason
    } else {
        StartPhase::SeasonStart
    }
}

pub(super) fn preseason_season_start(clock: &GameClock) -> chrono::DateTime<Utc> {
    clock.start_date + Duration::days(30)
}

pub(super) fn preseason_league_year(clock: &GameClock) -> u32 {
    let year = clock.start_date.year() + i32::from(clock.start_date.month() == 12);
    // Only reachable for a negative year, which the start-year floor rules out.
    u32::try_from(year).unwrap_or(MIN_START_YEAR as u32)
}

pub(super) fn normalize_startup_options(
    raw: Option<RawStartupOptions>,
) -> Result<StartupOptions, String> {
    let raw = raw.unwrap_or_default();
    let start_year = raw.start_year.unwrap_or_else(default_start_year);
    if start_year < MIN_START_YEAR {
        return Err("be.error.createManager.startYearMin".to_string());
    }

    let start_phase = match raw.start_phase.as_deref() {
        None | Some("") => StartPhase::SeasonStart,
        Some(value) => StartPhase::parse(value)
            .ok_or_else(|| "be.error.createManager.invalidStartPhase".to_string())?,
    };
    let history_depth_years = raw
        .history_depth_years
        .unwrap_or_else(default_history_depth_years);
    if history_depth_years > MAX_GENERATED_HISTORY_DEPTH_YEARS {
        return Err("be.error.createManager.historyDepthMax".to_string());
    }

    Ok(StartupOptions {
        start_year,
        start_phase,
        history_depth_years,
    })
}

pub(super) fn apply_generated_past_history(game: &mut Game, startup_options: &StartupOptions) {
    ofm_core::history_generation::generate_past_world_history(
        game,
        startup_options.start_year,
        startup_options.history_depth_years,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::game::game_clock_for_world;
    use crate::commands::game::testkit::make_historical_snapshot_world;
    use ofm_core::season_context::refresh_game_context;

    #[test]
    fn normalize_startup_options_defaults_to_current_year_and_season_start() {
        let options = normalize_startup_options(None).unwrap();

        use chrono::Datelike;

        // Defaulting still means "today", not the floor — a fresh career should
        // open in the current year even though historical years are now legal.
        assert_eq!(options.start_year, chrono::Utc::now().year());
        assert_eq!(options.start_phase, StartPhase::SeasonStart);
        assert_eq!(
            options.history_depth_years,
            DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
        );
    }

    #[test]
    fn normalize_startup_options_rejects_years_before_the_floor() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(MIN_START_YEAR - 1),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(result.unwrap_err(), "be.error.createManager.startYearMin");
    }

    #[test]
    fn normalize_startup_options_accepts_historical_start_years() {
        // A 1962 career is the motivating case: historical world packages need a
        // clock that predates the modern era by decades.
        let options = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(1962),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: None,
        }))
        .expect("1962 is above the floor and must be accepted");

        assert_eq!(options.start_year, 1962);
    }

    #[test]
    fn start_date_for_historical_world_cup_year_opens_in_june() {
        use chrono::Datelike;

        let start = start_date_for_year(1962).expect("1962 is a valid start year");

        assert_eq!(start.year(), 1962);
        assert_eq!(start.month(), 6);
    }

    #[test]
    fn normalize_startup_options_rejects_unknown_start_phase() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("playoffs".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartPhase"
        );
    }

    #[test]
    fn normalize_startup_options_rejects_history_depths_above_maximum() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(MAX_GENERATED_HISTORY_DEPTH_YEARS + 1),
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.historyDepthMax"
        );
    }

    #[test]
    fn normalize_startup_options_accepts_custom_history_depth() {
        let options = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(24),
        }))
        .unwrap();

        assert_eq!(options.history_depth_years, 24);
    }

    #[test]
    fn start_date_for_year_uses_selected_july_first() {
        let start_date = start_date_for_year(2032).unwrap();

        assert_eq!(start_date.to_rfc3339(), "2032-07-01T00:00:00+00:00");
    }

    #[test]
    fn start_date_for_year_rejects_out_of_range_years() {
        let result = start_date_for_year(i32::MAX);

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
    }

    #[test]
    fn current_date_for_midseason_phase_is_after_start_date() {
        let current_date = current_date_for_phase(2032, StartPhase::MidSeason).unwrap();

        assert_eq!(current_date.to_rfc3339(), "2032-10-29T00:00:00+00:00");
    }

    #[test]
    fn age_on_date_uses_selected_start_year() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 1, 1).unwrap();
        let reference_date = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();

        assert_eq!(age_on_date(birth_date, reference_date), 30);
    }

    #[test]
    fn age_on_date_changes_between_season_start_and_midseason() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 8, 1).unwrap();
        let season_start = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();
        let midseason = current_date_for_phase(2038, StartPhase::MidSeason)
            .unwrap()
            .date_naive();

        assert_eq!(age_on_date(birth_date, season_start), 29);
        assert_eq!(age_on_date(birth_date, midseason), 30);
    }

    #[test]
    fn age_on_date_uses_world_snapshot_date_over_startup_phase() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let reference_date = game_clock_for_world(&startup_options, &world.metadata)
            .unwrap()
            .current_date
            .date_naive();
        let birth_date = chrono::NaiveDate::from_ymd_opt(2001, 12, 15).unwrap();

        assert_eq!(reference_date.to_string(), "2031-11-20");
        assert_eq!(age_on_date(birth_date, reference_date), 29);
    }

    #[test]
    fn preseason_league_setup_uses_selected_start_year_for_context() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let season_start = preseason_season_start(&game.clock);
        let team_ids = game
            .teams
            .iter()
            .map(|team| team.id.clone())
            .collect::<Vec<_>>();
        game.league = Some(ofm_core::schedule::generate_league(
            "Premier Division",
            preseason_league_year(&game.clock),
            &team_ids,
            season_start,
        ));
        refresh_game_context(&mut game);

        assert_eq!(
            game.clock.start_date.to_rfc3339(),
            "2032-07-01T00:00:00+00:00"
        );
        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2032));
        assert_eq!(
            game.season_context.season_start.as_deref(),
            Some("2032-07-31")
        );
        assert_eq!(game.season_context.days_until_season_start, Some(30));
    }

    #[test]
    fn apply_generated_past_history_populates_default_twelve_prior_seasons() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let staff = vec![
            {
                let mut staff = domain::staff::Staff::new(
                    "staff1".to_string(),
                    "Pat".to_string(),
                    "Coach".to_string(),
                    "1978-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 70,
                        judging_ability: 65,
                        judging_potential: 64,
                        physiotherapy: 40,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team1".to_string());
                staff
            },
            {
                let mut staff = domain::staff::Staff::new(
                    "staff2".to_string(),
                    "Lee".to_string(),
                    "Coach".to_string(),
                    "1979-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 72,
                        judging_ability: 66,
                        judging_potential: 65,
                        physiotherapy: 39,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team2".to_string());
                staff
            },
        ];
        let players = vec![
            {
                let mut player = domain::player::Player::new(
                    "player1".to_string(),
                    "A. Keeper".to_string(),
                    "Alex Keeper".to_string(),
                    "1994-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 48,
                        stamina: 62,
                        strength: 64,
                        agility: 66,
                        passing: 50,
                        shooting: 20,
                        tackling: 18,
                        dribbling: 32,
                        defending: 24,
                        positioning: 68,
                        vision: 48,
                        decisions: 63,
                        composure: 61,
                        aggression: 38,
                        teamwork: 64,
                        leadership: 58,
                        handling: 76,
                        reflexes: 77,
                        aerial: 72,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 68;
                player.potential = 73;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player2".to_string(),
                    "A. Striker".to_string(),
                    "Alex Striker".to_string(),
                    "1996-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 72,
                        stamina: 68,
                        strength: 70,
                        agility: 71,
                        passing: 60,
                        shooting: 79,
                        tackling: 34,
                        dribbling: 73,
                        defending: 28,
                        positioning: 74,
                        vision: 62,
                        decisions: 68,
                        composure: 69,
                        aggression: 52,
                        teamwork: 64,
                        leadership: 47,
                        handling: 18,
                        reflexes: 18,
                        aerial: 61,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 74;
                player.potential = 80;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player3".to_string(),
                    "B. Keeper".to_string(),
                    "Ben Keeper".to_string(),
                    "1993-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 47,
                        stamina: 61,
                        strength: 63,
                        agility: 65,
                        passing: 49,
                        shooting: 19,
                        tackling: 18,
                        dribbling: 30,
                        defending: 23,
                        positioning: 67,
                        vision: 47,
                        decisions: 62,
                        composure: 60,
                        aggression: 39,
                        teamwork: 63,
                        leadership: 57,
                        handling: 75,
                        reflexes: 76,
                        aerial: 71,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 67;
                player.potential = 72;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player4".to_string(),
                    "B. Striker".to_string(),
                    "Ben Striker".to_string(),
                    "1995-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 71,
                        stamina: 67,
                        strength: 69,
                        agility: 70,
                        passing: 59,
                        shooting: 78,
                        tackling: 33,
                        dribbling: 72,
                        defending: 27,
                        positioning: 73,
                        vision: 61,
                        decisions: 67,
                        composure: 68,
                        aggression: 51,
                        teamwork: 63,
                        leadership: 46,
                        handling: 18,
                        reflexes: 18,
                        aerial: 60,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 73;
                player.potential = 79;
                player
            },
        ];
        let mut game = Game::new(clock, manager, teams, players, staff, vec![]);

        apply_generated_past_history(
            &mut game,
            &StartupOptions {
                start_year: 2032,
                start_phase: StartPhase::SeasonStart,
                history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
            },
        );

        assert!(game.teams.iter().all(|team| team.history.len() == 12));
        assert_eq!(game.world_history.season_awards.len(), 12);
        assert!(game.players.iter().any(|player| player.career.len() == 12));
        assert!(game
            .managers
            .iter()
            .any(|manager| !manager.career_history.is_empty()));
    }
}
