//! Promotion and relegation between a domestic competition and its sibling feeders.
//!
//! Distinct from [`super::pyramid`]: these movements are driven by declared berths between
//! competitions in the same country — regional groups merging into a central division — rather
//! than by adjacent rungs of one ladder.

use super::continental::{BerthFieldOptions, evaluate_berth_rule, resolve_berth_fields};
use crate::game::Game;
use domain::league::{
    Berth, BerthRule, CompetitionFormat, CompetitionScope, CompetitionType, League,
};

/// Domestic league-table fields populated from sibling regional berths
/// (e.g. several state leagues feeding one national "Central League").
/// Same cross-target exclusivity as the continental path, but `fallbackTo` is
/// not followed: a club that misses its primary target has nowhere to drop to
/// domestically, so it stays in its own league. Targets without incoming
/// berths are absent.
pub(crate) fn resolve_domestic_berth_fields(
    game: &Game,
) -> std::collections::HashMap<String, Vec<String>> {
    resolve_berth_fields(
        game,
        BerthFieldOptions {
            scope_match: |competition: &League| {
                competition.scope == CompetitionScope::Domestic
                    && competition.kind == CompetitionType::League
            },
            berth_eligible: |source: &League, berth: &Berth| {
                source.rules.format == CompetitionFormat::LeagueTable
                    && matches!(berth.rule, BerthRule::PositionRange { .. })
            },
            fill: false,
            follow_fallback: false,
        },
    )
}

/// Clubs a `LeagueTable` source sends up via a primary `PositionRange` berth
/// at `target_id`. `None` when the source is not a domestic feeder for this
/// target (wrong format, no such berth, or the range yields nobody) so it
/// must not receive relegated dropouts.
fn domestic_position_range_promoted(
    source: &League,
    target_id: &str,
) -> Option<std::collections::HashSet<String>> {
    if source.rules.format != CompetitionFormat::LeagueTable {
        return None;
    }
    let promoted: std::collections::HashSet<String> = source
        .berths
        .iter()
        .filter(|berth| {
            berth.target == target_id && matches!(berth.rule, BerthRule::PositionRange { .. })
        })
        .flat_map(|berth| evaluate_berth_rule(source, &berth.rule))
        .collect();
    if promoted.is_empty() {
        None
    } else {
        Some(promoted)
    }
}

