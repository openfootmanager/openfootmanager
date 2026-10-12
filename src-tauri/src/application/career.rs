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
