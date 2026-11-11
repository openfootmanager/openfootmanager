//! Recording one match in the two participating rows, shared by club and national tables.

use domain::league::StandingEntry;

pub(crate) fn record_result(
    standings: &mut [StandingEntry],
    home_team_id: &str,
    away_team_id: &str,
    home_goals: u8,
    away_goals: u8,
) {
    for (team_id, scored, conceded) in [
        (home_team_id, home_goals, away_goals),
        (away_team_id, away_goals, home_goals),
    ] {
        if let Some(entry) = standings.iter_mut().find(|entry| entry.team_id == team_id) {
            entry.record_result(scored, conceded);
        }
    }
}
