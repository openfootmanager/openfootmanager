use crate::game::Game;
use domain::league::{
    Berth, BerthRule, CompetitionFormat, CompetitionScope, CompetitionType, League,
};

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
/// `CupWinner`, `PlayoffWinner`, and `fallback_to` targets stay
/// in the ladder (play-offs are Phase C.3b; fallbacks are continental).
pub(super) fn apply_pyramid_promotion_relegation(competitions: &mut [League]) {
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

    let mut tiers_by_country: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, competition) in competitions.iter().enumerate() {
        if competition.rules.format != CompetitionFormat::LeagueTable {
            continue;
        }
        if berth_targets.contains(competition.id.as_str())
            || sibling_feeders.contains(competition.id.as_str())
        {
            continue;
        }
        if !super::is_league_season_ended(competition) {
            continue; // mid-season foreign leagues skip this rollover's P/R
        }
        if let Some(country) = &competition.country_id {
            tiers_by_country
                .entry(country.clone())
                .or_default()
                .push(index);
        }
    }

    for mut indices in tiers_by_country.into_values() {
        if indices.len() < 2 {
            continue;
        }
        indices.sort_by_key(|&index| competitions[index].priority);
        apply_linear_chain(competitions, &indices);
    }
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

/// Domestic league finishes that earn a continental berth — the top N of each
/// first division. Cup winners qualify on top of these.
const CONTINENTAL_LEAGUE_SLOTS: usize = 4;

/// The clubs that qualify for a continental competition next season, decided by
/// the domestic season just completed: the top finishers of each first division
/// in the competition's feeder regions, plus domestic cup winners. The field is
/// seeded by reputation and capped to the competition's size; a thin field is
/// topped up by reputation so the bracket keeps its shape.
///
/// This is what makes domestic results feed continental qualification — a club
/// that finishes top of its league then plays continental football, instead of
/// the field being frozen at world creation. Read from final standings, so call
/// it before regeneration resets them.
pub fn continental_qualified_entrants(game: &Game, competition: &League) -> Vec<String> {
    use std::collections::{BTreeMap, HashSet};

    // Feeder regions: the competition's declared regions, or — if it declares
    // none — every region present in the domestic competition set.
    let feeder_regions: HashSet<String> = if competition.required_region_ids.is_empty() {
        game.competitions
            .iter()
            .filter_map(|c| c.region_id.clone())
            .collect()
    } else {
        competition.required_region_ids.iter().cloned().collect()
    };
    let in_feeder = |c: &League| {
        c.region_id
            .as_deref()
            .is_some_and(|region| feeder_regions.contains(region))
    };

    let mut qualified: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // The first division of each feeder country is its lowest-priority league.
    let mut first_division: BTreeMap<&str, &League> = BTreeMap::new();
    for competition in &game.competitions {
        if competition.scope != CompetitionScope::Domestic
            || competition.kind != CompetitionType::League
            || !in_feeder(competition)
        {
            continue;
        }
        let Some(country) = competition.country_id.as_deref() else {
            continue;
        };
        first_division
            .entry(country)
            .and_modify(|best| {
                if competition.priority < best.priority {
                    *best = competition;
                }
            })
            .or_insert(competition);
    }
    for league in first_division.values() {
        for entry in league
            .sorted_standings()
            .into_iter()
            .take(CONTINENTAL_LEAGUE_SLOTS)
        {
            if seen.insert(entry.team_id.clone()) {
                qualified.push(entry.team_id);
            }
        }
    }

    // Domestic cup winners earn a berth too.
    for competition in &game.competitions {
        if competition.scope != CompetitionScope::Domestic
            || competition.kind != CompetitionType::Cup
            || !in_feeder(competition)
        {
            continue;
        }
        if let Some(winner) = crate::world_cup::world_cup_champion(competition)
            && seen.insert(winner.clone())
        {
            qualified.push(winner);
        }
    }

    seed_cap_and_fill(game, competition, qualified, seen)
}

/// Whether any competition awards a berth into `target_id` — i.e. continental
/// qualification for that competition is data-defined rather than inferred.
pub fn competition_has_incoming_berths(game: &Game, target_id: &str) -> bool {
    game.competitions
        .iter()
        .flat_map(|source| &source.berths)
        .any(|berth| berth.target == target_id || berth.fallback_to.as_deref() == Some(target_id))
}

/// Teams a single berth rule selects from a competition's finished results.
/// `PlayoffWinner` is scheduled and resolved separately (Phase C.3b).
fn evaluate_berth_rule(source: &League, rule: &BerthRule) -> Vec<String> {
    match rule {
        BerthRule::PositionRange { from, to } => {
            let start = (*from as usize).saturating_sub(1);
            let count = (*to).saturating_sub(*from).saturating_add(1) as usize;
            source
                .sorted_standings()
                .into_iter()
                .skip(start)
                .take(count)
                .map(|entry| entry.team_id)
                .collect()
        }
        BerthRule::CupWinner => crate::world_cup::world_cup_champion(source)
            .into_iter()
            .collect(),
        BerthRule::PlayoffWinner { .. } => Vec::new(),
    }
}

/// Teams a single competition's results award to `target` via its berths.
fn berth_winners(source: &League, target_id: &str) -> Vec<String> {
    source
        .berths
        .iter()
        .filter(|berth| berth.target == target_id)
        .flat_map(|berth| evaluate_berth_rule(source, &berth.rule))
        .collect()
}

/// Continental field from data-defined berths: collect every competition's berth
/// winners for this target, then apply the same reputation seeding, field cap,
/// and top-up as the inferred path so a thin field still fills its bracket.
pub fn berth_qualified_entrants(game: &Game, target: &League) -> Vec<String> {
    use std::collections::HashSet;

    let mut qualified: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for source in &game.competitions {
        for team_id in berth_winners(source, &target.id) {
            if seen.insert(team_id.clone()) {
                qualified.push(team_id);
            }
        }
    }
    seed_cap_and_fill(game, target, qualified, seen)
}

