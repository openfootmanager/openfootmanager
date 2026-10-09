use crate::game::{Game, ScoutedPlayer};
use serde::Serialize;

/// A scout's report plus whether it still describes the player: one that predates the
/// current season's start is out of date. ISO dates order like the days they name.
#[derive(Debug, Serialize)]
pub struct ScoutedReportView {
    #[serde(flatten)]
    pub report: ScoutedPlayer,
    pub out_of_date: bool,
}

impl ScoutedReportView {
    fn of(report: &ScoutedPlayer, season_start: Option<&str>) -> Self {
        Self {
            out_of_date: season_start.is_some_and(|start| report.scouted_on.as_str() < start),
            report: report.clone(),
        }
    }
}

pub fn query_scouted_report(game: &Game, player_id: &str) -> Option<ScoutedReportView> {
    game.scouted_players
        .iter()
        .find(|report| report.player_id == player_id)
        .map(|report| ScoutedReportView::of(report, game.season_context.season_start.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::Utc;
    use domain::manager::Manager;

    fn game_with_report(scouted_on: &str, season_start: Option<&str>) -> Game {
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
        game.season_context.season_start = season_start.map(str::to_string);
        game
    }

    /// Given a player who was scouted
    /// When his report is queried
    /// Then the snapshot comes back, and another player has none
    #[test]
    fn a_scouted_player_has_a_report_and_others_do_not() {
        let game = game_with_report("2026-08-10", Some("2026-08-01"));

        assert_eq!(
            query_scouted_report(&game, "p9").unwrap().report.player_id,
            "p9"
        );
        assert!(query_scouted_report(&game, "p1").is_none());
    }

    /// Given a report from before the current season started
    /// When it is queried
    /// Then it is out of date
    #[test]
    fn a_report_from_before_the_season_start_is_out_of_date() {
        let game = game_with_report("2026-03-10", Some("2026-08-01"));

        assert!(query_scouted_report(&game, "p9").unwrap().out_of_date);
    }

    /// Given a report from the season's first day or later
    /// When it is queried
    /// Then it is current
    #[test]
    fn a_report_from_the_season_start_onward_is_current() {
        assert!(
            !query_scouted_report(&game_with_report("2026-08-01", Some("2026-08-01")), "p9")
                .unwrap()
                .out_of_date
        );
        assert!(
            !query_scouted_report(&game_with_report("2026-09-20", Some("2026-08-01")), "p9")
                .unwrap()
                .out_of_date
        );
    }

    /// Given a game with no known season start
    /// When a report is queried
    /// Then it is not called out of date
    #[test]
    fn no_report_is_out_of_date_without_a_season_start() {
        assert!(
            !query_scouted_report(&game_with_report("2020-01-01", None), "p9")
                .unwrap()
                .out_of_date
        );
    }
}
