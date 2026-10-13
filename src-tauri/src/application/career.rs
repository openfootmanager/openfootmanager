use domain::stats::StatsState;
use ofm_core::{game::Game, state::StateManager};

/// Install a created or loaded career together with its stats and save identity.
///
/// Ordinary same-career writers also use `set_game`, so reset the transient
/// session only here. Sharing the lifecycle lock prevents a concurrent start
/// or finish from publishing the previous career's session or result afterward.
pub(crate) fn install_career(
    state: &StateManager,
    game: Game,
    stats: StatsState,
    save_id: Option<String>,
) {
    let _operation = super::live_session::operation();
    drop(state.take_live_match());
    state.set_game(game);
    state.set_stats_state(stats);
    match save_id {
        Some(id) => state.set_save_id(id),
        None => state.clear_save_id(),
    }
}

/// The active game together with the save it belongs to, read under the career lock
/// `install_career` holds. Both come from one career: a load or a new game installed meanwhile
/// cannot hand back one career's game with another's save id.
pub(crate) fn snapshot_career(state: &StateManager) -> Option<(Game, Option<String>)> {
    let _operation = super::live_session::operation();
    let game = state.get_game(|game| game.clone())?;
    Some((game, state.get_save_id()))
}
