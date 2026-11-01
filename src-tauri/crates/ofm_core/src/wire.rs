//! What leaves the backend of a player's potential: nothing.
//!
//! A player's potential is the game's hidden truth. The club may read an opinion of it, a range,
//! on his profile (`player_knowledge::potential_opinion`), and never the number, its own players
//! included (maintainer, 21 Oct). The true value stays where the game needs it: in the live game
//! and in the save.
//!
//! So `Game` serializes its players, and the youth prospects its messages carry, with potential 0
//! (`players_without_potential`, `messages_without_potential`). `Game`'s JSON is only ever a copy
//! for a reader (the screens, every command that returns it or a response that embeds it, and the
//! MCP game-state dumps), never the save: the database stores players and message contexts
//! through their own types (`db::repositories`), and a legacy JSON save is only ever read. Doing it
//! in the serializer means a new path to the screens cannot leak by forgetting to redact.
//!
//! The one reader who needs the truth is the harness that checks a save against the live game
//! (`truthful_value`).

use domain::message::InboxMessage;
use domain::player::Player;
use serde::ser::{SerializeSeq, Serializer};

use crate::game::Game;

/// `player` as a reader may receive him: everything but his potential, which reads 0.
pub fn without_potential(mut player: Player) -> Player {
    player.potential = 0;
    player
}

/// `Game.players` for a reader. Each player is copied one at a time, so the copy never holds more
/// than one player beside the game.
pub(crate) fn players_without_potential<S: Serializer>(
    players: &[Player],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(players.len()))?;
    for player in players {
        seq.serialize_element(&without_potential(player.clone()))?;
    }
    seq.end()
}

/// `Game.messages` for a reader: a youth scouting report's prospects without their potential. Every
/// other message goes as it is, uncopied.
pub(crate) fn messages_without_potential<S: Serializer>(
    messages: &[InboxMessage],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(messages.len()))?;
    for message in messages {
        match message.context.youth_prospects.as_ref() {
            Some(prospects) if prospects.iter().any(|prospect| prospect.potential != 0) => {
                let mut wire = message.clone();
                wire.context.youth_prospects =
                    Some(prospects.iter().cloned().map(without_potential).collect());
                seq.serialize_element(&wire)?;
            }
            _ => seq.serialize_element(message)?,
        }
    }
    seq.end()
}

/// The whole game as JSON, potentials included: what the save holds, for the developers' MCP dump
/// (`dev_full_game_state`, refused on a competition server) and the harness that compares a save
/// with the live game through it. Nothing a player reads.
///
/// It is the redacted JSON with each true potential written back, by position: the serializer keeps
/// the game's own order of players, messages and prospects.
pub fn truthful_value(game: &Game) -> serde_json::Result<serde_json::Value> {
    let mut value = serde_json::to_value(game)?;
    if let Some(players) = value["players"].as_array_mut() {
        for (json, player) in players.iter_mut().zip(&game.players) {
            json["potential"] = player.potential.into();
        }
    }
    if let Some(messages) = value["messages"].as_array_mut() {
        for (json, message) in messages.iter_mut().zip(&game.messages) {
            let Some(prospects) = message.context.youth_prospects.as_ref() else {
                continue;
            };
            if let Some(json_prospects) = json["context"]["youth_prospects"].as_array_mut() {
                for (json, prospect) in json_prospects.iter_mut().zip(prospects) {
                    json["potential"] = prospect.potential.into();
                }
            }
        }
    }
    Ok(value)
}

