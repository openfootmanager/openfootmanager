//! What leaves the backend of a player's potential: nothing.
//!
//! A player's potential is the game's hidden truth. The club may read an opinion of it, a range,
//! on his profile (`player_knowledge::potential_opinion`), and never the number, its own players
//! included (maintainer, 21 Oct). The true value stays where the game needs it: in the live game
//! and in the save.
//!
//! So `Game` serializes its players, and the youth prospects its messages carry, with potential 0
//! (`players_for_reader`, `messages_for_reader`). `Game`'s JSON is only ever a copy
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
//!
//! **Who the copy is for.** `Game`'s serializer (`serialize_game`) writes for the reader already
//! set, if one is, and otherwise for the manager's club. The setting lives in a thread-local for
//! the length of one synchronous `serialize` call, behind a guard that restores the reader before
//! it, so a nested or unwinding call leaves nothing behind. The club's copy writes each player as
//! a [`ReaderPlayer`]: his rating and attributes only when the club knows him. Game's players
//! written with no reader set (the derived serializer called directly) are written for no club at
//! all, so everyone is unknown: never as the truth, and never with a rival's rating.
//! A `Player` serialized outside `Game` is written as he is: those paths still build their copy
//! with `without_potential`, as above.
use std::cell::RefCell;
use std::rc::Rc;

use domain::contract_ledger::MovementLedger;
use domain::message::InboxMessage;
use domain::player::{
    ActiveLoan, CareerEntry, Footedness, Injury, LoanOffer, Player, PlayerAttributes, PlayerMedia,
    PlayerMoraleCore, PlayerSeasonStats, PlayerTrait, Position, SquadRole, TransferOffer,
};
use domain::team::TrainingFocus;
use serde::Serialize;
use serde::ser::{SerializeSeq, Serializer};
use ts_rs::TS;

use crate::game::Game;
use crate::player_knowledge::{KnowledgeView, PlayerKnowledge, knows_rating};

/// Who a game's JSON is being written for.
#[derive(Clone)]
enum Reader {
    /// The manager's club, with what it knows of each player: what every screen and MCP tool
    /// receives.
    Club(Rc<KnowledgeView>),
    /// The save harness and the developers' dump: every player as the game holds him.
    Truthful,
}

thread_local! {
    static READER: RefCell<Option<Reader>> = const { RefCell::new(None) };
}

/// The reader set for as long as the scope lives; dropping it restores the one before.
///
/// Not `Send`: a scope held across an `.await` could be dropped on another worker thread, restoring
/// that thread's reader and leaving this one's set. As it is, such a future fails to compile.
struct ReaderScope {
    previous: Option<Reader>,
    _not_send: std::marker::PhantomData<*const ()>,
}

impl ReaderScope {
    fn enter(reader: Reader) -> Self {
        Self {
            previous: READER.with(|current| current.replace(Some(reader))),
            _not_send: std::marker::PhantomData,
        }
    }
}

impl Drop for ReaderScope {
    fn drop(&mut self) {
        let previous = self.previous.take();
        READER.with(|current| *current.borrow_mut() = previous);
    }
}

/// The reader set now, cloned out so no borrow is held while anything is written: writing
/// re-enters (a game inside a response, a scope inside a scope).
fn current_reader() -> Option<Reader> {
    READER.with(|current| current.borrow().clone())
}

fn writes_the_truth() -> bool {
    matches!(current_reader(), Some(Reader::Truthful))
}

/// `Game`'s serializer (`impl Serialize for Game`): the game written for the reader already set,
/// or else for the manager's club, through the derived serializer.
pub(crate) fn serialize_game<S: Serializer>(game: &Game, serializer: S) -> Result<S::Ok, S::Error> {
    let _scope = current_reader()
        .is_none()
        .then(|| ReaderScope::enter(Reader::Club(Rc::new(KnowledgeView::of(game)))));
    // The derived serializer, an inherent function (`remote = "Self"` on `Game`); the trait method
    // would come back here.
    Game::serialize(game, serializer)
}

/// `player` as a reader may receive him: everything but his potential, which reads 0.
pub fn without_potential(mut player: Player) -> Player {
    player.potential = 0;
    player
}

/// `Game.players` for a reader. Each player is copied one at a time, so the copy never holds more
/// than one player beside the game. In the truthful scope, the players as they are.
pub(crate) fn players_for_reader<S: Serializer>(
    players: &[Player],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let view = match current_reader() {
        Some(Reader::Truthful) => return players.serialize(serializer),
        Some(Reader::Club(view)) => view,
        // Nobody said who is reading: the view of no club, so everyone is unknown.
        None => Rc::new(KnowledgeView::of_nobody()),
    };
    let mut seq = serializer.serialize_seq(Some(players.len()))?;
    for player in players {
        seq.serialize_element(&ReaderPlayer::of(player, &view.knowledge(player)))?;
    }
    seq.end()
}

