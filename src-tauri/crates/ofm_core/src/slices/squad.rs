use crate::game::Game;
use crate::match_day_eligibility::match_day_eligible_players;
use crate::training::{ConditionStatus, condition_status};
use domain::player::Player;
use serde::Serialize;
use std::collections::HashSet;
use ts_rs::TS;

/// Read-only squad projection: eligibility is computed, never persisted on Player.
#[derive(Debug, Serialize, TS)]
pub struct SquadPlayer {
    #[serde(flatten)]
    pub player: Player,
    #[serde(default)]
    pub match_day_eligible: bool,
    /// The band the player's condition is in, with the word and tone the screens show it in.
    pub condition_status: ConditionStatus,
}

/// Returns the complete academy and senior roster, with the same match-day
/// eligibility the squad builder uses. Views decide what to display from this flag.
pub fn query_squad(game: &Game, team_id: &str) -> Vec<SquadPlayer> {
    let eligible: HashSet<_> = match_day_eligible_players(&game.players, team_id)
        .into_iter()
        .map(|p| p.id.as_str())
        .collect();
    game.players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .map(|p| SquadPlayer {
            player: p.clone(),
            match_day_eligible: eligible.contains(p.id.as_str()),
            condition_status: condition_status(p.condition),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turn::squad::match_day_pool_tests::match_day_game;

    /// Given a thin senior squad, when the frontend requests its roster,
    /// then the called-up youth is marked eligible without changing his squad role.
    #[test]
    fn the_squad_projection_marks_only_called_up_youth_as_eligible() {
        let game = match_day_game(true, 17, 3);
        let value = serde_json::to_value(query_squad(&game, "club")).unwrap();
        let rows = value.as_array().unwrap();
        let row = |id| rows.iter().find(|p| p["id"] == id).unwrap();
        assert_eq!(row("youth-02")["match_day_eligible"], true);
        assert_eq!(row("youth-00")["match_day_eligible"], false);
        assert_eq!(row("youth-02")["squad_role"], "Youth");
        assert_eq!(rows.len(), 20);
    }
    /// Given a player on 74 and one on 75, either side of the fresh line,
    /// when the squad is projected for the screens,
    /// then each row carries its condition band with the word key and the tone to paint it in,
    /// so the squad list never applies a threshold of its own.
    #[test]
    fn the_squad_projection_carries_each_players_condition_band() {
        let mut game = match_day_game(true, 17, 3);
        game.players[0].condition = 74;
        game.players[1].condition = 75;
        let tiring_id = game.players[0].id.clone();
        let fresh_id = game.players[1].id.clone();

        let value = serde_json::to_value(query_squad(&game, "club")).unwrap();
        let rows = value.as_array().unwrap();
        let status =
            |id: &str| rows.iter().find(|p| p["id"] == id).unwrap()["condition_status"].clone();

        assert_eq!(
            status(&tiring_id),
            serde_json::json!({ "band": "Tiring", "label_key": "be.condition.tiring", "tone": "fair" })
        );
        assert_eq!(
            status(&fresh_id),
            serde_json::json!({ "band": "Fresh", "label_key": "be.condition.fresh", "tone": "good" })
        );
    }

    /// Given an injury crisis and injured academy players, when the squad view is
    /// projected, then treatment-room call-ups remain visible to their manager.
    #[test]
    fn emergency_injured_youth_are_visible_in_the_squad_projection() {
        let mut game = match_day_game(true, 10, 2);
        for p in game
            .players
            .iter_mut()
            .filter(|p| p.squad_role == domain::player::SquadRole::Youth)
        {
            p.injury = Some(domain::player::Injury {
                name: "knock".into(),
                days_remaining: 3,
            });
        }
        let squad = query_squad(&game, "club");
        assert!(
            squad
                .iter()
                .filter(|p| p.player.squad_role == domain::player::SquadRole::Youth)
                .all(|p| p.match_day_eligible)
        );
        let (side, _) = crate::turn::squad::build_team_with_bench(&game, "club");
        assert_eq!(side.players.len(), 11);
        for starter in side.players {
            assert!(
                squad
                    .iter()
                    .any(|p| p.player.id == starter.id && p.match_day_eligible)
            );
        }
    }
}