/// Merge berth-fed domestic league tables after the linear pyramid pass.
///
/// For each domestic `LeagueTable` that received a non-empty resolved field:
/// incoming berth winners replace the bottom K finishers so the league keeps
/// its authored size (survivors stay in their existing `participant_ids`
/// order; the entrants are appended). Those K dropouts are then given to
/// each feeder in turn — `promoted.len()` clubs first, so the feeder keeps
/// its authored size — after each feeder drops the clubs it sent up. Those
/// quotas cover every dropout, so no remainder is left over; a round-robin
/// backstop shares one out if that ever stops holding.
///
/// Both sides are read from the roster as it stands *now*, not from the frozen
/// table: a place-getter only promotes while it is still registered with the
/// feeder that awarded it, and only a club still in the target can be
/// relegated out of it. The linear pass runs first and rewrites those rosters,
/// so trusting the table alone would let a club the ladder already relegated
/// be promoted as well and finish the rollover on two tables.
///
/// If the target or any contributing `LeagueTable` feeder is still mid-season
/// (`is_league_season_ended` is false), the whole merge is skipped so a
/// hemisphere-foreign rollover cannot promote from an unfinished table or
/// duplicate a feeder's place-getters. Oversubscribed berths keep the
/// evaluation-order prefix (source list, then berth, then standings) and leave
/// surplus place-getters in their feeder so they are not removed from both
/// tables. The final roster is capped to the target's authored length as a
/// backstop.
///
/// Leagues without incoming berths are left untouched. Must run after the
/// linear pyramid pass (which handles feeder-vs-lower-tier edges) and before
/// fixture regeneration.
pub(crate) fn apply_domestic_berth_promotion_relegation(
    game: &mut Game,
    fields: &std::collections::HashMap<String, Vec<String>>,
) {
    use std::collections::HashSet;

    let target_ids: Vec<String> = game
        .competitions
        .iter()
        .filter(|competition| {
            competition.scope == CompetitionScope::Domestic
                && competition.kind == CompetitionType::League
                && competition.rules.format == CompetitionFormat::LeagueTable
                && fields
                    .get(&competition.id)
                    .is_some_and(|field| !field.is_empty())
        })
        .map(|competition| competition.id.clone())
        .collect();

    for target_id in target_ids {
        let Some(target_index) = game.competitions.iter().position(|c| c.id == target_id) else {
            continue;
        };
        if !crate::end_of_season::is_league_season_ended(&game.competitions[target_index]) {
            continue;
        }
        let unfinished_feeder = game.competitions.iter().enumerate().any(|(index, source)| {
            index != target_index
                && source.rules.format == CompetitionFormat::LeagueTable
                && source.berths.iter().any(|berth| {
                    berth.target == target_id
                        && matches!(berth.rule, BerthRule::PositionRange { .. })
                })
                && !crate::end_of_season::is_league_season_ended(source)
        });
        if unfinished_feeder {
            continue;
        }

        // Standings are a frozen record of a finished season, but the linear
        // pyramid pass has already rewritten rosters. A finishing place only
        // earns promotion while the club is still registered with the feeder
        // that awarded it — otherwise a club the ladder relegated would also
        // be promoted here and end the rollover on two tables.
        let mut feeder_plans: Vec<(usize, HashSet<String>)> = game
            .competitions
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != target_index)
            .filter_map(|(index, source)| {
                let registered: HashSet<&str> =
                    source.participant_ids.iter().map(String::as_str).collect();
                let mut promoted = domestic_position_range_promoted(source, &target_id)?;
                promoted.retain(|club| registered.contains(club.as_str()));
                (!promoted.is_empty()).then_some((index, promoted))
            })
            .collect();

        // Rank the feeders before anything is allocated. Until now this ran in
        // whatever order the competitions happened to sit in, and persistence
        // does not preserve that order — competitions reload sorted by
        // priority, season and name — so saving and reloading changed which
        // feeder supplied a promoted club when more of them qualified than the
        // target had places. Rank, then id: the higher division's place-getters
        // go up first, and ties break on a name rather than on a vector index.
        feeder_plans.sort_by(|left, right| {
            let key = |index: usize| {
                (
                    game.competitions[index].priority,
                    game.competitions[index].id.clone(),
                )
            };
            key(left.0).cmp(&key(right.0))
        });

        let mut entrants: Vec<String> = {
            let promotable: HashSet<&str> = feeder_plans
                .iter()
                .flat_map(|(_, promoted)| promoted.iter().map(String::as_str))
                .collect();
            let already_in_target: HashSet<&str> = game.competitions[target_index]
                .participant_ids
                .iter()
                .map(String::as_str)
                .collect();
            // Order the field by its feeder's rank too, so truncating it to the
            // available places cuts the same clubs before and after a reload.
            // Within one feeder the order is the finishing order, which is
            // merit and must not be disturbed.
            let feeder_rank: std::collections::HashMap<&str, usize> = feeder_plans
                .iter()
                .enumerate()
                .flat_map(|(rank, (_, promoted))| {
                    promoted.iter().map(move |club| (club.as_str(), rank))
                })
                .collect();
            let mut field: Vec<String> = fields
                .get(&target_id)
                .into_iter()
                .flatten()
                .filter(|club| {
                    promotable.contains(club.as_str()) && !already_in_target.contains(club.as_str())
                })
                .cloned()
                .collect();
            field.sort_by_key(|club| {
                feeder_rank
                    .get(club.as_str())
                    .copied()
                    .unwrap_or(usize::MAX)
            });
            field
        };
        if entrants.is_empty() {
            continue;
        }
        let incoming = entrants.len();

        // Relegate from the roster as it stands now, worst finisher first. A
        // club the ladder already moved out has no place left to release.
        let authored = game.competitions[target_index].participant_ids.len();
        let dropouts: Vec<String> = {
            let target = &game.competitions[target_index];
            let registered: HashSet<&str> =
                target.participant_ids.iter().map(String::as_str).collect();
            let mut eligible: Vec<String> = target
                .sorted_standings()
                .into_iter()
                .map(|entry| entry.team_id)
                .filter(|club| registered.contains(club.as_str()))
                .collect();
            let drop_count = incoming.min(eligible.len());
            eligible.split_off(eligible.len() - drop_count)
        };
        // Berths can award more clubs than the target releases. Only the
        // clubs that take a released place leave their feeder; surplus
        // stay put instead of vanishing from both tables.
        entrants.truncate(dropouts.len());
        let placed: HashSet<String> = entrants.iter().cloned().collect();

        let mut next_participants: Vec<String> = {
            let dropout_set: HashSet<&str> = dropouts.iter().map(String::as_str).collect();
            game.competitions[target_index]
                .participant_ids
                .iter()
                .filter(|club| !dropout_set.contains(club.as_str()))
                .cloned()
                .collect()
        };
        next_participants.extend(entrants);
        next_participants.truncate(authored);
        game.competitions[target_index].participant_ids = next_participants;

        feeder_plans.retain_mut(|(_, promoted)| {
            promoted.retain(|club| placed.contains(club));
            !promoted.is_empty()
        });
        if feeder_plans.is_empty() {
            continue;
        }
        let mut received: Vec<Vec<String>> = vec![Vec::new(); feeder_plans.len()];
        let mut leftover = dropouts;
        for (slot, (_, promoted)) in feeder_plans.iter().enumerate() {
            let take = promoted.len().min(leftover.len());
            received[slot].extend(leftover.drain(..take));
        }
        // Unreachable while the quotas above cover every dropout: `entrants`
        // was truncated to `dropouts.len()` and each retained plan holds only
        // placed clubs, so the drain always empties `leftover`. Kept as a
        // backstop in case a future rule awards places without a matching
        // quota; drop it once that relationship is enforced by construction.
        for (offset, club) in leftover.into_iter().enumerate() {
            received[offset % feeder_plans.len()].push(club);
        }
        for ((index, promoted), arrivals) in feeder_plans.into_iter().zip(received) {
            let participants = &mut game.competitions[index].participant_ids;
            participants.retain(|id| !promoted.contains(id));
            participants.extend(arrivals);
        }
    }
}

#[cfg(test)]
#[path = "domestic_tests.rs"]
mod tests;
