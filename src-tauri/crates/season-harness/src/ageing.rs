//! What a season does to players of each age, and who is still good when old (#869).
//!
//! Measurement only: it reports the shape of the development curve (the mean rating change per age
//! band per season) and the age profile of the best players, so a change to how players grow and
//! decline can be read as a before and after against the real-world anchors.

use std::collections::HashMap;

use chrono::NaiveDate;
use domain::age::age_on_or_unknown;
use domain::player::{Player, Position};
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use ofm_core::player_tier::{PlayerTier, player_tier};

use crate::driver::SeasonObserver;

/// The age bands the curve is read in. A player is placed by his age at the start of the season
/// the change was made in.
pub const AGE_BANDS: [(&str, i32, i32); 7] = [
    ("16-18", 16, 18),
    ("19-21", 19, 21),
    ("22-25", 22, 25),
    ("26-29", 26, 29),
    ("30-32", 30, 32),
    ("33-35", 33, 35),
    ("36+", 36, 99),
];

/// How many of the best players the age profile looks at.
const TOP_PLAYERS: usize = 100;
const TOP_FOR_FLATNESS: usize = 50;

/// What one season did to the players of one age band.
#[derive(Debug, Clone, PartialEq)]
pub struct BandChange {
    pub label: &'static str,
    pub players: usize,
    pub mean_change: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgeingReport {
    pub bands: Vec<BandChange>,
    pub aged_33_plus: usize,
    /// Of the players aged 33 or more, the share who are still A or S tier by rating.
    pub aged_33_plus_a_or_s_share: f64,
    pub top100_median_age: f64,
    pub top100_aged_31_plus_share: f64,
    pub top100_aged_33_plus_share: f64,
    pub top100_aged_35_plus_share: f64,
    /// Mean rating of the best 50: it should not drift up or down season after season.
    pub top50_mean_ovr: f64,
    pub top100_goalkeepers_aged_31_plus_share: f64,
    pub top100_outfield_aged_31_plus_share: f64,
}

/// A player's rating and age on the day the last report was made.
pub type Snapshot = HashMap<String, (u8, i32)>;

pub fn snapshot_of(game: &Game) -> Snapshot {
    let on = game.clock.current_date.date_naive();
    game.players
        .iter()
        .filter(|player| !player.retired)
        .map(|player| {
            (
                player.id.clone(),
                (player.ovr, age_on_or_unknown(on, &player.date_of_birth)),
            )
        })
        .collect()
}

/// What `players` look like on `on`, against `previous`. A player in only one of the two (a new
/// signing of the academy, a retiree) has no change to report and is left out of the bands.
pub fn ageing_report(previous: &Snapshot, players: &[Player], on: NaiveDate) -> AgeingReport {
    let aged: Vec<(&Player, i32)> = players
        .iter()
        .filter(|player| !player.retired)
        .map(|player| (player, age_on_or_unknown(on, &player.date_of_birth)))
        .collect();
    AgeingReport {
        bands: band_changes(previous, players),
        aged_33_plus: aged.iter().filter(|(_, age)| *age >= 33).count(),
        aged_33_plus_a_or_s_share: share(
            aged.iter()
                .filter(|(_, age)| *age >= 33)
                .filter(|(player, _)| player_tier(player.ovr) != PlayerTier::Rest)
                .count(),
            aged.iter().filter(|(_, age)| *age >= 33).count(),
        ),
        ..top_players_profile(&aged)
    }
}

fn band_changes(previous: &Snapshot, players: &[Player]) -> Vec<BandChange> {
    AGE_BANDS
        .iter()
        .map(|&(label, from, to)| {
            let changes: Vec<f64> = players
                .iter()
                .filter(|player| !player.retired)
                .filter_map(|player| {
                    let (earlier_ovr, earlier_age) = previous.get(&player.id)?;
                    (from..=to)
                        .contains(earlier_age)
                        .then(|| f64::from(player.ovr) - f64::from(*earlier_ovr))
                })
                .collect();
            BandChange {
                label,
                players: changes.len(),
                mean_change: mean(&changes),
            }
        })
        .collect()
}

/// A keeper by what he is, as development reads it, not by the coarse group he is listed under.
fn is_keeper(player: &Player) -> bool {
    player.natural_position.to_group_position() == Position::Goalkeeper
}

fn top_players_profile(aged: &[(&Player, i32)]) -> AgeingReport {
    let best = |of: Vec<&(&Player, i32)>, count: usize| -> Vec<(u8, i32)> {
        let mut ranked: Vec<(u8, i32)> = of
            .into_iter()
            .map(|(player, age)| (player.ovr, *age))
            .collect();
        ranked.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        ranked.truncate(count);
        ranked
    };
    let top = best(aged.iter().collect(), TOP_PLAYERS);
    let ages: Vec<f64> = top.iter().map(|(_, age)| f64::from(*age)).collect();
    let older_than = |list: &[(u8, i32)], age: i32| {
        share(list.iter().filter(|(_, a)| *a >= age).count(), list.len())
    };
    let top_goalkeepers = best(
        aged.iter()
            .filter(|(player, _)| is_keeper(player))
            .collect(),
        TOP_PLAYERS,
    );
    let top_outfield = best(
        aged.iter()
            .filter(|(player, _)| !is_keeper(player))
            .collect(),
        TOP_PLAYERS,
    );
    let flat: Vec<f64> = top
        .iter()
        .take(TOP_FOR_FLATNESS)
        .map(|(ovr, _)| f64::from(*ovr))
        .collect();
    AgeingReport {
        bands: Vec::new(),
        aged_33_plus: 0,
        aged_33_plus_a_or_s_share: 0.0,
        top100_median_age: median(&ages),
        top100_aged_31_plus_share: older_than(&top, 31),
        top100_aged_33_plus_share: older_than(&top, 33),
        top100_aged_35_plus_share: older_than(&top, 35),
        top50_mean_ovr: mean(&flat),
        top100_goalkeepers_aged_31_plus_share: older_than(&top_goalkeepers, 31),
        top100_outfield_aged_31_plus_share: older_than(&top_outfield, 31),
    }
}

fn share(count: usize, of: usize) -> f64 {
    if of == 0 {
        0.0
    } else {
        count as f64 / of as f64
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[middle]
    } else {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    }
}

/// The report after each season's rollover, in order, each against the season before.
pub struct AgeingLog {
    previous: Snapshot,
    pub after_each_season: Vec<AgeingReport>,
}

impl AgeingLog {
    /// A log that measures the first season against the world as it opens.
    pub fn starting_from(game: &Game) -> Self {
        Self {
            previous: snapshot_of(game),
            after_each_season: Vec::new(),
        }
    }
}

impl SeasonObserver for AgeingLog {
    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {
        let on = game.clock.current_date.date_naive();
        self.after_each_season
            .push(ageing_report(&self.previous, &game.players, on));
        self.previous = snapshot_of(game);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{PlayerAttributes, Position};

    fn on() -> NaiveDate {
        NaiveDate::from_ymd_opt(2030, 7, 1).unwrap()
    }

    fn attributes() -> PlayerAttributes {
        PlayerAttributes {
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
        }
    }

    /// A player aged `age` on `on()` with rating `ovr`.
    fn aged(id: &str, age: i32, ovr: u8, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            "P".to_string(),
            "P P".to_string(),
            format!("{}-01-01", 2030 - age),
            "GB".to_string(),
            position,
            attributes(),
        );
        player.ovr = ovr;
        player
    }

