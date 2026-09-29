//! Promotion and relegation within a country's own league pyramid.
//!
//! The ladder is the set of league-table competitions sharing a country, ordered by `priority`
//! (lowest = highest division). Everything here decides which rungs take part and which clubs
//! swap; the berth-fed routes in and out of a pyramid live in [`super::continental`] and
//! [`super::domestic`].

use domain::league::{BerthRule, CompetitionFormat, CompetitionScope, CompetitionType, League};

/// Apply promotion/relegation within each domestic pyramid. A pyramid is the
/// set of league-table competitions sharing a country, ordered by `priority`
/// (lowest priority = highest division).
///
/// Competitions that are the *primary* `PositionRange` target of an incoming
/// berth are excluded from the linear adjacent-tier swap — both as a source
/// and as a destination — so a berth-fed Central League is not poisoned by
/// auto P/R. Leagues that share such a target are sibling regional groups and
/// leave the ladder as a set, so none of them is left swapping with the tier
/// below by accident of declaration order. A tier below a sibling group
/// therefore has no automatic promotion path; it needs berths of its own.
/// A league that is the *sole* `PositionRange` feeder into a target keeps its
/// ladder edge — that is an ordinary two-tier pyramid written as data.
/// Only a berth into a domestic league table makes siblings at all: a berth
/// into a continental cup is qualification rather than promotion, and every
/// first division carries one, so counting those would empty every ladder.
/// `CupWinner`, `PlayoffWinner`, and `fallback_to` targets stay
/// in the ladder (play-offs are Phase C.3b; fallbacks are continental).
pub(crate) fn apply_pyramid_promotion_relegation(competitions: &mut [League]) {
    use std::collections::{BTreeMap, HashMap, HashSet};

    let berth_targets: HashSet<&str> = competitions
        .iter()
        .flat_map(|competition| &competition.berths)
        .filter(|berth| matches!(berth.rule, BerthRule::PositionRange { .. }))
        .map(|berth| berth.target.as_str())
        .collect();

    // Leagues sharing a PositionRange target are sibling regional groups. They
    // exchange clubs with that target through berths, so the whole set leaves
    // the ladder together: splitting it pair by pair would leave whichever
    // group sorts last still chained to the tier below, making the outcome
    // depend on declaration order.
    //
    // Only a target a club can actually be *promoted into* counts. A berth into
    // a continental cup is qualification, not promotion — the club keeps its
    // league — and `create_game` gives every country's first division one such
    // berth into the same cup. Grouping on the target id alone therefore made
    // every first division in the world a sibling of every other, emptied each
    // country's ladder down to its second division, and stopped promotion and
    // relegation happening at all (#555). The predicate is the one
    // `apply_domestic_berth_promotion_relegation` uses to pick its targets, so
    // a league leaves the ladder exactly when the berth pass will move clubs
    // into that target on its behalf.
    let promotion_destinations: HashSet<&str> = competitions
        .iter()
        .filter(|competition| is_ladder_tier(competition))
        .map(|competition| competition.id.as_str())
        .collect();

    let mut feeders_by_target: HashMap<&str, Vec<&str>> = HashMap::new();
    for competition in competitions.iter() {
        if competition.rules.format != CompetitionFormat::LeagueTable {
            continue;
        }
        let targets: HashSet<&str> = competition
            .berths
            .iter()
            .filter(|berth| matches!(berth.rule, BerthRule::PositionRange { .. }))
            .map(|berth| berth.target.as_str())
            .filter(|target| promotion_destinations.contains(target))
            .collect();
        for target in targets {
            feeders_by_target
                .entry(target)
                .or_default()
                .push(competition.id.as_str());
        }
    }
    let sibling_feeders: HashSet<&str> = feeders_by_target
        .into_values()
        .filter(|feeders| feeders.len() > 1)
        .flatten()
        .collect();

    // Every rung of every country, excluded ones included: a tier that leaves
    // the ladder has to leave a *gap* in it, not be spirited away so that the
    // tiers on either side close up and become neighbours.
    let mut tiers_by_country: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, competition) in competitions.iter().enumerate() {
        if !is_ladder_tier(competition) {
            continue;
        }
        if let Some(country) = &competition.country_id {
            tiers_by_country
                .entry(country.clone())
                .or_default()
                .push(index);
        }
    }

    // Cut each country's tiers into runs at every excluded rung. A berth-fed
    // tier exchanges clubs through its berths instead, and the tiers above and
    // below it are then two divisions apart — closing that gap would promote a
    // third-division champion into the first. Collected before anything moves,
    // so the borrows the exclusion sets hold on `competitions` end here.
    let ladder_runs: Vec<Vec<usize>> = tiers_by_country
        .into_values()
        .flat_map(|mut indices| {
            indices.sort_by_key(|&index| competitions[index].priority);
            // Overlap is a property of the whole country, not of one run. Two
            // repeated phases of a pyramid separated by an excluded rung land
            // in different runs, and a per-run check never compares them: both
            // chained, and the phases stopped describing the same division —
            // a club first-division in one and second in the other. Refuse the
            // country outright, as the ladder did before it learned to split.
            if tiers_share_clubs(competitions, &indices) {
                log::warn!(
                    "[end-of-season] no promotion or relegation for {}: two tiers register the same club",
                    indices
                        .iter()
                        .map(|&index| competitions[index].id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Vec::new();
            }
            indices
                .split(|&index| {
                    let id = competitions[index].id.as_str();
                    berth_targets.contains(id) || sibling_feeders.contains(id)
                })
                .filter(|run| run.len() >= 2)
                .map(<[usize]>::to_vec)
                .collect::<Vec<_>>()
        })
        .collect();

    for run in ladder_runs {
        // A ladder is only meaningful over a run of tiers that have all
        // finished, rank unambiguously, and hold their own clubs. Anything else
        // is not a pyramid, and swapping across it moves clubs that should not
        // move. The run waits rather than half-exchanging.
        //
        // Say so. Every one of these refusals looks identical from the outside
        // — nobody is promoted, nobody is relegated, no message is sent — which
        // is exactly how a dead ladder went unnoticed in the first place.
        let refusal = if !every_tier_has_finished(competitions, &run) {
            Some("a tier has not finished its season")
        } else if !tiers_are_ranked_distinctly(competitions, &run) {
            Some("two tiers share a priority, so their order is arbitrary")
        } else {
            None
        };
        if let Some(reason) = refusal {
            log::warn!(
                "[end-of-season] no promotion or relegation for {}: {reason}",
                run.iter()
                    .map(|&index| competitions[index].id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            continue;
        }
        apply_linear_chain(competitions, &run);
    }
}

/// A rung of a domestic pyramid: the country's own league competition, played
/// as a table. A cup is not a rung even when it draws from the same clubs, and
/// neither is a regional side-competition — which is why Brazil's state cups
/// (`Cup` / `Regional` / `GroupAndKnockout`) cannot reach the ladder and cannot
/// trip the overlap check below.
pub(crate) fn is_ladder_tier(competition: &League) -> bool {
    competition.scope == CompetitionScope::Domestic
        && competition.kind == CompetitionType::League
        && competition.rules.format == CompetitionFormat::LeagueTable
}

/// True when every tier named by `indices` has played its season out.
///
/// A tier that is still running has no final table to promote from, and it used
/// to be dropped from the chain rather than stopping it — which left the tiers
/// on either side of it adjacent. A third division's champion was promoted
/// straight into the first while the second was mid-season, and the club it
/// displaced fell two divisions. A country's ladder therefore waits for all of
/// its tiers, and a hemisphere-foreign country simply rolls over on its own
/// calendar instead.
fn every_tier_has_finished(competitions: &[League], indices: &[usize]) -> bool {
    indices
        .iter()
        .all(|&index| crate::end_of_season::is_league_season_ended(&competitions[index]))
}

/// True when no two tiers claim the same `priority`.
///
/// `priority` is what says which division is above which, so two leagues
/// sharing one are peers — parallel regional groups, or a pyramid whose data is
/// simply wrong — and the sort order between them is arbitrary. Swapping across
/// that edge exchanges clubs between two leagues at the same level, decided by
/// declaration order. This is also what keeps the sibling rule's fail-closed
/// behaviour safe: feeders whose berth target does not exist stay on the
/// ladder, and staying on it must not mean trading clubs with each other.
fn tiers_are_ranked_distinctly(competitions: &[League], indices: &[usize]) -> bool {
    indices
        .windows(2)
        .all(|pair| competitions[pair[0]].priority != competitions[pair[1]].priority)
}

/// True when two of the leagues named by `indices` register the same club.
///
/// Tiers of a pyramid are disjoint, so this means the ladder has mistaken
/// something else for one. A split-season country is the case that exists:
/// `create_game` gives it an Apertura and a Clausura over the *same* division,
/// at consecutive priorities, which reads as two adjacent tiers. Swapping
/// between them puts the promoted club on a table it is already on, and the
/// regenerated schedule then has it playing itself.
///
/// Such a country keeps the no-promotion behaviour it has always had. Giving it
/// a working ladder means deciding which half of the season settles the
/// movement and applying the result to both halves' rosters — a gameplay
/// decision, not a bug fix, and none of the shipped split-season nations has
/// more than one division to promote into yet.
fn tiers_share_clubs(competitions: &[League], indices: &[usize]) -> bool {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    indices.iter().any(|&index| {
        competitions[index]
            .participant_ids
            .iter()
            .any(|club| !seen.insert(club.as_str()))
    })
}

fn apply_linear_chain(competitions: &mut [League], indices: &[usize]) {
    if indices.len() < 2 {
        return;
    }
    let mut divisions: Vec<League> = indices
        .iter()
        .map(|&index| competitions[index].clone())
        .collect();
    crate::promotion::apply_promotion_relegation(&mut divisions);
    for (slot, &index) in indices.iter().enumerate() {
        competitions[index].participant_ids = divisions[slot].participant_ids.clone();
    }
}

#[cfg(test)]
#[path = "pyramid_tests.rs"]
mod tests;
