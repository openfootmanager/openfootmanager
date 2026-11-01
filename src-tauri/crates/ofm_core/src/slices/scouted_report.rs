use crate::game::{Game, ScoutedPlayer};
use serde::Serialize;

/// A scout's report plus whether it still describes the player. Players age and develop once a
/// season, at the rollover, so a report taken before the last rollover is out of date and one taken
/// in preseason, after it, is not. ISO dates order like the days they name.
#[derive(Debug, Serialize)]
pub struct ScoutedReportView {
    #[serde(flatten)]
    pub report: ScoutedPlayer,
    pub out_of_date: bool,
}

impl ScoutedReportView {
    fn of(report: &ScoutedPlayer, last_rollover: Option<&str>) -> Self {
        Self {
            out_of_date: is_out_of_date(report, last_rollover),
            report: report.clone(),
        }
    }
}

/// The report was taken before the last rollover, so it no longer describes the player.
pub(crate) fn is_out_of_date(report: &ScoutedPlayer, last_rollover: Option<&str>) -> bool {
    last_rollover.is_some_and(|rollover| report.scouted_on.as_str() < rollover)
}

pub fn query_scouted_report(game: &Game, player_id: &str) -> Option<ScoutedReportView> {
    game.scouted_players
        .iter()
        .find(|report| report.player_id == player_id)
        .map(|report| ScoutedReportView::of(report, game.last_season_rollover_on.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::Utc;
    use domain::manager::Manager;

    // Every game here has a season that starts on 1 August, after the rollover, so a test that
    // passes cannot be reading the season start.
    fn game_with_report(scouted_on: &str, last_rollover: Option<&str>) -> Game {
        let clock = GameClock {
            current_date: Utc::now(),
            start_date: Utc::now(),
        };
        let manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.scouted_players.push(ScoutedPlayer {
            player_id: "p9".to_string(),
            scouted_on: scouted_on.to_string(),
            attributes: serde_json::from_str(r#"{"pace":50,"stamina":50,"strength":50,"agility":50,"passing":50,"shooting":50,"tackling":50,"dribbling":50,"defending":50,"positioning":50,"vision":50,"decisions":50,"composure":50,"aggression":50,"teamwork":50,"leadership":50,"handling":50,"reflexes":50,"aerial":50}"#).unwrap(),
        });
        game.season_context.season_start = Some("2026-08-01".to_string());
        game.last_season_rollover_on = last_rollover.map(str::to_string);
        game
    }

    fn is_out_of_date(game: &Game) -> bool {
        query_scouted_report(game, "p9").unwrap().out_of_date
    }

    /// Given a player who was scouted
    /// When his report is queried
    /// Then the snapshot comes back, and another player has none
    #[test]
    fn a_scouted_player_has_a_report_and_others_do_not() {
        let game = game_with_report("2026-08-10", Some("2026-05-25"));

        assert_eq!(
            query_scouted_report(&game, "p9").unwrap().report.player_id,
            "p9"
        );
        assert!(query_scouted_report(&game, "p1").is_none());
    }

    /// Given a season that rolled over on 25 May, and a report taken in preseason on 4 June
    /// When it is queried on 15 September, after the league's 1 August start
    /// Then it is current: the season's growth applied before it was taken
    #[test]
    fn a_preseason_report_taken_after_the_rollover_is_current() {
        assert!(!is_out_of_date(&game_with_report(
            "2026-06-04",
            Some("2026-05-25")
        )));
    }

    /// Given a report taken on 10 April, before the season rolled over on 25 May
    /// When it is queried
    /// Then it is out of date: the player has aged and the season's growth applied since
    #[test]
    fn a_report_from_before_the_last_rollover_is_out_of_date() {
        assert!(is_out_of_date(&game_with_report(
            "2026-04-10",
            Some("2026-05-25")
        )));
    }

    /// Given a report taken on the day the season rolled over
    /// When it is queried
    /// Then it is current
    #[test]
    fn a_report_from_the_rollover_day_is_current() {
        assert!(!is_out_of_date(&game_with_report(
            "2026-05-25",
            Some("2026-05-25")
        )));
    }

    /// Given a career that has not reached a season end yet (or a save from before it was kept)
    /// When a report from long before the season start is queried
    /// Then it is current: no rollover has happened since it was taken
    #[test]
    fn no_report_is_out_of_date_before_any_rollover() {
        assert!(!is_out_of_date(&game_with_report("2020-01-01", None)));
    }
}
