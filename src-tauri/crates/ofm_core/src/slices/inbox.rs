use crate::game::Game;
use domain::message::InboxMessage;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesQuery {}

pub fn query_messages(game: &Game, _query: &MessagesQuery) -> Vec<InboxMessage> {
    game.messages.clone()
}
