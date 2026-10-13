//! Injuries a club match really inflicts.
//!
//! The engine records an injury event when a player is hurt on the pitch. This is where that event
//! stops being a line in the match report and becomes an injury on the player, with a name and a
//! time out, and where the user's physio tells them about it.

use crate::game::Game;
use crate::messages;
use crate::player_wear::roll_injury;

/// Turns the engine's injury events into player injuries.
///
/// One injury per player per match, never on an already injured player, and never one the engine
/// did not produce. The rolls are seeded from the day, the fixture and the player, so the same
/// season always injures the same players for the same time.
pub(super) fn apply_match_injuries(
    game: &mut Game,
    fixture_id: &str,
    report: &engine::MatchReport,
) {
    let user_team_id = game.manager.team_id.clone();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let mut physio_reports = Vec::new();

    for event in report
        .events
        .iter()
        .filter(|event| event.event_type == engine::EventType::Injury)
    {
        let Some(player_id) = event.player_id.as_deref() else {
            continue;
        };

        let mut rng = game.rng_today(&format!("post-match/injury/{fixture_id}/{player_id}"));
        let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
        else {
            continue;
        };
        if player.injury.is_some() {
            continue;
        }

        let injury = roll_injury(&mut rng);
        if user_team_id.is_some() && player.team_id == user_team_id {
            physio_reports.push((player.id.clone(), player.match_name.clone(), injury.clone()));
        }
        player.injury = Some(injury);
    }

    for (player_id, player_name, injury) in physio_reports {
        let message_id = format!("match_injury_{fixture_id}_{player_id}");
        crate::inbox::emit_once(game, &message_id.clone(), || {
            messages::match_injury_message(
                &message_id,
                &player_id,
                &player_name,
                &injury.name,
                injury.days_remaining,
                &today,
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{clock::GameClock, test_support::uniform_attributes};
    use chrono::{TimeZone, Utc};
    use domain::{
        manager::Manager,
        player::{Injury, Player, Position},
        team::Team,
    };
    use engine::{EventType, MatchEvent, MatchReport, Side, TeamStats, Zone};
    use std::collections::HashMap;

    fn game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager".into(),
            "Test".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        manager.hire("home".into());
        let mut teams = Vec::new();
        let mut players = Vec::new();
        for id in ["home", "away"] {
            teams.push(Team::new(
                id.into(),
                format!("{id} FC"),
                id.into(),
                "ENG".into(),
                "London".into(),
                "Ground".into(),
                20_000,
            ));
            for index in 0..3 {
                let mut player = Player::new(
                    format!("{id}-{index}"),
                    "Player".into(),
                    format!("{id} Player {index}"),
                    "1995-01-01".into(),
                    "ENG".into(),
                    Position::Midfielder,
                    uniform_attributes(65),
                );
                player.team_id = Some(id.into());
                players.push(player);
            }
        }
        let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
        game.seed = 77;
        game
    }

    fn report_with_injuries(player_ids: &[&str]) -> MatchReport {
        MatchReport {
            home_goals: 0,
            away_goals: 0,
            home_stats: TeamStats::default(),
            away_stats: TeamStats::default(),
            events: player_ids
                .iter()
                .map(|id| {
                    MatchEvent::new(30, EventType::Injury, Side::Home, Zone::Midfield)
                        .with_player(id)
                })
                .collect(),
            goals: vec![],
            player_stats: HashMap::new(),
            home_possession: 50.0,
            total_minutes: 90,
            home_penalties: None,
            away_penalties: None,
        }
    }

    fn injury_of<'a>(game: &'a Game, id: &str) -> Option<&'a Injury> {
        game.players
            .iter()
            .find(|player| player.id == id)
            .and_then(|player| player.injury.as_ref())
    }

    fn physio_messages(game: &Game) -> Vec<&domain::message::InboxMessage> {
        game.messages
            .iter()
            .filter(|message| message.id.starts_with("match_injury_"))
            .collect()
    }

    /// Given a club match in which the engine records an injury to a player
    /// When the match result is applied
    /// Then that player is injured, with a translated name and an expected time out.
    #[test]
    fn an_injury_in_a_club_match_sticks() {
        let mut game = game();

        apply_match_injuries(&mut game, "fixture-1", &report_with_injuries(&["home-1"]));

        let injury = injury_of(&game, "home-1").expect("injured");
        assert!(injury.name.starts_with("common.injuries."));
        assert!(injury.days_remaining >= 1);
    }

    /// Given a club match with no injury event
    /// When the result is applied
    /// Then no player is injured by the match.
    #[test]
    fn nothing_is_invented_when_the_engine_recorded_no_injury() {
        let mut game = game();

        apply_match_injuries(&mut game, "fixture-1", &report_with_injuries(&[]));

        assert!(game.players.iter().all(|player| player.injury.is_none()));
    }

    /// Given a player who is already injured
    /// When an injury event names them again
    /// Then they keep exactly the injury they had.
    #[test]
    fn an_already_injured_player_is_not_injured_twice() {
        let mut game = game();
        let existing = Injury {
            name: "common.injuries.calfStrain".into(),
            days_remaining: 40,
        };
        game.players[1].injury = Some(existing.clone());

        apply_match_injuries(&mut game, "fixture-1", &report_with_injuries(&["home-1"]));

        let injury = injury_of(&game, "home-1").expect("still injured");
        assert_eq!(injury.name, existing.name);
        assert_eq!(injury.days_remaining, 40);
        assert!(physio_messages(&game).is_empty());
    }

    /// Given a player named by two injury events in one match
    /// When the result is applied
    /// Then they carry one injury and the physio reports it once.
    #[test]
    fn two_events_for_one_player_make_one_injury_and_one_message() {
        let mut game = game();

        apply_match_injuries(
            &mut game,
            "fixture-1",
            &report_with_injuries(&["home-1", "home-1"]),
        );

        assert!(injury_of(&game, "home-1").is_some());
        assert_eq!(physio_messages(&game).len(), 1);
    }

    /// Given a player of the user's club injured in a match
    /// When the result is applied, the message is deleted, the game is saved and reloaded, and the
    ///      injury is cleared and the same match is applied again
    /// Then exactly one physio message was ever sent, naming the injury and the time out.
    #[test]
    fn the_physio_tells_the_manager_once_even_after_delete_and_reload() {
        let mut game = game();
        apply_match_injuries(&mut game, "fixture-1", &report_with_injuries(&["home-1"]));
        let injury = injury_of(&game, "home-1").expect("injured").clone();
        let sent = physio_messages(&game);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].id, "match_injury_fixture-1_home-1");
        assert_eq!(
            sent[0].subject_key.as_deref(),
            Some("be.msg.matchInjury.subject")
        );
        assert_eq!(sent[0].i18n_params.get("injury"), Some(&injury.name));
        assert_eq!(
            sent[0].i18n_params.get("days"),
            Some(&injury.days_remaining.to_string())
        );

        game.messages.clear();
        let mut reloaded: Game =
            serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        reloaded.players[1].injury = None;
        apply_match_injuries(
            &mut reloaded,
            "fixture-1",
            &report_with_injuries(&["home-1"]),
        );

        assert!(physio_messages(&reloaded).is_empty());
    }

    /// Given a player of another club injured in a match
    /// When the result is applied
    /// Then they are injured and the user receives no message.
    #[test]
    fn other_clubs_injuries_are_silent() {
        let mut game = game();

        apply_match_injuries(&mut game, "fixture-1", &report_with_injuries(&["away-1"]));

        assert!(injury_of(&game, "away-1").is_some());
        assert!(physio_messages(&game).is_empty());
    }

    /// Given the same seed
    /// When the same match is applied to two identical games
    /// Then the same players are injured for the same durations.
    #[test]
    fn the_same_seed_gives_the_same_injuries() {
        let mut first = game();
        let mut second = game();
        let report = report_with_injuries(&["home-0", "away-2"]);

        apply_match_injuries(&mut first, "fixture-1", &report);
        apply_match_injuries(&mut second, "fixture-1", &report);

        for id in ["home-0", "away-2"] {
            let a = injury_of(&first, id).expect("injured");
            let b = injury_of(&second, id).expect("injured");
            assert_eq!((&a.name, a.days_remaining), (&b.name, b.days_remaining));
        }
    }
}