/// Every number a JSON value holds under a key named `potential`, with where it sits; a scout's
/// range (an object) is not one. The guard tests read a reader's copy through it.
#[cfg(test)]
pub(crate) fn potentials_in(value: &serde_json::Value) -> Vec<(String, u64)> {
    fn walk(value: &serde_json::Value, path: &str, found: &mut Vec<(String, u64)>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    let at = format!("{path}.{key}");
                    if key == "potential"
                        && let Some(number) = child.as_u64()
                    {
                        found.push((at.clone(), number));
                    }
                    walk(child, &at, found);
                }
            }
            serde_json::Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    walk(child, &format!("{path}[{index}]"), found);
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    walk(value, "", &mut found);
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::game::{PotentialRange, ScoutedPlayer};
    use crate::test_support::uniform_attributes;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::message::MessageContext;
    use domain::player::Position;

    fn player(id: &str, team_id: Option<&str>, potential: u8) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2008-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            uniform_attributes(60),
        );
        player.team_id = team_id.map(str::to_string);
        player.potential = potential;
        player
    }

    /// A world holding the manager's own youngster (potential 88), a rival (82), a scouted player
    /// with a report (79, the report's range 75-83), and a youth report naming a prospect (91).
    fn world() -> Game {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        manager.team_id = Some("mine".to_string());
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        let players = vec![
            player("own", Some("mine"), 88),
            player("rival", Some("rival"), 82),
            player("far", Some("faraway"), 79),
        ];
        let mut game = Game::new(clock, manager, vec![], players, vec![], vec![]);
        game.scouted_players.push(ScoutedPlayer {
            player_id: "far".to_string(),
            scouted_on: "2026-09-01".to_string(),
            attributes: uniform_attributes(60),
            potential: Some(PotentialRange { low: 75, high: 83 }),
            scout_name: Some("Ann Scout".to_string()),
        });
        game.messages.push(
            InboxMessage::new(
                "youth-scout-1".to_string(),
                String::new(),
                String::new(),
                "Ann Scout".to_string(),
                "2026-09-01".to_string(),
            )
            .with_context(MessageContext {
                youth_prospects: Some(vec![player("prospect", None, 91)]),
                ..MessageContext::default()
            }),
        );
        game
    }

    /// Given a world whose players, scouted player and youth prospect all have a true potential
    /// When the game is serialized for a reader
    /// Then no potential in it is a number but 0: not the club's own player's, a rival's, a
    ///      scouted player's or a prospect's (the scout's range is an object and stays)
    #[test]
    fn a_reader_never_receives_a_players_potential() {
        let value = serde_json::to_value(world()).unwrap();

        let leaked: Vec<_> = potentials_in(&value)
            .into_iter()
            .filter(|(_, potential)| *potential != 0)
            .collect();
        assert_eq!(leaked, vec![]);
        assert_eq!(potentials_in(&value).len(), 4, "every player was looked at");
        assert_eq!(value["scouted_players"][0]["potential"]["low"], 75);
    }

    /// Given the same world
    /// When it is serialized for a reader
    /// Then the live game still holds every true potential: only the copy is redacted
    #[test]
    fn redacting_the_copy_leaves_the_game_its_truth() {
        let game = world();

        serde_json::to_value(&game).unwrap();

        let potentials: Vec<u8> = game.players.iter().map(|player| player.potential).collect();
        assert_eq!(potentials, vec![88, 82, 79]);
        let prospect = &game.messages[0].context.youth_prospects.as_ref().unwrap()[0];
        assert_eq!(prospect.potential, 91);
    }

    /// Given the same world
    /// When it is written as the truthful form and as a reader's copy
    /// Then the two differ only in the potentials, which the truthful form gives as they are
    #[test]
    fn the_truthful_form_differs_from_a_readers_only_in_potential() {
        let game = world();
        let truthful = truthful_value(&game).unwrap();
        let mut redacted = serde_json::to_value(&game).unwrap();

        // A JSON object keeps its keys sorted, so the messages come before the players: compare
        // the potentials as a set.
        let mut truths: Vec<u64> = potentials_in(&truthful)
            .into_iter()
            .map(|(_, potential)| potential)
            .collect();
        truths.sort_unstable();
        assert_eq!(truths, vec![79, 82, 88, 91]);
        for (index, player) in game.players.iter().enumerate() {
            redacted["players"][index]["potential"] = player.potential.into();
        }
        redacted["messages"][0]["context"]["youth_prospects"][0]["potential"] = 91.into();
        assert_eq!(redacted, truthful);
    }
}
