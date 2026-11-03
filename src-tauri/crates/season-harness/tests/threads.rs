//! The game can be shared across threads and a match session sent to one: what a parallel
//! matchday (#849) stands on. A compile-time check, so the day somebody adds an `Rc` or a
//! `RefCell` to either, this stops compiling instead of a parallel run misbehaving.

use ofm_core::game::Game;
use ofm_core::live_match_manager::LiveMatchSession;

fn is_sync<T: Sync>() {}
fn is_send<T: Send>() {}

/// Given the game and a live match session,
/// When they are asked for the thread-safety markers a parallel matchday needs,
/// Then the game is `Sync` and the session is `Send`.
#[test]
fn the_game_is_sync_and_a_match_session_is_send() {
    is_sync::<Game>();
    is_send::<LiveMatchSession>();
}