/// A player as a reader receives him in the game's JSON: every field of [`Player`], his rating and
/// attributes only when the club knows them (`player_knowledge::knows_rating`: its own player, or
/// one in its division), and his potential as 0 ([`without_potential`]).
#[derive(Debug, Clone, Serialize, TS)]
pub struct ReaderPlayer {
    pub id: String,
    pub match_name: String,
    pub full_name: String,
    pub date_of_birth: String,
    pub nationality: String,
    pub football_nation: String,
    pub birth_country: Option<String>,
    pub media: PlayerMedia,
    pub position: Position,
    pub natural_position: Position,
    pub alternate_positions: Vec<Position>,
    pub footedness: Footedness,
    pub weak_foot: u8,
    /// None when the club does not know him.
    pub attributes: Option<PlayerAttributes>,
    pub condition: u8,
    pub morale: u8,
    pub fitness: u8,
    pub injury: Option<Injury>,
    pub team_id: Option<String>,
    pub retired: bool,
    pub squad_role: SquadRole,
    pub traits: Vec<PlayerTrait>,
    /// None when the club does not know him.
    pub ovr: Option<u8>,
    pub potential: u8,
    pub market_value: u64,
    pub stats: PlayerSeasonStats,
    pub career: Vec<CareerEntry>,
    #[serde(flatten)]
    pub movement_history: MovementLedger,
    pub training_focus: Option<TrainingFocus>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub transfer_offers: Vec<TransferOffer>,
    pub loan_offers: Vec<LoanOffer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_loan: Option<ActiveLoan>,
    pub morale_core: PlayerMoraleCore,
    pub jersey_number: Option<u8>,
}

impl ReaderPlayer {
    /// `player` as the club that knows him as `knowledge` may receive him.
    pub fn of(player: &Player, knowledge: &PlayerKnowledge) -> Self {
        let knows = knows_rating(knowledge);
        // Every field by name, so a field added to `Player` does not compile here until the wire
        // decides what a reader receives of it.
        let Player {
            id,
            match_name,
            full_name,
            date_of_birth,
            nationality,
            football_nation,
            birth_country,
            media,
            position,
            natural_position,
            alternate_positions,
            footedness,
            weak_foot,
            attributes,
            condition,
            morale,
            fitness,
            injury,
            team_id,
            retired,
            squad_role,
            traits,
            ovr,
            potential,
            market_value,
            stats,
            career,
            movement_history,
            training_focus,
            transfer_listed,
            loan_listed,
            transfer_offers,
            loan_offers,
            active_loan,
            morale_core,
            jersey_number,
        } = without_potential(player.clone());
        Self {
            id,
            match_name,
            full_name,
            date_of_birth,
            nationality,
            football_nation,
            birth_country,
            media,
            position,
            natural_position,
            alternate_positions,
            footedness,
            weak_foot,
            attributes: knows.then_some(attributes),
            condition,
            morale,
            fitness,
            injury,
            team_id,
            retired,
            squad_role,
            traits,
            ovr: knows.then_some(ovr),
            potential,
            market_value,
            stats,
            career,
            movement_history,
            training_focus,
            transfer_listed,
            loan_listed,
            transfer_offers,
            loan_offers,
            active_loan,
            morale_core,
            jersey_number,
        }
    }
}