struct BerthFieldOptions<S, E> {
    /// Which competitions may receive a field.
    scope_match: S,
    /// Which of a source's berths award places on this path.
    berth_eligible: E,
    /// Seed by reputation, cap to the field size, and top up a thin field.
    fill: bool,
    /// Whether a berth's `fallbackTo` also offers the club a place. Continental
    /// qualification cascades this way; domestic promotion does not, because a
    /// club that misses its target simply stays in its own league.
    follow_fallback: bool,
}

/// Resolve every berth-fed field whose target matches `options.scope_match`,
/// honouring cross-target exclusivity: a club ends in the single most
/// prestigious matching target (lowest priority) it earns. When
/// `options.follow_fallback` is set, a berth's `fallbackTo` offers the club a
/// further target on the same terms. Each field is emitted in evaluation order
/// (source list, then that source's berths, then standings) so an
/// oversubscribed domestic cap keeps a stable prefix rather than HashMap
/// iteration. Returns `target_id -> field`; targets without incoming berths
/// are absent.
fn resolve_berth_fields<S, E>(
    game: &Game,
    options: BerthFieldOptions<S, E>,
) -> std::collections::HashMap<String, Vec<String>>
where
    S: Fn(&League) -> bool,
    E: Fn(&League, &Berth) -> bool,
{
    use std::collections::{HashMap, HashSet};

    // Matching berth-fed targets, most prestigious (lowest priority) first.
    let mut targets: Vec<&League> = game
        .competitions
        .iter()
        .filter(|competition| {
            (options.scope_match)(competition)
                && competition_has_incoming_berths(game, &competition.id)
        })
        .collect();
    targets.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
    let priority_of: HashMap<&str, u32> = targets
        .iter()
        .map(|c| (c.id.as_str(), c.priority))
        .collect();

    // Each club keeps the most prestigious target any of its berths award it,
    // whether it reached that target as a primary or as a fallback.
    let mut best: HashMap<String, (u32, String)> = HashMap::new();
    let mut consider = |club: &str, target: &str| {
        if let Some(&prio) = priority_of.get(target) {
            let slot = best
                .entry(club.to_string())
                .or_insert((u32::MAX, String::new()));
            if prio < slot.0 {
                *slot = (prio, target.to_string());
            }
        }
    };
    for source in &game.competitions {
        for berth in &source.berths {
            if !(options.berth_eligible)(source, berth) {
                continue;
            }
            for winner in evaluate_berth_rule(source, &berth.rule) {
                consider(&winner, &berth.target);
                if options.follow_fallback
                    && let Some(fallback) = &berth.fallback_to
                {
                    consider(&winner, fallback);
                }
            }
        }
    }

    // Every club placed in any target — excluded from all targets' reputation
    // top-up so a thin field never pulls in a club already qualified elsewhere.
    let all_placed: HashSet<String> = best.keys().cloned().collect();
    // Replay evaluation order so each field's vec is stable: competitions,
    // then authored berths, then standings. HashMap iteration is not.
    let mut raw: HashMap<String, Vec<String>> = HashMap::new();
    let mut emitted: HashSet<String> = HashSet::new();
    for source in &game.competitions {
        for berth in &source.berths {
            if !(options.berth_eligible)(source, berth) {
                continue;
            }
            for winner in evaluate_berth_rule(source, &berth.rule) {
                if !emitted.insert(winner.clone()) {
                    continue;
                }
                if let Some((_, target)) = best.get(&winner) {
                    raw.entry(target.clone()).or_default().push(winner);
                }
            }
        }
    }

    let mut fields = HashMap::new();
    for target in &targets {
        let qualified = raw.remove(&target.id).unwrap_or_default();
        let field = if options.fill {
            seed_cap_and_fill(game, target, qualified, all_placed.clone())
        } else {
            qualified
        };
        fields.insert(target.id.clone(), field);
    }
    fields
}

/// Resolve every berth-fed continental field at once, honouring cross-target
/// exclusivity and the `fallbackTo` cascade: a club ends in the single most
/// prestigious target (lowest priority) it earns, and a berth's `fallbackTo`
/// is a lower-preference target used when the club doesn't earn the primary.
/// Returns `target_id -> field`; targets without incoming berths are absent
/// (the caller keeps the inferred path for those).
pub fn resolve_continental_fields(game: &Game) -> std::collections::HashMap<String, Vec<String>> {
    resolve_berth_fields(
        game,
        BerthFieldOptions {
            scope_match: |competition: &League| competition.scope == CompetitionScope::Continental,
            berth_eligible: |_: &League, _: &Berth| true,
            fill: true,
            follow_fallback: true,
        },
    )
}

