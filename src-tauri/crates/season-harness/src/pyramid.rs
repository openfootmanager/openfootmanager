//! How old the world's players are (#869, slice 3).
//!
//! Measurement only: nothing here judges the numbers. Players are placed in five age bands and the
//! report gives each band's share of all players and of the seniors, so a change to the youth
//! intake, to retirement ages or to the ages generation draws can be read as a before and after.
//! The age profile of the best 100 players is `ageing::ageing_report`'s.

use domain::age::age_on_or_unknown;
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;

use crate::distribution::SENIOR_MIN_AGE;
use crate::driver::SeasonObserver;

/// The bands, as `(label, youngest, oldest)` in years. A player is placed by his age on the day.
pub const PYRAMID_BANDS: [(&str, i32, i32); 5] = [
    ("<=20", 0, 20),
    ("21-24", 21, 24),
    ("25-28", 25, 28),
    ("29-32", 29, 32),
    ("33+", 33, i32::MAX),
];

/// One band of the pyramid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandShare {
    pub label: &'static str,
    pub players: usize,
    /// Of every player in the world.
    pub share_of_players: f64,
    /// Of the players aged `SENIOR_MIN_AGE` or more; zero for the band below that age.
    pub share_of_seniors: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgePyramid {
    pub players: usize,
    pub seniors: usize,
    pub bands: Vec<BandShare>,
}

/// The pyramid of `game` on its own date. Retired players are not counted.
pub fn age_pyramid(game: &Game) -> AgePyramid {
    let on = game.clock.current_date.date_naive();
    let ages: Vec<i32> = game
        .players
        .iter()
        .filter(|player| !player.retired)
        .map(|player| age_on_or_unknown(on, &player.date_of_birth))
        .collect();
    let seniors = ages.iter().filter(|age| **age >= SENIOR_MIN_AGE).count();
    let share = |count: usize, of: usize| {
        if of == 0 {
            0.0
        } else {
            count as f64 / of as f64
        }
    };
    let bands = PYRAMID_BANDS
        .iter()
        .map(|&(label, youngest, oldest)| {
            let players = ages
                .iter()
                .filter(|age| (youngest..=oldest).contains(*age))
                .count();
            let senior_players = if youngest >= SENIOR_MIN_AGE {
                players
            } else {
                0
            };
            BandShare {
                label,
                players,
                share_of_players: share(players, ages.len()),
                share_of_seniors: share(senior_players, seniors),
            }
        })
        .collect();
    AgePyramid {
        players: ages.len(),
        seniors,
        bands,
    }
}

/// The pyramid after each season's rollover, in order.
#[derive(Default)]
pub struct PyramidLog {
    pub after_each_season: Vec<AgePyramid>,
}

impl SeasonObserver for PyramidLog {
    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {
        self.after_each_season.push(age_pyramid(game));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use ofm_core::clock::GameClock;

    fn player(born: &str) -> Player {
        let attributes = PlayerAttributes {
            pace: 50,
            stamina: 50,
            strength: 50,
            agility: 50,
            passing: 50,
            shooting: 50,
            tackling: 50,
            dribbling: 50,
            defending: 50,
            positioning: 50,
            vision: 50,
            decisions: 50,
            composure: 50,
            aggression: 50,
            teamwork: 50,
            leadership: 50,
            handling: 50,
            reflexes: 50,
            aerial: 50,
        };
        Player::new(
            format!("p-{born}"),
            "P".to_string(),
            "P P".to_string(),
            born.to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attributes,
        )
    }

    /// The world's date is 1 July 2026.
    fn world(players: Vec<Player>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".to_string(),
            "M".to_string(),
            "M".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(clock, manager, vec![], players, vec![], vec![])
    }

    fn band<'a>(pyramid: &'a AgePyramid, label: &str) -> &'a BandShare {
        pyramid.bands.iter().find(|b| b.label == label).unwrap()
    }

    /// Given the log watching a season end,
    /// When the season ends,
    /// Then it keeps that season's pyramid.
    #[test]
    fn the_log_keeps_a_pyramid_for_each_season_end() {
        let game = world(vec![player("1990-01-01")]);
        let mut log = PyramidLog::default();

        log.on_season_end(&game, &EndOfSeasonSummary::default(), 1);

        assert_eq!(log.after_each_season, vec![age_pyramid(&game)]);
    }

    /// Given players aged 19, 20, 21, 24, 25, 28, 29, 32, 33 and 38,
    /// When the pyramid is read,
    /// Then each falls in its own band, the edges included.
    #[test]
    fn every_age_falls_in_the_band_it_names() {
        let ages = [19, 20, 21, 24, 25, 28, 29, 32, 33, 38];
        let pyramid = age_pyramid(&world(
            ages.iter()
                .map(|age| player(&format!("{}-01-01", 2026 - age)))
                .collect(),
        ));

        for label in ["<=20", "21-24", "25-28", "29-32"] {
            assert_eq!(band(&pyramid, label).players, 2, "{label}");
        }
        assert_eq!(band(&pyramid, "33+").players, 2);
        assert_eq!(pyramid.players, 10);
    }

    /// Given two players of 18 and 19 and six seniors, two of them over 33,
    /// When the pyramid is read,
    /// Then the shares of all players use eight as the base, and those of the seniors use six.
    #[test]
    fn shares_of_players_and_of_seniors_have_different_bases() {
        let born = |age: i32| player(&format!("{}-01-01", 2026 - age));
        let pyramid = age_pyramid(&world(vec![
            born(18),
            born(19),
            born(22),
            born(26),
            born(30),
            born(31),
            born(34),
            born(36),
        ]));

        assert_eq!(pyramid.seniors, 6);
        assert_eq!(band(&pyramid, "<=20").share_of_players, 0.25);
        assert_eq!(band(&pyramid, "<=20").share_of_seniors, 0.0);
        assert_eq!(band(&pyramid, "33+").share_of_players, 0.25);
        assert!((band(&pyramid, "33+").share_of_seniors - 2.0 / 6.0).abs() < 1e-12);
        let senior_total: f64 = pyramid.bands.iter().map(|b| b.share_of_seniors).sum();
        assert!((senior_total - 1.0).abs() < 1e-12);
    }

    /// Given a retiree still on the roster, when the pyramid is read, then he is in no band.
    #[test]
    fn a_retiree_is_not_in_the_pyramid() {
        let mut retired = player("1980-01-01");
        retired.retired = true;

        let pyramid = age_pyramid(&world(vec![retired, player("1995-01-01")]));

        assert_eq!(pyramid.players, 1);
        assert_eq!(band(&pyramid, "33+").players, 0);
    }

    /// Given no players, when the pyramid is read, then every figure is zero rather than NaN.
    #[test]
    fn an_empty_world_reports_zeros() {
        let pyramid = age_pyramid(&world(vec![]));

        assert_eq!(pyramid.players, 0);
        assert!(
            pyramid
                .bands
                .iter()
                .all(|b| b.share_of_players == 0.0 && b.share_of_seniors == 0.0)
        );
    }
}