/// `message` as a reader may receive it: a youth scouting report's prospects without their
/// potential.
pub fn message_for_readers(message: &InboxMessage) -> InboxMessage {
    let mut wire = message.clone();
    if let Some(prospects) = wire.context.youth_prospects.as_mut() {
        for prospect in prospects {
            prospect.potential = 0;
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
/// other message goes as it is, uncopied. In the truthful scope, every message as it is.
pub(crate) fn messages_for_reader<S: Serializer>(
    messages: &[InboxMessage],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if writes_the_truth() {
        return messages.serialize(serializer);
    }
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

/// The whole game as JSON, potentials included: what the save holds, for the developers' MCP dump
/// (`dev_full_game_state`, refused on a competition server) and the harness that compares a save
/// with the live game through it. Nothing a player reads. It reads back as the game
/// (`serde_json::from_value`), but for the fields `Game` never serializes (`#[serde(skip)]`, such
/// as the archives and the cash journal); a reader's copy does not read back at all.
pub fn truthful_value(game: &Game) -> serde_json::Result<serde_json::Value> {
    let _scope = ReaderScope::enter(Reader::Truthful);
    serde_json::to_value(game)
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
    /// When it is written as the truthful form and read back
    /// Then it is the game again, every true potential included: the form the save harness and the
    ///      developers' dump rely on
    #[test]
    fn the_truthful_form_reads_back_as_the_game() {
        let game = world();

        let back: Game = serde_json::from_value(truthful_value(&game).unwrap()).unwrap();

        let potentials: Vec<u8> = back.players.iter().map(|player| player.potential).collect();
        assert_eq!(potentials, vec![88, 82, 79]);
        let prospect = &back.messages[0].context.youth_prospects.as_ref().unwrap()[0];
        assert_eq!(prospect.potential, 91);
    }

    /// Given Game's players written through their field serializer with no reader set
    /// When they are serialized
    /// Then they are written as for the club, never as the truth
    #[test]
    fn game_players_written_with_no_reader_are_never_the_truth() {
        #[derive(Serialize)]
        struct Loose {
            #[serde(serialize_with = "players_for_reader")]
            players: Vec<Player>,
        }

        let value = serde_json::to_value(Loose {
            players: world().players,
        })
        .unwrap();

        let potentials: Vec<u64> = potentials_in(&value).into_iter().map(|(_, p)| p).collect();
        assert_eq!(potentials, vec![0, 0, 0]);
        let ratings: Vec<&serde_json::Value> = value["players"]
            .as_array()
            .unwrap()
            .iter()
            .map(|player| &player["ovr"])
            .collect();
        assert!(ratings.iter().all(|ovr| ovr.is_null()), "{ratings:?}");
    }

    /// Given the game, and the derived serializer called directly (the inherent function
    ///       `remote = "Self"` makes public), with no reader set
    /// When the game is serialized that way
    /// Then it is a reader's copy, not the truth: going round `serialize_game` gains nothing
    #[test]
    fn the_derived_serializer_called_directly_is_never_the_truth() {
        let game = world();

        let value = Game::serialize(&game, serde_json::value::Serializer).unwrap();

        assert_eq!(value["players"][0]["potential"], 0);
        assert_eq!(
            value["players"][0]["ovr"],
            serde_json::Value::Null,
            "written for no club: even the manager's own player's rating is hidden, never a rival's shown"
        );
    }

    /// Given the manager's own player, a rival outside his division, and a player his scout
    ///       reported on
    /// When the game is serialized for the club
    /// Then the own player's rating and attributes are there, and the other two's are not: the
    ///      scouted player's are in the scout's report, the rival's nowhere
    #[test]
    fn the_club_sees_the_ratings_of_the_players_it_knows() {
        let value = serde_json::to_value(world()).unwrap();
        let player = |id: &str| {
            value["players"]
                .as_array()
                .unwrap()
                .iter()
                .find(|player| player["id"] == id)
                .unwrap()
                .clone()
        };

        assert_eq!(player("own")["ovr"], world().players[0].ovr);
        assert!(player("own")["attributes"].is_object());
        for hidden in ["rival", "far"] {
            assert!(player(hidden)["ovr"].is_null(), "{hidden}");
            assert!(player(hidden)["attributes"].is_null(), "{hidden}");
        }
        assert_eq!(value["scouted_players"][0]["player_id"], "far");
    }

    /// Given a player the club knows as a division rival, and the same player unknown
    /// When each is made a reader's player
    /// Then the known one carries his rating and attributes, the unknown one neither
    #[test]
    fn a_division_rival_is_seen_and_a_stranger_is_not() {
        let rival = player("rival", Some("rival"), 82);

        let known = ReaderPlayer::of(&rival, &PlayerKnowledge::Known);
        let stranger = ReaderPlayer::of(&rival, &PlayerKnowledge::Unknown);

        assert_eq!(known.ovr, Some(rival.ovr));
        assert!(known.attributes.is_some());
        assert_eq!((stranger.ovr, stranger.attributes), (None, None));
    }

    /// Given the club's own player
    /// When he is written as a reader's player and as the game holds him
    /// Then the two are the same JSON but for his potential: the wire type carries every field
    ///      `Player` writes, so none is lost or added by mirroring it
    #[test]
    fn a_reader_player_writes_every_field_a_player_does() {
        let own = player("own", Some("mine"), 88);

        let reader = serde_json::to_value(ReaderPlayer::of(&own, &PlayerKnowledge::Own)).unwrap();
        let truth = serde_json::to_value(without_potential(own)).unwrap();

        assert_eq!(reader, truth);
    }

    /// Given the truthful scope, and a reader's copy written inside it and after it
    /// When the game is serialized
    /// Then inside it the game is written truthfully (the outer reader wins), and after it the
    ///      scope is gone: the next copy is a reader's again
    #[test]
    fn a_reader_scope_restores_the_reader_before_it() {
        let game = world();

        let inside = {
            let _scope = ReaderScope::enter(Reader::Truthful);
            serde_json::to_value(&game).unwrap()
        };
        let after = serde_json::to_value(&game).unwrap();

        assert_eq!(inside["players"][0]["potential"], 88);
        assert_eq!(after["players"][0]["potential"], 0);
        assert!(
            READER.with(|current| current.borrow().is_none()),
            "nothing is left set"
        );
    }

    /// Given the same world
    /// When it is written as the truthful form and as a reader's copy
    /// Then the two differ only in the potentials, and in the ratings and attributes the club does
    ///      not know, which the truthful form gives as they are
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
            // What the club cannot know of a rival or a scouted player, which the truth holds.
            redacted["players"][index]["ovr"] = player.ovr.into();
            redacted["players"][index]["attributes"] =
                serde_json::to_value(&player.attributes).unwrap();
        }
        redacted["messages"][0]["context"]["youth_prospects"][0]["potential"] = 91.into();
        assert_eq!(redacted, truthful);
    }
}