/// Domestic league-table fields populated from sibling regional berths
/// (e.g. several state leagues feeding one national "Central League").
/// Same cross-target exclusivity as the continental path, but `fallbackTo` is
/// not followed: a club that misses its primary target has nowhere to drop to
/// domestically, so it stays in its own league. Targets without incoming
/// berths are absent.
pub(super) fn resolve_domestic_berth_fields(
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
pub(super) fn apply_domestic_berth_promotion_relegation(
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
        if !super::is_league_season_ended(&game.competitions[target_index]) {
            continue;
        }
        let unfinished_feeder = game.competitions.iter().enumerate().any(|(index, source)| {
            index != target_index
                && source.rules.format == CompetitionFormat::LeagueTable
                && source.berths.iter().any(|berth| {
                    berth.target == target_id
                        && matches!(berth.rule, BerthRule::PositionRange { .. })
                })
                && !super::is_league_season_ended(source)
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
            fields
                .get(&target_id)
                .into_iter()
                .flatten()
                .filter(|club| {
                    promotable.contains(club.as_str()) && !already_in_target.contains(club.as_str())
                })
                .cloned()
                .collect()
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

/// Shared tail for both qualification paths: seed by reputation, cap to the
/// target's field size, and top up a thin field from the feeder regions.
fn seed_cap_and_fill(
    game: &Game,
    competition: &League,
    mut qualified: Vec<String>,
    seen: std::collections::HashSet<String>,
) -> Vec<String> {
    let field_size = competition.participant_ids.len().max(4);
    let feeder_regions: std::collections::HashSet<String> =
        if competition.required_region_ids.is_empty() {
            game.competitions
                .iter()
                .filter_map(|c| c.region_id.clone())
                .collect()
        } else {
            competition.required_region_ids.iter().cloned().collect()
        };

    let reputation = |id: &str| {
        game.teams
            .iter()
            .find(|team| team.id == id)
            .map(|team| team.reputation)
            .unwrap_or(0)
    };
    qualified.sort_by(|a, b| reputation(b).cmp(&reputation(a)).then_with(|| a.cmp(b)));
    qualified.truncate(field_size);

    if qualified.len() < field_size {
        let mut fillers: Vec<_> = game
            .teams
            .iter()
            .filter(|team| !seen.contains(&team.id))
            .filter(|team| {
                feeder_regions.contains(game.region_for_country(&team.football_nation).as_str())
            })
            .collect();
        fillers.sort_by(|a, b| {
            b.reputation
                .cmp(&a.reputation)
                .then_with(|| a.id.cmp(&b.id))
        });
        for team in fillers {
            if qualified.len() >= field_size {
                break;
            }
            qualified.push(team.id.clone());
        }
    }

    qualified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        Berth, BerthRule, Fixture, FixtureCompetition, FixtureStatus, KnockoutRoundState,
        MatchResult, StandingEntry,
    };
    use domain::manager::Manager;
    use std::collections::HashSet;

    fn empty_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    /// Domestic league table with finished standings (`played > 0` so the
    /// season-ended guard treats it as complete). Clubs are listed best-first.
    fn division(id: &str, priority: u32, country: &str, standings: &[(&str, u32)]) -> League {
        let team_ids: Vec<String> = standings.iter().map(|(id, _)| id.to_string()).collect();
        let mut league = League::new(id.to_string(), id.to_string(), 2026, &team_ids);
        league.priority = priority;
        league.country_id = Some(country.to_string());
        league.standings = standings
            .iter()
            .map(|(team, points)| {
                let mut entry = StandingEntry::new(team.to_string());
                entry.points = *points;
                entry.played = 1;
                entry
            })
            .collect();
        league
    }

    fn position_berth(target: &str, from: u32, to: u32) -> Berth {
        Berth {
            target: target.to_string(),
            rule: BerthRule::PositionRange { from, to },
            fallback_to: None,
        }
    }

    fn two_region_central_pyramid() -> Game {
        let mut north = division(
            "north",
            1,
            "BR",
            &[("n1", 40), ("n2", 30), ("n3", 20), ("n4", 10)],
        );
        north.berths = vec![position_berth("central", 1, 2)];
        let mut south = division(
            "south",
            1,
            "BR",
            &[("s1", 40), ("s2", 30), ("s3", 20), ("s4", 10)],
        );
        south.berths = vec![position_berth("central", 1, 2)];
        let central = division(
            "central",
            0,
            "BR",
            &[
                ("c1", 80),
                ("c2", 70),
                ("c3", 60),
                ("c4", 50),
                ("c5", 40),
                ("c6", 30),
                ("c7", 20),
                ("c8", 10),
            ],
        );
        let mut game = empty_game();
        game.competitions = vec![central, north, south];
        game
    }

    /// 4-club Central, four 2-club feeders each sending two — 8 winners, 4 places.
    fn four_region_oversubscribed_pyramid() -> Game {
        let central = division(
            "central",
            0,
            "BR",
            &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
        );
        let regions = [
            ("north", "n1", "n2"),
            ("south", "s1", "s2"),
            ("east", "e1", "e2"),
            ("west", "w1", "w2"),
        ];
        let mut game = empty_game();
        game.competitions = std::iter::once(central)
            .chain(regions.iter().map(|(id, a, b)| {
                let mut league = division(id, 1, "BR", &[(a, 20), (b, 10)]);
                league.berths = vec![position_berth("central", 1, 2)];
                league
            }))
            .collect();
        game
    }

    fn filler_club(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
        let mut team = domain::team::Team::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            nation.to_string(),
            "City".to_string(),
            "Stadium".to_string(),
            10_000,
        );
        team.football_nation = nation.to_string();
        team.reputation = reputation;
        team
    }

    #[test]
    fn resolve_domestic_berth_fields_collects_regional_place_getters() {
        let mut game = two_region_central_pyramid();
        for competition in &mut game.competitions {
            competition.region_id = Some("sa".to_string());
        }
        // High-reputation outsiders must not pad a domestic table: fill is a
        // continental-bracket concern only.
        game.teams.push(filler_club("rep-star", "BR", 99));
        // A continental target with incoming berths must not leak into the
        // domestic map — scope filtering is the whole point of the split.
        let mut continental = League::new(
            "ucl".to_string(),
            "UCL".to_string(),
            2026,
            &["seed-0".to_string(), "seed-1".to_string()],
        );
        continental.scope = CompetitionScope::Continental;
        continental.kind = CompetitionType::ContinentalClub;
        game.competitions[1]
            .berths
            .push(position_berth("ucl", 1, 1));
        game.competitions.push(continental);

        let domestic = resolve_domestic_berth_fields(&game);
        let continental_fields = resolve_continental_fields(&game);

        let central = domestic
            .get("central")
            .expect("Central League is berth-fed");
        let winners: HashSet<&str> = central.iter().map(String::as_str).collect();
        assert_eq!(winners, HashSet::from(["n1", "n2", "s1", "s2"]));
        assert!(
            !domestic.contains_key("ucl"),
            "continental targets stay out of the domestic map: {domestic:?}"
        );
        assert!(
            !domestic.contains_key("north") && !domestic.contains_key("south"),
            "feeders without incoming berths are absent: {domestic:?}"
        );
        assert!(
            continental_fields.contains_key("ucl"),
            "continental resolution is unchanged for berth-fed cups"
        );
        assert!(
            !continental_fields.contains_key("central"),
            "a domestic league must not appear in the continental map"
        );
    }

    #[test]
    fn resolve_domestic_berth_fields_preserves_evaluation_order() {
        let game = four_region_oversubscribed_pyramid();
        let field = resolve_domestic_berth_fields(&game)
            .remove("central")
            .expect("berth-fed");
        assert_eq!(
            field,
            vec![
                "n1".to_string(),
                "n2".to_string(),
                "s1".to_string(),
                "s2".to_string(),
                "e1".to_string(),
                "e2".to_string(),
                "w1".to_string(),
                "w2".to_string(),
            ],
            "field order is competition then berth then standings, not HashMap iteration"
        );
    }

    #[test]
    fn apply_pyramid_excludes_berth_targets_and_leaves_plain_leagues_alone() {
        // Country BR: Central (berth-fed) sits above a regional feeder and a
        // tier below that feeder. Without exclusion the linear chain would
        // swap Central's bottom club with the feeder's champion.
        let mut north = division(
            "north",
            1,
            "BR",
            &[
                ("n1", 60),
                ("n2", 50),
                ("n3", 40),
                ("n4", 30),
                ("n5", 20),
                ("n6", 10),
            ],
        );
        north.berths = vec![position_berth("central", 1, 2)];
        let central = division(
            "central",
            0,
            "BR",
            &[
                ("c1", 60),
                ("c2", 50),
                ("c3", 40),
                ("c4", 30),
                ("c5", 20),
                ("c6", 10),
            ],
        );
        let interior = division(
            "interior",
            2,
            "BR",
            &[
                ("i1", 60),
                ("i2", 50),
                ("i3", 40),
                ("i4", 30),
                ("i5", 20),
                ("i6", 10),
            ],
        );
        // Country ENG: a plain two-tier pyramid with no incoming berths.
        let eng_top = division(
            "eng-1",
            0,
            "ENG",
            &[
                ("t1", 60),
                ("t2", 50),
                ("t3", 40),
                ("t4", 30),
                ("t5", 20),
                ("t6", 10),
            ],
        );
        let eng_second = division(
            "eng-2",
            1,
            "ENG",
            &[
                ("s1", 60),
                ("s2", 50),
                ("s3", 40),
                ("s4", 30),
                ("s5", 20),
                ("s6", 10),
            ],
        );

        let central_before = central.participant_ids.clone();
        let mut competitions = vec![central, north, interior, eng_top, eng_second];
        apply_pyramid_promotion_relegation(&mut competitions);

        let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);

        assert_eq!(
            by_id("central").participant_ids,
            central_before,
            "a berth-fed Central League must be excluded from linear P/R"
        );
        // Feeder vs the tier below it still swaps (6-club → one up / one down).
        let north_ids: HashSet<&String> = by_id("north").participant_ids.iter().collect();
        let interior_ids: HashSet<&String> = by_id("interior").participant_ids.iter().collect();
        assert!(
            north_ids.contains(&"i1".to_string()),
            "feeder still receives the lower-tier champion: {:?}",
            by_id("north").participant_ids
        );
        assert!(
            interior_ids.contains(&"n6".to_string()),
            "lower tier still receives the feeder's bottom club: {:?}",
            by_id("interior").participant_ids
        );
        assert!(!north_ids.contains(&"n6".to_string()));
        assert!(!interior_ids.contains(&"i1".to_string()));

        // No-regression: a country without incoming berths still promotes.
        let eng1: HashSet<&String> = by_id("eng-1").participant_ids.iter().collect();
        let eng2: HashSet<&String> = by_id("eng-2").participant_ids.iter().collect();
        assert!(
            eng1.contains(&"s1".to_string()),
            "plain top division promotes"
        );
        assert!(
            !eng1.contains(&"t6".to_string()),
            "plain top division relegates"
        );
        assert!(eng2.contains(&"t6".to_string()));
        assert!(!eng2.contains(&"s1".to_string()));
    }

    #[test]
    fn apply_pyramid_does_not_swap_sibling_position_range_feeders() {
        // Two same-priority regional groups that both send PositionRange
        // berths to the same Central League must not linearly swap — that
        // would put a promoted club on two tables after the berth merge.
        let mut north = division(
            "north",
            1,
            "BR",
            &[("n1", 40), ("n2", 30), ("n3", 20), ("n4", 10)],
        );
        north.berths = vec![position_berth("central", 1, 2)];
        let mut south = division(
            "south",
            1,
            "BR",
            &[("s1", 40), ("s2", 30), ("s3", 20), ("s4", 10)],
        );
        south.berths = vec![position_berth("central", 1, 2)];
        let north_before = north.participant_ids.clone();
        let south_before = south.participant_ids.clone();

        let mut competitions = vec![north, south];
        apply_pyramid_promotion_relegation(&mut competitions);

        let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
        assert_eq!(
            by_id("north").participant_ids,
            north_before,
            "sibling feeders must not linearly swap: {:?}",
            by_id("north").participant_ids
        );
        assert_eq!(
            by_id("south").participant_ids,
            south_before,
            "sibling feeders must not linearly swap: {:?}",
            by_id("south").participant_ids
        );
    }

    #[test]
    fn apply_pyramid_takes_every_sibling_feeder_off_the_ladder() {
        // Three groups feed one Central League. Splitting the chain pair by
        // pair leaves the last group attached to the tier below it, so which
        // group swaps with `lower` would depend on competition order alone.
        let mut competitions = vec![
            division("lower", 2, "BR", &[("d1", 20), ("d2", 10)]),
            division(
                "central",
                0,
                "BR",
                &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
            ),
        ];
        for (id, first, second) in [
            ("north", "n1", "n2"),
            ("south", "s1", "s2"),
            ("east", "e1", "e2"),
        ] {
            let mut group = division(id, 1, "BR", &[(first, 20), (second, 10)]);
            group.berths = vec![position_berth("central", 1, 1)];
            competitions.push(group);
        }
        let before: Vec<(String, Vec<String>)> = competitions
            .iter()
            .map(|c| (c.id.clone(), c.participant_ids.clone()))
            .collect();

        apply_pyramid_promotion_relegation(&mut competitions);

        for (competition, (id, expected)) in competitions.iter().zip(&before) {
            assert_eq!(
                &competition.participant_ids, expected,
                "sibling feeders and their target stay off the linear ladder, whatever \
                 order they are declared in: {id} became {:?}",
                competition.participant_ids
            );
        }
    }

    #[test]
    fn apply_pyramid_still_swaps_when_top_flight_is_only_a_non_position_berth_target() {
        // CupWinner, PlayoffWinner, and fallback_to must not pull a league out
        // of the linear pyramid — only a primary PositionRange target does.
        let eng_top = division(
            "eng-1",
            0,
            "ENG",
            &[
                ("t1", 60),
                ("t2", 50),
                ("t3", 40),
                ("t4", 30),
                ("t5", 20),
                ("t6", 10),
            ],
        );
        let mut eng_second = division(
            "eng-2",
            1,
            "ENG",
            &[
                ("s1", 60),
                ("s2", 50),
                ("s3", 40),
                ("s4", 30),
                ("s5", 20),
                ("s6", 10),
            ],
        );
        eng_second.berths = vec![Berth {
            target: "eng-1".to_string(),
            rule: BerthRule::PlayoffWinner { from: 3, to: 6 },
            fallback_to: None,
        }];
        let mut cup = League::new(
            "eng-cup".to_string(),
            "Cup".to_string(),
            2026,
            &["t1".to_string(), "s1".to_string()],
        );
        cup.kind = CompetitionType::Cup;
        cup.country_id = Some("ENG".to_string());
        cup.rules.format = CompetitionFormat::Knockout;
        cup.berths = vec![Berth {
            target: "eng-1".to_string(),
            rule: BerthRule::CupWinner,
            fallback_to: None,
        }];
        let mut other = division("other", 0, "WAL", &[("w1", 20), ("w2", 10)]);
        other.berths = vec![Berth {
            target: "missing-continental".to_string(),
            rule: BerthRule::PositionRange { from: 1, to: 1 },
            fallback_to: Some("eng-1".to_string()),
        }];

        let mut competitions = vec![eng_top, eng_second, cup, other];
        apply_pyramid_promotion_relegation(&mut competitions);

        let by_id = |id: &str| competitions.iter().find(|c| c.id == id).expect(id);
        let eng1: HashSet<&String> = by_id("eng-1").participant_ids.iter().collect();
        let eng2: HashSet<&String> = by_id("eng-2").participant_ids.iter().collect();
        assert!(
            eng1.contains(&"s1".to_string()),
            "CupWinner / PlayoffWinner / fallback_to must not block linear promotion: {:?}",
            by_id("eng-1").participant_ids
        );
        assert!(!eng1.contains(&"t6".to_string()));
        assert!(eng2.contains(&"t6".to_string()));
        assert!(!eng2.contains(&"s1".to_string()));
    }

    #[test]
    fn cup_winner_into_top_flight_does_not_block_linear_pr_or_mutate_the_cup() {
        // CupWinner → top flight must not become a domestic entrant or turn
        // the cup into a feeder. Linear P/R with the adjacent tier still runs.
        let eng_top = division(
            "eng-1",
            0,
            "ENG",
            &[
                ("t1", 60),
                ("t2", 50),
                ("t3", 40),
                ("t4", 30),
                ("t5", 20),
                ("t6", 10),
            ],
        );
        let eng_second = division(
            "eng-2",
            1,
            "ENG",
            &[
                ("s1", 60),
                ("s2", 50),
                ("s3", 40),
                ("s4", 30),
                ("s5", 20),
                ("s6", 10),
            ],
        );
        let final_id = "eng-cup-final";
        let mut cup = League::new(
            "eng-cup".to_string(),
            "Cup".to_string(),
            2026,
            &["t1".to_string(), "s1".to_string()],
        );
        cup.kind = CompetitionType::Cup;
        cup.country_id = Some("ENG".to_string());
        cup.rules.format = CompetitionFormat::Knockout;
        cup.berths = vec![Berth {
            target: "eng-1".to_string(),
            rule: BerthRule::CupWinner,
            fallback_to: None,
        }];
        cup.fixtures = vec![Fixture {
            id: final_id.to_string(),
            matchday: 1,
            date: "2026-05-01".to_string(),
            home_team_id: "t1".to_string(),
            away_team_id: "s1".to_string(),
            competition: FixtureCompetition::Cup,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: 2,
                away_goals: 1,
                ..Default::default()
            }),
            ..Default::default()
        }];
        cup.knockout_rounds = vec![KnockoutRoundState {
            id: "eng-cup-final-round".to_string(),
            name: "Final".to_string(),
            fixture_ids: vec![final_id.to_string()],
            bye_team_ids: vec![],
            completed: true,
        }];
        let cup_before = cup.participant_ids.clone();

        let mut game = empty_game();
        game.competitions = vec![eng_top, eng_second, cup];

        let fields = resolve_domestic_berth_fields(&game);
        apply_pyramid_promotion_relegation(&mut game.competitions);
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
        assert_eq!(
            by_id("eng-cup").participant_ids,
            cup_before,
            "CupWinner must not turn the cup into a feeder: {:?}",
            by_id("eng-cup").participant_ids
        );
        let eng1_ids: Vec<&str> = by_id("eng-1")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(
            eng1_ids.iter().filter(|id| **id == "t1").count(),
            1,
            "cup winner already in top flight must not be duplicated: {eng1_ids:?}"
        );
        let eng1: HashSet<&str> = eng1_ids.iter().copied().collect();
        let eng2: HashSet<&str> = by_id("eng-2")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert!(
            eng1.contains("s1"),
            "linear P/R must still promote the feeder champion: {:?}",
            by_id("eng-1").participant_ids
        );
        assert!(!eng1.contains("t6"));
        assert!(eng2.contains("t6"));
        assert!(!eng2.contains("s1"));
        assert_eq!(
            by_id("eng-1").participant_ids.len(),
            6,
            "top flight keeps its authored size: {:?}",
            by_id("eng-1").participant_ids
        );
    }

    #[test]
    fn apply_domestic_berth_merges_two_regional_groups_into_central() {
        let mut game = two_region_central_pyramid();
        let fields = resolve_domestic_berth_fields(&game);

        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);

        let central = &by_id("central").participant_ids;
        assert_eq!(
            central.len(),
            8,
            "Central League keeps its authored size: {central:?}"
        );
        // Survivors retained in place; the four regional winners appended.
        assert_eq!(
            &central[..4],
            &[
                "c1".to_string(),
                "c2".to_string(),
                "c3".to_string(),
                "c4".to_string()
            ],
            "top-four finishers survive: {central:?}"
        );
        let promoted: HashSet<&str> = central[4..].iter().map(String::as_str).collect();
        assert_eq!(promoted, HashSet::from(["n1", "n2", "s1", "s2"]));

        let north: HashSet<&str> = by_id("north")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        let south: HashSet<&str> = by_id("south")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert!(
            !north.contains("n1") && !north.contains("n2"),
            "north sent its place-getters up: {:?}",
            by_id("north").participant_ids
        );
        assert!(
            !south.contains("s1") && !south.contains("s2"),
            "south sent its place-getters up: {:?}",
            by_id("south").participant_ids
        );
        // Four dropouts (c5–c8) split round-robin across the two feeders.
        let dropouts: HashSet<&str> = north
            .union(&south)
            .copied()
            .filter(|id| id.starts_with('c'))
            .collect();
        assert_eq!(dropouts, HashSet::from(["c5", "c6", "c7", "c8"]));
        assert_eq!(
            by_id("north").participant_ids.len(),
            4,
            "north keeps its authored size"
        );
        assert_eq!(
            by_id("south").participant_ids.len(),
            4,
            "south keeps its authored size"
        );
        assert_eq!(
            north.intersection(&south).count(),
            0,
            "each dropout lands in exactly one feeder"
        );
    }

    #[test]
    fn domestic_promotion_ignores_a_fallback_target() {
        // `fallbackTo` is a continental cascade. A club that misses its
        // primary target has nowhere to drop to domestically — it stays put.
        // Following the fallback here would add it to the fallback league
        // while the feeder-removal path, which only matches a direct target,
        // left it where it was.
        let eng = division(
            "eng-1",
            0,
            "ENG",
            &[("t1", 40), ("t2", 30), ("t3", 20), ("t4", 10)],
        );
        let mut wales = division("wal-1", 0, "WAL", &[("w1", 20), ("w2", 10)]);
        wales.berths = vec![Berth {
            target: "missing-continental".to_string(),
            rule: BerthRule::PositionRange { from: 1, to: 1 },
            fallback_to: Some("eng-1".to_string()),
        }];
        let mut game = empty_game();
        game.competitions = vec![eng, wales];

        let fields = resolve_domestic_berth_fields(&game);
        assert!(
            !fields
                .get("eng-1")
                .is_some_and(|field| field.contains(&"w1".to_string())),
            "a domestic fallback must not award a place: {fields:?}"
        );

        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
        assert_eq!(
            by_id("wal-1").participant_ids,
            vec!["w1".to_string(), "w2".to_string()],
            "the club stays in its own league"
        );
        assert_eq!(
            by_id("eng-1").participant_ids,
            vec![
                "t1".to_string(),
                "t2".to_string(),
                "t3".to_string(),
                "t4".to_string()
            ],
            "the fallback league neither gains a club nor releases a place"
        );
    }

    #[test]
    fn apply_domestic_berth_is_noop_without_incoming_berths() {
        let mut game = empty_game();
        let plain = division("eng-1", 0, "ENG", &[("t1", 30), ("t2", 20), ("t3", 10)]);
        game.competitions = vec![plain];
        let before = game.competitions[0].clone();

        let fields = resolve_domestic_berth_fields(&game);
        assert!(
            fields.is_empty(),
            "a league with no incoming berths is absent from the field map"
        );
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        assert_eq!(
            serde_json::to_vec(&game.competitions[0]).expect("serialize after"),
            serde_json::to_vec(&before).expect("serialize before"),
            "plain leagues must be byte-for-byte unchanged"
        );
    }

    #[test]
    fn apply_domestic_berth_merges_four_regional_champions() {
        let central = division(
            "central",
            0,
            "BR",
            &[
                ("c1", 80),
                ("c2", 70),
                ("c3", 60),
                ("c4", 50),
                ("c5", 40),
                ("c6", 30),
                ("c7", 20),
                ("c8", 10),
            ],
        );
        let regions = [
            ("north", "n1", "n2"),
            ("south", "s1", "s2"),
            ("east", "e1", "e2"),
            ("west", "w1", "w2"),
        ];
        let mut game = empty_game();
        game.competitions = std::iter::once(central)
            .chain(regions.iter().map(|(id, champ, runner)| {
                let mut league = division(id, 1, "BR", &[(champ, 20), (runner, 10)]);
                league.berths = vec![position_berth("central", 1, 1)];
                league
            }))
            .collect();

        let fields = resolve_domestic_berth_fields(&game);
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);

        let central = &by_id("central").participant_ids;
        assert_eq!(central.len(), 8);
        let promoted: HashSet<&str> = central[4..].iter().map(String::as_str).collect();
        assert_eq!(promoted, HashSet::from(["n1", "s1", "e1", "w1"]));
        assert_eq!(
            &central[..4],
            &[
                "c1".to_string(),
                "c2".to_string(),
                "c3".to_string(),
                "c4".to_string()
            ]
        );

        let mut received_dropouts = HashSet::new();
        for (id, champ, _runner) in regions {
            let ids: HashSet<&str> = by_id(id)
                .participant_ids
                .iter()
                .map(String::as_str)
                .collect();
            assert!(
                !ids.contains(champ),
                "{id} must lose its champion {champ}: {:?}",
                by_id(id).participant_ids
            );
            assert_eq!(by_id(id).participant_ids.len(), 2);
            let dropout = by_id(id)
                .participant_ids
                .iter()
                .find(|club| club.starts_with('c'))
                .expect("each feeder receives one Central dropout");
            received_dropouts.insert(dropout.as_str());
        }
        assert_eq!(received_dropouts, HashSet::from(["c5", "c6", "c7", "c8"]));
    }

    #[test]
    fn apply_domestic_berth_preserves_feeder_size_when_quotas_differ() {
        // North sends three, South sends one. A naive 2+2 split of the four
        // Central dropouts would shrink North and grow South.
        let central = division(
            "central",
            0,
            "BR",
            &[
                ("c1", 80),
                ("c2", 70),
                ("c3", 60),
                ("c4", 50),
                ("c5", 40),
                ("c6", 30),
                ("c7", 20),
                ("c8", 10),
            ],
        );
        let mut north = division(
            "north",
            1,
            "BR",
            &[("n1", 40), ("n2", 30), ("n3", 20), ("n4", 10)],
        );
        north.berths = vec![position_berth("central", 1, 3)];
        let mut south = division(
            "south",
            1,
            "BR",
            &[("s1", 40), ("s2", 30), ("s3", 20), ("s4", 10)],
        );
        south.berths = vec![position_berth("central", 1, 1)];
        let mut game = empty_game();
        game.competitions = vec![central, north, south];

        let fields = resolve_domestic_berth_fields(&game);
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);

        let central = &by_id("central").participant_ids;
        assert_eq!(
            central.len(),
            8,
            "Central League keeps its authored size: {central:?}"
        );
        let central_set: HashSet<&str> = central.iter().map(String::as_str).collect();
        assert!(
            ["n1", "n2", "n3", "s1"]
                .iter()
                .all(|club| central_set.contains(club)),
            "unequal place-getters must all promote: {central:?}"
        );
        assert!(
            ["c5", "c6", "c7", "c8"]
                .iter()
                .all(|club| !central_set.contains(club)),
            "Central dropouts must leave: {central:?}"
        );

        let north: HashSet<&str> = by_id("north")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        let south: HashSet<&str> = by_id("south")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert!(
            !north.contains("n1") && !north.contains("n2") && !north.contains("n3"),
            "north sent its three place-getters up: {:?}",
            by_id("north").participant_ids
        );
        assert!(
            !south.contains("s1"),
            "south sent its champion up: {:?}",
            by_id("south").participant_ids
        );
        assert_eq!(
            by_id("north").participant_ids.len(),
            4,
            "north keeps its authored size after receiving three dropouts: {:?}",
            by_id("north").participant_ids
        );
        assert_eq!(
            by_id("south").participant_ids.len(),
            4,
            "south keeps its authored size after receiving one dropout: {:?}",
            by_id("south").participant_ids
        );
        let north_dropouts: HashSet<&str> = north
            .iter()
            .copied()
            .filter(|id| id.starts_with('c'))
            .collect();
        let south_dropouts: HashSet<&str> = south
            .iter()
            .copied()
            .filter(|id| id.starts_with('c'))
            .collect();
        assert_eq!(
            north_dropouts.len(),
            3,
            "north receives exactly as many dropouts as it promoted: {:?}",
            by_id("north").participant_ids
        );
        assert_eq!(
            south_dropouts.len(),
            1,
            "south receives exactly as many dropouts as it promoted: {:?}",
            by_id("south").participant_ids
        );
        assert_eq!(
            north_dropouts
                .union(&south_dropouts)
                .copied()
                .collect::<HashSet<_>>(),
            HashSet::from(["c5", "c6", "c7", "c8"])
        );
        assert_eq!(
            north.intersection(&south).count(),
            0,
            "each dropout lands in exactly one feeder"
        );
    }

    #[test]
    fn apply_domestic_berth_keeps_surplus_place_getters_in_their_feeder() {
        // 4-club target, four feeders each sending two → 8 winners, 4 places.
        // Evaluation order is north, south, east, west (1st then 2nd). The
        // prefix takes the released places; the rest stay in their feeder.
        let mut game = four_region_oversubscribed_pyramid();
        let fields = resolve_domestic_berth_fields(&game);
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
        let central: HashSet<&str> = by_id("central")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(
            central,
            HashSet::from(["n1", "n2", "s1", "s2"]),
            "evaluation-order prefix occupies the released places: {:?}",
            by_id("central").participant_ids
        );

        let feeder = |id: &str| -> HashSet<&str> {
            by_id(id)
                .participant_ids
                .iter()
                .map(String::as_str)
                .collect()
        };
        assert_eq!(
            by_id("north").participant_ids.len(),
            2,
            "north keeps authored size: {:?}",
            by_id("north").participant_ids
        );
        assert_eq!(
            by_id("south").participant_ids.len(),
            2,
            "south keeps authored size: {:?}",
            by_id("south").participant_ids
        );
        assert!(!feeder("north").contains("n1") && !feeder("north").contains("n2"));
        assert!(!feeder("south").contains("s1") && !feeder("south").contains("s2"));
        assert!(
            feeder("east").contains("e1") && feeder("east").contains("e2"),
            "east surplus stay put: {:?}",
            by_id("east").participant_ids
        );
        assert!(
            feeder("west").contains("w1") && feeder("west").contains("w2"),
            "west surplus stay put: {:?}",
            by_id("west").participant_ids
        );
        assert_eq!(by_id("east").participant_ids.len(), 2);
        assert_eq!(by_id("west").participant_ids.len(), 2);
    }

    #[test]
    fn apply_domestic_berth_does_not_promote_a_club_the_ladder_already_relegated() {
        // A feeder that both sends clubs up and swaps with the tier below is
        // handled by both passes in one rollover. The linear pass relegates
        // `m2` into `low`; the berth merge must not then promote it into
        // `central` off the frozen table, or `m2` plays in two leagues.
        let central = division(
            "central",
            0,
            "BR",
            &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
        );
        let mut mid = division("mid", 1, "BR", &[("m1", 20), ("m2", 10)]);
        mid.berths = vec![position_berth("central", 1, 2)];
        let low = division("low", 2, "BR", &[("l1", 20), ("l2", 10)]);

        let mut game = empty_game();
        game.competitions = vec![central, mid, low];

        let fields = resolve_domestic_berth_fields(&game);
        apply_pyramid_promotion_relegation(&mut game.competitions);
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
        let roster = |id: &str| -> HashSet<&str> {
            by_id(id)
                .participant_ids
                .iter()
                .map(String::as_str)
                .collect()
        };

        assert_eq!(
            by_id("central").participant_ids.len(),
            4,
            "central keeps its authored size: {:?}",
            by_id("central").participant_ids
        );
        assert_eq!(
            by_id("mid").participant_ids.len(),
            2,
            "mid keeps its authored size: {:?}",
            by_id("mid").participant_ids
        );
        assert_eq!(
            by_id("low").participant_ids.len(),
            2,
            "low keeps its authored size: {:?}",
            by_id("low").participant_ids
        );

        let (central_ids, mid_ids, low_ids) = (roster("central"), roster("mid"), roster("low"));
        assert!(
            central_ids.is_disjoint(&mid_ids)
                && central_ids.is_disjoint(&low_ids)
                && mid_ids.is_disjoint(&low_ids),
            "no club may sit on two tables: central {:?} mid {:?} low {:?}",
            by_id("central").participant_ids,
            by_id("mid").participant_ids,
            by_id("low").participant_ids
        );
        assert!(
            low_ids.contains("m2"),
            "the ladder's relegation stands: {:?}",
            by_id("low").participant_ids
        );
        assert!(
            central_ids.contains("m1"),
            "the place-getter still in its feeder is promoted: {:?}",
            by_id("central").participant_ids
        );
    }

    #[test]
    fn apply_domestic_berth_skips_when_target_or_feeder_is_unfinished() {
        // Linear P/R already leaves mid-season tables alone. The berth merge
        // must do the same for the whole target — skipping only an unfinished
        // feeder while still taking its clubs would duplicate them.
        let scheduled = Fixture {
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            ..Default::default()
        };
        for unfinished_id in ["central", "north"] {
            let mut game = two_region_central_pyramid();
            game.competitions
                .iter_mut()
                .find(|competition| competition.id == unfinished_id)
                .expect(unfinished_id)
                .fixtures
                .push(scheduled.clone());
            let before: Vec<Vec<String>> = game
                .competitions
                .iter()
                .map(|competition| competition.participant_ids.clone())
                .collect();

            let fields = resolve_domestic_berth_fields(&game);
            apply_domestic_berth_promotion_relegation(&mut game, &fields);

            for (competition, expected) in game.competitions.iter().zip(&before) {
                assert_eq!(
                    &competition.participant_ids, expected,
                    "unfinished {unfinished_id} must leave {} unchanged: {:?}",
                    competition.id, competition.participant_ids
                );
            }
        }
    }

    #[test]
    fn apply_domestic_berth_ignores_unfinished_non_position_range_source() {
        // A mid-season PlayoffWinner table aimed at Central must not block
        // finished PositionRange feeders — resolve cannot take those clubs.
        let scheduled = Fixture {
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            ..Default::default()
        };
        let mut game = two_region_central_pyramid();
        let mut playoff = division("playoff", 2, "BR", &[("p1", 20), ("p2", 10)]);
        playoff.berths = vec![Berth {
            target: "central".to_string(),
            rule: BerthRule::PlayoffWinner { from: 1, to: 2 },
            fallback_to: None,
        }];
        playoff.fixtures.push(scheduled);
        game.competitions.push(playoff);

        let fields = resolve_domestic_berth_fields(&game);
        apply_domestic_berth_promotion_relegation(&mut game, &fields);

        let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
        let central: HashSet<&str> = by_id("central")
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert!(
            ["n1", "n2", "s1", "s2"]
                .iter()
                .all(|club| central.contains(club)),
            "finished feeders still promote: {:?}",
            by_id("central").participant_ids
        );
        assert!(
            ["c5", "c6", "c7", "c8"]
                .iter()
                .all(|club| !central.contains(club)),
            "Central dropouts must leave: {:?}",
            by_id("central").participant_ids
        );
        assert_eq!(
            by_id("playoff").participant_ids,
            vec!["p1".to_string(), "p2".to_string()],
            "PlayoffWinner source is not a feeder"
        );
    }
}