    fn previous(entries: &[(&str, u8, i32)]) -> Snapshot {
        entries
            .iter()
            .map(|(id, ovr, age)| (id.to_string(), (*ovr, *age)))
            .collect()
    }

    fn band<'a>(report: &'a AgeingReport, label: &str) -> &'a BandChange {
        report.bands.iter().find(|b| b.label == label).unwrap()
    }

    /// Given two 17-year-olds who gained 5 and 3 and a 31-year-old who lost 2,
    /// When the season is reported,
    /// Then the 16-18 band's mean change is 4 and the 30-32 band's is -2.
    #[test]
    fn the_mean_change_is_taken_per_band_by_the_age_the_season_began_at() {
        let players = [
            aged("a", 18, 55, Position::Midfielder),
            aged("b", 18, 53, Position::Midfielder),
            aged("c", 32, 68, Position::Midfielder),
        ];
        let before = previous(&[("a", 50, 17), ("b", 50, 17), ("c", 70, 31)]);

        let report = ageing_report(&before, &players, on());

        assert_eq!(band(&report, "16-18").players, 2);
        assert_eq!(band(&report, "16-18").mean_change, 4.0);
        assert_eq!(band(&report, "30-32").mean_change, -2.0);
        assert_eq!(band(&report, "22-25").players, 0);
    }

    /// Given a player who was not there last season (an academy graduate) and one who left,
    /// When the season is reported,
    /// Then neither counts toward a band.
    #[test]
    fn a_player_in_only_one_of_the_two_snapshots_has_no_change_to_report() {
        let players = [aged("new", 17, 60, Position::Midfielder)];
        let before = previous(&[("gone", 70, 30)]);

        let report = ageing_report(&before, &players, on());

        assert!(report.bands.iter().all(|b| b.players == 0));
    }

    /// Given four players aged 33 or more, one rated S and three ordinary (and a star of 25),
    /// When the season is reported,
    /// Then a quarter of the 33+ are still A or S: not the three quarters who are not.
    #[test]
    fn the_share_of_the_old_who_are_still_a_or_s_is_reported() {
        let players = [
            aged("s", 34, 82, Position::Forward),
            aged("a", 36, 74, Position::Forward),
            aged("x", 33, 65, Position::Forward),
            aged("y", 35, 60, Position::Forward),
            aged("young", 25, 90, Position::Forward),
        ];

        let report = ageing_report(&Snapshot::new(), &players, on());

        assert_eq!(report.aged_33_plus, 4);
        assert_eq!(report.aged_33_plus_a_or_s_share, 0.25);
    }

    /// Given players of different ratings and ages,
    /// When the season is reported,
    /// Then the profile is taken over the best by rating: the median age, the shares from 31, 33
    ///      and 35, and the mean rating of the top.
    #[test]
    fn the_age_profile_is_that_of_the_best_players_by_rating() {
        let players = [
            aged("p1", 24, 90, Position::Forward),
            aged("p2", 26, 88, Position::Forward),
            aged("p3", 32, 86, Position::Forward),
            aged("p4", 34, 84, Position::Forward),
            aged("p5", 36, 82, Position::Forward),
            aged("weak-old", 40, 40, Position::Forward),
        ];

        let report = ageing_report(&Snapshot::new(), &players, on());

        assert_eq!(report.top100_median_age, 33.0);
        assert!((report.top100_aged_31_plus_share - 4.0 / 6.0).abs() < 1e-9);
        assert!((report.top100_aged_33_plus_share - 3.0 / 6.0).abs() < 1e-9);
        assert!((report.top100_aged_35_plus_share - 2.0 / 6.0).abs() < 1e-9);
        assert!((report.top50_mean_ovr - 470.0 / 6.0).abs() < 1e-9);
    }

    /// Given goalkeepers who are older than the outfield players,
    /// When the season is reported,
    /// Then the 31+ share is read separately for goalkeepers and for outfield players.
    #[test]
    fn goalkeepers_and_outfield_players_are_aged_separately() {
        let players = [
            aged("gk1", 34, 80, Position::Goalkeeper),
            aged("gk2", 33, 78, Position::Goalkeeper),
            aged("o1", 25, 85, Position::Forward),
            aged("o2", 32, 84, Position::Forward),
        ];

        let report = ageing_report(&Snapshot::new(), &players, on());

        assert_eq!(report.top100_goalkeepers_aged_31_plus_share, 1.0);
        assert_eq!(report.top100_outfield_aged_31_plus_share, 0.5);
    }

    /// Given a world and two season ends told to the log,
    /// When the log is read,
    /// Then the first report is against the world as it opened and the second against the first.
    #[test]
    fn the_log_measures_each_season_against_the_one_before() {
        let mut game = crate::worlds::WorldSpec::gate(1)
            .build()
            .expect("the world builds");
        let mut log = AgeingLog::starting_from(&game);
        let summary = EndOfSeasonSummary::default();
        let started = game.players[0].ovr;

        game.players[0].ovr = started.saturating_add(4);
        log.on_season_end(&game, &summary, 1);
        game.players[0].ovr = started.saturating_add(4).saturating_add(2);
        log.on_season_end(&game, &summary, 2);

        let changed = |report: &AgeingReport| -> f64 {
            report
                .bands
                .iter()
                .map(|b| b.mean_change * b.players as f64)
                .sum::<f64>()
        };
        assert_eq!(log.after_each_season.len(), 2);
        assert_eq!(changed(&log.after_each_season[0]), 4.0);
        assert_eq!(changed(&log.after_each_season[1]), 2.0);
    }

    /// Given a player who has retired and is still on the roster (as in a save from before the
    ///       archive) and one who plays,
    /// When the season is reported,
    /// Then the retiree counts for nothing: no band, no age profile.
    #[test]
    fn a_retiree_still_on_the_roster_is_not_measured() {
        let mut retired = aged("retired", 38, 90, Position::Forward);
        retired.retired = true;
        let playing = aged("playing", 30, 70, Position::Forward);
        let before = previous(&[("retired", 90, 37), ("playing", 70, 29)]);

        let report = ageing_report(&before, &[retired, playing], on());

        assert_eq!(band(&report, "36+").players, 0);
        assert_eq!(band(&report, "26-29").players, 1);
        assert_eq!(report.top100_median_age, 30.0);
        assert_eq!(report.top50_mean_ovr, 70.0);
    }
}
