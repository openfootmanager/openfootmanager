//! Small pure formatting and clock helpers shared across the game module
//! (league naming, save naming, date formatting, preseason anchors). Kept
//! dependency-free so every other cluster can import them without back-refs.

use chrono::{Datelike, Duration, Utc};
use ofm_core::clock::GameClock;

pub(crate) fn default_league_name() -> String {
    ["Premier", "Division"].join(" ")
}

pub(crate) fn long_date_format() -> String {
    ['%', 'B', ' ', '%', 'd', ',', ' ', '%', 'Y']
        .into_iter()
        .collect()
}

pub(crate) fn default_save_name(manager_name: &str) -> String {
    let mut save_name = manager_name.to_string();
    save_name.push('\'');
    save_name.push('s');
    save_name.push(' ');
    save_name.push_str("Career");
    save_name
}

pub(crate) fn preseason_season_start(clock: &GameClock) -> chrono::DateTime<Utc> {
    clock.start_date + Duration::days(30)
}

pub(crate) fn preseason_league_year(clock: &GameClock) -> u32 {
    let year = clock.start_date.year() + i32::from(clock.start_date.month() == 12);
    u32::try_from(year).unwrap_or(2020)
}
