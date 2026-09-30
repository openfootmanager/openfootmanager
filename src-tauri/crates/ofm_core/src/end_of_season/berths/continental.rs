//! Who qualifies for a competition fed by berths, and the fields those berths produce.
//!
//! Berth rules name a source competition and a rule for reading clubs out of it; this module
//! evaluates them. It covers continental qualification and the shared field resolution that
//! [`super::domestic`] also uses.
//!
//! The tests for this module live in `crates/ofm_core/tests/end_of_season_tests.rs`, because they
//! exercise it through the public re-exports rather than against its internals.

use crate::game::Game;
use domain::league::{Berth, BerthRule, CompetitionScope, CompetitionType, League};

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
pub(super) fn evaluate_berth_rule(source: &League, rule: &BerthRule) -> Vec<String> {
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

pub(super) struct BerthFieldOptions<S, E> {
    /// Which competitions may receive a field.
    pub(super) scope_match: S,
    /// Which of a source's berths award places on this path.
    pub(super) berth_eligible: E,
    /// Seed by reputation, cap to the field size, and top up a thin field.
    pub(super) fill: bool,
    /// Whether a berth's `fallbackTo` also offers the club a place. Continental
    /// qualification cascades this way; domestic promotion does not, because a
    /// club that misses its target simply stays in its own league.
    pub(super) follow_fallback: bool,
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
pub(super) fn resolve_berth_fields<S, E>(
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
