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
//! The few answers that carry players or messages outside a `Game` (the squad, an archived player,
//! the inbox and its archive) build their copies with `without_potential` and
//! `message_for_readers`. They are the paths to watch: a new one needs the same.
//!
//! The one reader who needs the truth is the harness that checks a save against the live game
//! (`truthful_value`).

use domain::message::InboxMessage;
use domain::player::{Player, PlayerTrait};
use serde::ser::{SerializeSeq, Serializer};

use crate::game::Game;

/// What a reader never receives of a player: his potential, and the Wonderkid tag the game reads off
/// it (`player_rating::qualifies_for_wonderkid`), which would tell the true potential in one bit.
/// The club's own verdict reaches the profile as `PotentialOpinion::wonderkid` instead.
fn redact(player: &mut Player) {
    player.potential = 0;
    player.traits.retain(|tag| *tag != PlayerTrait::Wonderkid);
    // Each season's record of it too (`CareerEntry::potential`).
    for season in &mut player.career {
        season.potential = None;
    }
}

/// `player` as a reader may receive him: everything but his potential, which reads 0, and the
/// Wonderkid tag read from it (`redact`).
pub fn without_potential(mut player: Player) -> Player {
    redact(&mut player);
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

/// `message` as a reader may receive it: a youth scouting report's prospects without their
/// potential or the Wonderkid tag read from it.
pub fn message_for_readers(message: &InboxMessage) -> InboxMessage {
    let mut wire = message.clone();
    if let Some(prospects) = wire.context.youth_prospects.as_mut() {
        for prospect in prospects {
            redact(prospect);
        }
    }
    wire
}

/// The inbox as a reader may receive it (`message_for_readers`): what every command that answers
/// with the messages themselves returns, rather than the game.
pub fn messages_for_readers(messages: &[InboxMessage]) -> Vec<InboxMessage> {
    messages.iter().map(message_for_readers).collect()
}

/// `Game.messages` for a reader: a youth scouting report's prospects without their potential. Every
/// other message goes as it is, uncopied.
pub(crate) fn messages_without_potential<S: Serializer>(
    messages: &[InboxMessage],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(messages.len()))?;
    for message in messages {
        if message.context.youth_prospects.is_some() {
            seq.serialize_element(&message_for_readers(message))?;
        } else {
            seq.serialize_element(message)?;
        }
    }
    seq.end()
}

/// The whole game as JSON, potentials and Wonderkid tags included: what the save holds, for the developers' MCP dump
/// (`dev_full_game_state`, refused on a competition server) and the harness that compares a save
/// with the live game through it. Nothing a player reads.
///
/// It is the redacted JSON with each true potential and trait list written back, by position: the
/// serializer keeps the game's own order of players, messages and prospects.
pub fn truthful_value(game: &Game) -> serde_json::Result<serde_json::Value> {
    let mut value = serde_json::to_value(game)?;
    if let Some(players) = value["players"].as_array_mut() {
        for (json, player) in players.iter_mut().zip(&game.players) {
            json["potential"] = player.potential.into();
            json["traits"] = serde_json::to_value(&player.traits)?;
            json["career"] = serde_json::to_value(&player.career)?;
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
                    json["traits"] = serde_json::to_value(&prospect.traits)?;
                    json["career"] = serde_json::to_value(&prospect.career)?;
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
        // The game's own tag, read from his true potential: no reader may receive it either.
        player.traits = vec![PlayerTrait::Wonderkid];
        // A recorded season, its potential the game's record: no reader receives that either.
        player.career = vec![domain::player::CareerEntry {
            season: 2025,
            ovr: Some(60),
            potential: Some(potential),
            ..Default::default()
        }];
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
        assert!(
            !value.to_string().contains("\"Wonderkid\""),
            "a reader's copy carries no Wonderkid tag"
        );
        let career_potentials: Vec<&serde_json::Value> = value["players"]
            .as_array()
            .unwrap()
            .iter()
            .map(|player| &player["career"][0]["potential"])
            .collect();
        assert!(
            career_potentials
                .iter()
                .all(|potential| potential.is_null()),
            "no season's record carries his potential: {career_potentials:?}"
        );
        assert_eq!(
            value["players"][0]["career"][0]["ovr"], 60,
            "his season's rating does"
        );
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

    /// Given the world's youth report, its prospect's true potential 91, in the inbox and in the
    ///       archive
    /// When the inbox and a page of the archive are read
    /// Then each gives the prospect with potential 0, and the game still holds 91
    #[test]
    fn the_inbox_and_its_archive_never_send_a_prospects_potential() {
        let mut game = world();
        game.archived_messages = game.messages.clone();
        let prospect_of =
            |message: &InboxMessage| message.context.youth_prospects.as_ref().unwrap()[0].potential;

        let inbox =
            crate::slices::inbox::query_messages(&game, &crate::slices::inbox::MessagesQuery {});
        let archive = crate::inbox_archive::page(&game, None, 10);

        assert_eq!(prospect_of(&inbox[0]), 0);
        assert_eq!(prospect_of(&archive.items[0]), 0);
        assert_eq!(prospect_of(&game.messages[0]), 91);
        assert_eq!(prospect_of(&game.archived_messages[0]), 91);
    }

    /// Given the same world
    /// When it is written as the truthful form and as a reader's copy
    /// Then the two differ only in the potentials (his own and each season's) and the Wonderkid
    ///      tags, which the truthful form gives as they are
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
        // Each player's potential and his recorded season's.
        assert_eq!(truths, vec![79, 79, 82, 82, 88, 88, 91, 91]);
        for (index, player) in game.players.iter().enumerate() {
            redacted["players"][index]["potential"] = player.potential.into();
        }
        redacted["messages"][0]["context"]["youth_prospects"][0]["potential"] = 91.into();
        let true_tags = serde_json::json!(["Wonderkid"]);
        for (index, player) in game.players.iter().enumerate() {
            redacted["players"][index]["traits"] = true_tags.clone();
            redacted["players"][index]["career"] = serde_json::to_value(&player.career).unwrap();
        }
        redacted["messages"][0]["context"]["youth_prospects"][0]["traits"] = true_tags;
        redacted["messages"][0]["context"]["youth_prospects"][0]["career"] = serde_json::to_value(
            &game.messages[0].context.youth_prospects.as_ref().unwrap()[0].career,
        )
        .unwrap();
        assert_eq!(redacted, truthful);
    }
}
