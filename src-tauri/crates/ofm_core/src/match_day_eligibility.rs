//! The club player pool every match-day consumer reads.
use crate::player_rating::natural_ovr;
use domain::player::{Player, Position, SquadRole};

// A healthy eleven plus seven reserves; healthy seniors are never capped.
const MATCH_DAY_POOL_TARGET: usize = 18;

/// Healthy seniors, with youth covering a missing keeper before the numeric
/// shortage. In a crisis the treatment room remains available to complete an XI.
/// Call-ups never change the player's squad role.
pub(crate) fn match_day_eligible_players<'a>(
    players: &'a [Player],
    team_id: &str,
) -> Vec<&'a Player> {
    let (mut eligible, mut youth): (Vec<_>, Vec<_>) = players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.injury.is_none())
        .partition(|p| p.squad_role == SquadRole::Senior);
    youth.sort_by(|left, right| {
        natural_ovr(right)
            .total_cmp(&natural_ovr(left))
            .then_with(|| left.id.cmp(&right.id))
    });
    if !eligible
        .iter()
        .any(|p| p.natural_position == Position::Goalkeeper)
        && let Some(index) = youth
            .iter()
            .position(|p| p.natural_position == Position::Goalkeeper)
    {
        eligible.push(youth.remove(index));
    }
    let youth_needed = MATCH_DAY_POOL_TARGET.saturating_sub(eligible.len());
    eligible.extend(youth.into_iter().take(youth_needed));
    // The builder can field injured youth only when a healthy eleven is impossible.
    // Projecting them too ensures that no emergency call-up is invisible in training.
    if eligible.len() < 11 {
        eligible.extend(
            players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some(team_id) && p.injury.is_some()),
        );
    }
    eligible
}
