use crate::game::Game;
use crate::season_awards::compute_division_season_awards;
use chrono::{DateTime, Datelike, Duration, Utc};
use domain::league::{
    BerthRule, CompetitionFormat, CompetitionScope, CompetitionType, FixtureStatus, League,
    StandingEntry,
};
use domain::manager::Manager;
use domain::message::*;
use domain::player::PlayerSeasonStats;
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::{FinancialTransaction, FinancialTransactionKind, TeamSeasonRecord};

pub fn expected_fixture_count(team_count: usize) -> Option<usize> {
    if team_count >= 2 {
        // Double round robin: every club plays every other home and away.
        // Odd-sized leagues reach the same total via byes.
        Some(team_count * (team_count - 1))
    } else {
        None
    }
}

pub fn has_full_schedule(league: &League) -> bool {
    match expected_fixture_count(league.standings.len()) {
        Some(expected_fixture_count) => {
            league
                .fixtures
                .iter()
                .filter(|fixture| fixture.counts_for_league_standings())
                .count()
                == expected_fixture_count
        }
        None => false,
    }
}

fn free_agent_team_name() -> String {
    ["Free", "Agent"].join(" ")
}

/// Returns true if at least one competitive fixture has been completed or any
/// standing entry records a played match. Used as a guard to prevent premature
/// end-of-season processing for a season that has not yet kicked off.
pub fn season_has_started(league: &League) -> bool {
    league
        .fixtures
        .iter()
        .any(|f| f.counts_for_league_standings() && f.status == FixtureStatus::Completed)
        || league.standings.iter().any(|e| e.played > 0)
}

/// True when a league's season has started and no more scheduled fixtures
/// remain. Unlike `is_league_complete` this does not require a full schedule to
/// be present — which makes it suitable for prize money / history distribution
/// where we only want to skip leagues that are actively in-progress.
fn is_league_season_ended(league: &League) -> bool {
    season_has_started(league)
        && !league
            .fixtures
            .iter()
            .any(|f| f.counts_for_league_standings() && f.status == FixtureStatus::Scheduled)
}

pub fn is_league_complete(league: &League) -> bool {
    season_has_started(league)
        && has_full_schedule(league)
        && league
            .fixtures
            .iter()
            .filter(|fixture| fixture.counts_for_league_standings())
            .all(|fixture| fixture.status == FixtureStatus::Completed)
}

/// A competition is complete when all of its fixtures have been played.
/// Guard for whether a competition should be regenerated at rollover. League
/// tables that are mid-season (started but not finished) return false so their
/// fixtures are not wiped during a hemisphere-foreign rollover. Cups and
/// group-knockout competitions are always regenerated — freshly-seeded cups
/// have no fixtures yet and must receive new participants each season.
fn is_competition_complete(competition: &League) -> bool {
    match competition.rules.format {
        CompetitionFormat::LeagueTable => is_league_season_ended(competition),
        _ => true,
    }
}

/// Check if the season is complete for the purposes of allowing rollover.
/// Only the user's own league(s) gate the button; foreign leagues on different
/// hemispheres may still be in progress and are skipped during regeneration.
pub fn is_season_complete(game: &Game) -> bool {
    // Only the user's own league(s) gate the season-complete button. Other
    // leagues (different hemisphere, foreign divisions) may still be in
    // progress without blocking the rollover.
    let user_id = game.manager.team_id.as_deref().unwrap_or_default();
    if !game.competitions.is_empty() {
        let user_leagues: Vec<&League> = game
            .competitions
            .iter()
            .filter(|c| {
                c.rules.format == CompetitionFormat::LeagueTable
                    && c.participant_ids.iter().any(|id| id == user_id)
            })
            .collect();
        if !user_leagues.is_empty() {
            return user_leagues.into_iter().all(is_league_complete);
        }
        // Fallback when user has no known league (e.g. international-only):
        // all league tables must complete before rollover is available.
        let all_leagues: Vec<&League> = game
            .competitions
            .iter()
            .filter(|c| c.rules.format == CompetitionFormat::LeagueTable)
            .collect();
        return !all_leagues.is_empty() && all_leagues.into_iter().all(is_league_season_ended);
    }
    // Legacy single-league path.
    let leagues: Vec<&League> = game.league.iter().collect();
    !leagues.is_empty() && leagues.into_iter().all(is_league_complete)
}

const PRIZE_MONEY_BY_POSITION: [i64; 10] = [
    5_000_000, 3_000_000, 1_500_000, 750_000, 400_000, 300_000, 250_000, 200_000, 175_000, 150_000,
];

const SEASON_PAYOUT_LEDGER_DESCRIPTION_KEY: &str = "be.msg.seasonPayout.ledgerDescription";

fn position_suffix(position: u32) -> &'static str {
    match position {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

fn backend_text_with_params(key: &str, params: [(&str, String); 3]) -> String {
    let mut text = String::from(key);

    for (index, (param_name, param_value)) in params.into_iter().enumerate() {
        text.push(if index == 0 { '?' } else { '&' });
        text.push_str(param_name);
        text.push('=');
        text.push_str(&param_value);
    }

    text
}

fn prize_money_ledger_description(season: u32, position: u32, suffix: &str) -> String {
    backend_text_with_params(
        SEASON_PAYOUT_LEDGER_DESCRIPTION_KEY,
        [
            ("season", season.to_string()),
            ("position", position.to_string()),
            ("suffix", suffix.to_string()),
        ],
    )
}

fn prize_money_for_position(position: u32) -> i64 {
    if position == 0 {
        return 0;
    }

    PRIZE_MONEY_BY_POSITION
        .get(position.saturating_sub(1) as usize)
        .copied()
        .unwrap_or(150_000)
}

/// Prize money for a finishing position, halved for each tier below the top
/// flight (tier 0 = top division).
fn division_prize_money(position: u32, tier: u32) -> i64 {
    prize_money_for_position(position) >> tier
}

/// Final standings and pyramid tier for every league-table competition, falling
/// back to the legacy single league. Tier is the rank by `priority` among the
/// leagues sharing a country; standalone leagues are tier 0.
fn division_standings_with_tiers(game: &Game) -> Vec<(Vec<StandingEntry>, u32)> {
    use std::collections::BTreeMap;

    let leagues: Vec<&League> = if game.competitions.is_empty() {
        game.league.iter().collect()
    } else {
        game.competitions
            .iter()
            .filter(|competition| {
                competition.rules.format == CompetitionFormat::LeagueTable
                    && is_league_season_ended(competition)
            })
            .collect()
    };

    let mut by_country: BTreeMap<&str, Vec<&League>> = BTreeMap::new();
    let mut standalone: Vec<&League> = Vec::new();
    for league in leagues {
        match league.country_id.as_deref() {
            Some(country) => by_country.entry(country).or_default().push(league),
            None => standalone.push(league),
        }
    }

    let mut divisions: Vec<(Vec<StandingEntry>, u32)> = Vec::new();
    for mut group in by_country.into_values() {
        group.sort_by_key(|league| league.priority);
        for (tier, league) in group.into_iter().enumerate() {
            divisions.push((league.sorted_standings(), tier as u32));
        }
    }
    for league in standalone {
        divisions.push((league.sorted_standings(), 0));
    }
    divisions
}

/// Process end-of-season: record history, compute awards, reset stats, generate next season.
/// Returns a summary struct for the frontend to display.
/// Apply promotion/relegation within each domestic pyramid. A pyramid is the
/// set of league-table competitions sharing a country, ordered by `priority`
/// (lowest priority = highest division).
fn apply_pyramid_promotion_relegation(competitions: &mut [League]) {
    use std::collections::BTreeMap;

    let mut tiers_by_country: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, competition) in competitions.iter().enumerate() {
        if competition.rules.format != CompetitionFormat::LeagueTable {
            continue;
        }
        if !is_league_season_ended(competition) {
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

        let mut divisions: Vec<League> = indices
            .iter()
            .map(|&index| competitions[index].clone())
            .collect();
        crate::promotion::apply_promotion_relegation(&mut divisions);

        for (slot, &index) in indices.iter().enumerate() {
            competitions[index].participant_ids = divisions[slot].participant_ids.clone();
        }
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

/// Resolve every berth-fed continental field at once, honouring cross-target
/// exclusivity and the `fallbackTo` cascade: a club ends in the single most
/// prestigious target (lowest priority) it earns, and a berth's `fallbackTo`
/// is a lower-preference target used when the club doesn't earn the primary.
/// Returns `target_id -> field`; targets without incoming berths are absent
/// (the caller keeps the inferred path for those).
pub fn resolve_continental_fields(game: &Game) -> std::collections::HashMap<String, Vec<String>> {
    use std::collections::{HashMap, HashSet};

    // Berth-fed continental targets, most prestigious (lowest priority) first.
    let mut targets: Vec<&League> = game
        .competitions
        .iter()
        .filter(|competition| {
            competition.scope == CompetitionScope::Continental
                && competition_has_incoming_berths(game, &competition.id)
        })
        .collect();
    targets.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
    let priority_of: HashMap<&str, u32> = targets
        .iter()
        .map(|c| (c.id.as_str(), c.priority))
        .collect();

    // Each club keeps the most prestigious target any of its berths award it;
    // a berth's primary target outranks its fallback for the same club.
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
            for winner in evaluate_berth_rule(source, &berth.rule) {
                consider(&winner, &berth.target);
                if let Some(fallback) = &berth.fallback_to {
                    consider(&winner, fallback);
                }
            }
        }
    }

    // Every club placed in any target — excluded from all targets' reputation
    // top-up so a thin field never pulls in a club already qualified elsewhere.
    let all_placed: HashSet<String> = best.keys().cloned().collect();
    let mut raw: HashMap<String, Vec<String>> = HashMap::new();
    for (club, (_prio, target)) in best {
        raw.entry(target).or_default().push(club);
    }

    let mut fields = HashMap::new();
    for target in &targets {
        let qualified = raw.remove(&target.id).unwrap_or_default();
        fields.insert(
            target.id.clone(),
            seed_cap_and_fill(game, target, qualified, all_placed.clone()),
        );
    }
    fields
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

/// Roll every competition over to the next season: apply promotion/relegation
/// to domestic pyramids, regenerate fixtures and standings in place (preserving
/// each competition's identity), and keep the legacy `league` slot in sync.
///
/// `rollover_anchor` is the global trigger point (current date + 28 days). Each
/// competition derives its own next-season start date from its stored
/// `season_start_month`/`season_start_day` fields so that northern and southern
/// hemisphere leagues renew on their respective calendars.
fn regenerate_competitions_for_new_season(
    game: &mut Game,
    _next_season: u32,
    rollover_anchor: DateTime<Utc>,
) {
    // Fall back to the legacy single league when no competition list exists yet.
    if game.competitions.is_empty()
        && let Some(league) = game.league.clone()
    {
        game.competitions.push(league);
    }
    if game.competitions.is_empty() {
        return;
    }

    // Read the World Cup field from this cycle's qualifying before competitions
    // are retired below.
    let kickoff = game.clock.current_date + Duration::days(2);
    let world_cup_due = crate::world_cup::is_world_cup_summer(kickoff.year());
    let qualified_field = if world_cup_due {
        // The club season can finish before the campaign's June dates play
        // out; settle whatever remains (the last matchday, the playoff) so the
        // field is derived from a finished campaign, not a truncated one.
        // Seeded per year so a reloaded save settles to the same field.
        let mut settle_rng = world_cup_rng(kickoff.year());
        crate::world_cup::settle_outstanding_qualifying(game, &mut settle_rng);
        let host = crate::world_cup::host_for_year(game, kickoff.year());
        crate::world_cup::qualified_field_from_game(
            game,
            crate::world_cup::FORMAT_48.field,
            host.as_deref(),
        )
    } else {
        None
    };

    // World Cups and their qualifying are one-shot competitions: retire last
    // cycle's editions instead of regenerating them, and stage new ones when the
    // calendar says so. A full qualifying campaign spans two seasons, so an
    // in-progress campaign (its cup still ahead) survives the intermediate
    // rollover and is re-anchored below.
    let kickoff_year = kickoff.year().max(0) as u32;
    game.competitions.retain(|competition| {
        if !crate::world_cup::is_world_cup_competition(competition) {
            return true;
        }
        (crate::world_cup::is_world_cup_qualifying(competition)
            || crate::world_cup::is_world_cup_playoff(competition))
            && competition.season > kickoff_year
    });
    let remaining_ids: std::collections::HashSet<String> = game
        .competitions
        .iter()
        .map(|competition| competition.id.clone())
        .collect();
    game.active_competition_ids
        .retain(|competition_id| remaining_ids.contains(competition_id));

    // Continental qualification reflects the season just completed: capture each
    // continental field from final domestic standings and cup winners before
    // regeneration resets those standings.
    let berth_fields = resolve_continental_fields(game);
    let continental_entrants: std::collections::HashMap<String, Vec<String>> = game
        .competitions
        .iter()
        .filter(|competition| {
            competition.scope == CompetitionScope::Continental
                && competition.kind == CompetitionType::ContinentalClub
        })
        .map(|competition| {
            // Data-defined berths win when present (resolved together so the
            // cascade and cross-target exclusivity hold); otherwise fall back to
            // the inferred top-of-each-first-division + cup-winners field.
            let entrants = berth_fields
                .get(&competition.id)
                .cloned()
                .unwrap_or_else(|| continental_qualified_entrants(game, competition));
            (competition.id.clone(), entrants)
        })
        .collect();

    apply_pyramid_promotion_relegation(&mut game.competitions);

    // Re-seed continental competitions with this season's qualified entrants
    // before regeneration resets their brackets. Done as a separate pass so
    // cups that haven't started yet (no fixtures) still get new participants
    // even when the completeness guard below would otherwise skip them.
    for competition in game.competitions.iter_mut() {
        if let Some(entrants) = continental_entrants.get(&competition.id)
            && entrants.len() >= 2
        {
            competition.participant_ids = entrants.clone();
        }
    }

    for competition in game.competitions.iter_mut() {
        // A surviving mid-campaign qualifying competition must not be rebuilt
        // as a plain league — its groups and played results carry over; the
        // international-calendar pass below re-anchors its remaining fixtures.
        if crate::world_cup::is_world_cup_competition(competition) {
            continue;
        }
        if !is_competition_complete(competition) {
            // TODO(hemisphere-stall): Mid-season foreign leagues are correctly
            // skipped here to prevent their fixtures from being wiped. However,
            // they will also miss regeneration when *they* finish, because
            // rollover is driven by the user's league rather than by each
            // competition finishing independently. Fix requires a per-competition
            // completion hook separate from the global rollover trigger.
            continue;
        }

        // Each competition starts its next season on its own calendar date so
        // northern and southern hemisphere leagues renew independently.
        let comp_next_start = crate::generator::next_season_start(
            rollover_anchor,
            competition.season_start_month,
            competition.season_start_day,
        );
        let comp_next_season = comp_next_start.year() as u32;

        match competition.rules.format {
            CompetitionFormat::LeagueTable => {
                crate::schedule::regenerate_league_for_season(
                    competition,
                    comp_next_season,
                    comp_next_start,
                );
            }
            CompetitionFormat::GroupAndKnockout => {
                crate::group_stage::regenerate_for_season(
                    competition,
                    comp_next_season,
                    comp_next_start,
                );
            }
            CompetitionFormat::Knockout => {
                crate::schedule::regenerate_knockout_for_season(
                    competition,
                    comp_next_season,
                    comp_next_start,
                );
            }
        }
    }

    manage_international_calendar(
        game,
        rollover_anchor,
        kickoff,
        world_cup_due,
        qualified_field,
    );
    game.sync_legacy_league();
}

/// Decide what the upcoming season's international calendar looks like:
/// - a World Cup summer stages the tournament in the break (no club friendlies);
/// - the season before a World Cup hosts qualifying in the windows;
/// - any other season hosts national-team friendlies in the windows.
fn manage_international_calendar(
    game: &mut Game,
    next_start: DateTime<Utc>,
    kickoff: DateTime<Utc>,
    world_cup_due: bool,
    qualified_field: Option<Vec<String>>,
) {
    if world_cup_due {
        if !game
            .competitions
            .iter()
            .any(crate::world_cup::is_world_cup_competition)
        {
            crate::world_cup::schedule_world_cup_with_field(
                game,
                kickoff,
                &crate::world_cup::FORMAT_48,
                qualified_field,
            );
        }
        // The tournament fills the break; clear any stale window friendlies.
        for national_team in game.national_teams.iter_mut() {
            national_team.fixtures.clear();
        }
        return;
    }

    let window_dates = crate::national_team::international_window_dates(next_start);
    if window_dates.is_empty() {
        return;
    }
    // Qualifying spreads each window's matches across a multi-day block, so club
    // fixtures must keep clear of the whole span rather than just the openers.
    let leads_into_world_cup = crate::world_cup::season_leads_into_world_cup(next_start);
    let starts_qualifying = crate::world_cup::season_starts_world_cup_qualifying(next_start);
    let qualifying_in_progress = game
        .competitions
        .iter()
        .any(crate::world_cup::is_world_cup_qualifying);
    let reserved_dates = if leads_into_world_cup || starts_qualifying || qualifying_in_progress {
        crate::national_team::international_window_span_dates(&window_dates)
    } else {
        window_dates.clone()
    };
    for national_team in game.national_teams.iter_mut() {
        national_team.fixtures.clear();
    }
    for competition in game.competitions.iter_mut() {
        // The World Cup's own competitions live *on* the reserved windows —
        // shifting them off would move the very fixtures the reservation
        // protects. Only club competitions step aside.
        if crate::world_cup::is_world_cup_competition(competition) {
            continue;
        }
        crate::schedule::shift_fixtures_off_reserved_dates(competition, &reserved_dates);
    }
    crate::schedule::append_south_american_preseason_friendlies(
        &mut game.competitions,
        &reserved_dates,
    );
    crate::schedule::append_other_preseason_friendlies(
        &mut game.competitions,
        &reserved_dates,
    );

    if leads_into_world_cup {
        // The windows host the qualifying campaign instead of friendlies: the
        // second half of a two-season campaign continues on this season's
        // windows; a world without one (a save that started late) squeezes a
        // compressed campaign into the single remaining season.
        if qualifying_in_progress {
            crate::world_cup::continue_world_cup_qualifying(
                game,
                &window_dates,
                &mut world_cup_rng(next_start.year()),
            );
        } else {
            crate::world_cup::schedule_world_cup_qualifying(
                game,
                next_start.year() + 1,
                &window_dates,
            );
        }
        return;
    }

    if starts_qualifying {
        // Two summers out: the full home-and-away campaign gets under way.
        // The in-progress guard mirrors the branch above — under the four-year
        // cadence a campaign can't still be running here, but scheduling must
        // never double up if that invariant ever bends.
        if !qualifying_in_progress {
            crate::world_cup::schedule_world_cup_qualifying(
                game,
                next_start.year() + 2,
                &window_dates,
            );
        }
        return;
    }

    if qualifying_in_progress {
        // Unreachable under the four-year cadence, but if a campaign ever
        // survives into a neutral season, keep it anchored to this season's
        // windows rather than stacking friendlies on top of stale fixtures.
        crate::world_cup::continue_world_cup_qualifying(
            game,
            &window_dates,
            &mut world_cup_rng(next_start.year()),
        );
        return;
    }

    crate::national_team::schedule_national_team_friendlies(
        &mut game.national_teams,
        &window_dates,
        &mut rand::rng(),
    );
}

/// A per-year deterministic RNG for settling World Cup fixtures at rollover,
/// so reloading a save and rolling over again reproduces the same field.
fn world_cup_rng(year: i32) -> rand::rngs::StdRng {
    use rand::SeedableRng;
    rand::rngs::StdRng::seed_from_u64(u64::from(year.unsigned_abs()) ^ 0xF1FA)
}

/// The league-table competition the user's club contests. Falls back to the
/// primary competition when the user has no club in any division (e.g. an
/// unemployed manager) or for legacy single-league saves.
pub fn user_division<'a>(game: &'a Game, user_team_id: &str) -> Option<&'a League> {
    game.competitions
        .iter()
        .find(|competition| {
            competition.rules.format == CompetitionFormat::LeagueTable
                && (competition
                    .participant_ids
                    .iter()
                    .any(|id| id == user_team_id)
                    || competition
                        .standings
                        .iter()
                        .any(|standing| standing.team_id == user_team_id))
        })
        .or(game.league.as_ref())
}

/// Send a board message when the user's club starts the new season in a
/// different division of its pyramid (promoted when the new division ranks
/// higher, relegated when it ranks lower).
fn notify_user_division_change(
    game: &mut Game,
    division_before: Option<(String, u32)>,
    user_team_id: &str,
    next_season: u32,
    date: &str,
) {
    let Some((old_division_id, old_priority)) = division_before else {
        return;
    };
    let Some(new_division) = user_division(game, user_team_id) else {
        return;
    };
    if new_division.id == old_division_id {
        return;
    }

    let promoted = new_division.priority < old_priority;
    let division_name = new_division.name.clone();
    let kind = if promoted { "promotion" } else { "relegation" };
    let msg_id = format!("{kind}_{next_season}");
    if game.messages.iter().any(|m| m.id == msg_id) {
        return;
    }

    let mut params = std::collections::HashMap::new();
    params.insert("division".to_string(), division_name);
    params.insert("season".to_string(), next_season.to_string());

    let message = InboxMessage::new(
        msg_id,
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_i18n(
        &format!("be.msg.{kind}.subject"),
        &format!("be.msg.{kind}.body"),
        params,
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");
    game.messages.push(message);
}

pub fn process_end_of_season(game: &mut Game) -> EndOfSeasonSummary {
    // The summary, board review, and manager career must reflect the division
    // the user's club actually contests — not whichever competition happens to
    // be first in the list.
    let user_team_id = game.manager.team_id.clone().unwrap_or_default();
    let league = match user_division(game, &user_team_id) {
        Some(l) => l,
        None => return EndOfSeasonSummary::default(),
    };

    let season = league.season;
    let league_name = league.name.clone();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    // Messages should be dated on the last match day, not on the clock date
    // (which may already be one day ahead due to process_day advancing the clock).
    let last_fixture_date = league
        .fixtures
        .iter()
        .filter(|f| f.counts_for_league_standings() && f.status == FixtureStatus::Completed)
        .map(|f| f.date.as_str())
        .max()
        .unwrap_or(today.as_str())
        .to_string();

    // 1. Compute final standings
    let final_standings = league.sorted_standings();

    // 2. Compute the user's division's awards before resetting stats
    let awards = compute_division_season_awards(game, league);

    // 3. Build summary
    let user_position = final_standings
        .iter()
        .position(|s| s.team_id == user_team_id)
        .map(|i| i + 1)
        .unwrap_or(0) as u32;
    let user_standing = final_standings
        .iter()
        .find(|s| s.team_id == user_team_id)
        .cloned();

    let champion_id = final_standings
        .first()
        .map(|s| s.team_id.clone())
        .unwrap_or_default();
    let champion_name = game
        .teams
        .iter()
        .find(|t| t.id == champion_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    let summary = EndOfSeasonSummary {
        season,
        league_name: league_name.clone(),
        champion_id: champion_id.clone(),
        champion_name,
        user_position,
        user_points: user_standing.as_ref().map(|s| s.points).unwrap_or(0),
        user_won: user_standing.as_ref().map(|s| s.won).unwrap_or(0),
        user_drawn: user_standing.as_ref().map(|s| s.drawn).unwrap_or(0),
        user_lost: user_standing.as_ref().map(|s| s.lost).unwrap_or(0),
        user_goals_for: user_standing.as_ref().map(|s| s.goals_for).unwrap_or(0),
        user_goals_against: user_standing.as_ref().map(|s| s.goals_against).unwrap_or(0),
        golden_boot_player: awards
            .golden_boot
            .first()
            .map(|e| e.player_name.clone())
            .unwrap_or_default(),
        golden_boot_goals: awards
            .golden_boot
            .first()
            .map(|e| e.value as u32)
            .unwrap_or(0),
        poty_player: awards
            .player_of_year
            .first()
            .map(|e| e.player_name.clone())
            .unwrap_or_default(),
        poty_rating: awards
            .player_of_year
            .first()
            .map(|e| e.value)
            .unwrap_or(0.0),
        total_teams: final_standings.len() as u32,
        season_awards: awards.clone(),
    };

    // 4. Record team season history, pay prize money, and update reputation for
    //    every league division — not just the user's competition — so the whole
    //    pyramid crowns champions and keeps records.
    let divisions = division_standings_with_tiers(game);
    let user_division_tier = divisions
        .iter()
        .find(|(standings, _)| standings.iter().any(|s| s.team_id == user_team_id))
        .map(|(_, tier)| *tier)
        .unwrap_or(0);
    for (division_standings, tier) in divisions {
        for (idx, standing) in division_standings.iter().enumerate() {
            if let Some(team) = game.teams.iter_mut().find(|t| t.id == standing.team_id) {
                let position = (idx + 1) as u32;
                let prize_money = division_prize_money(position, tier);

                team.history.push(TeamSeasonRecord {
                    season,
                    league_position: position,
                    played: standing.played,
                    won: standing.won,
                    drawn: standing.drawn,
                    lost: standing.lost,
                    goals_for: standing.goals_for,
                    goals_against: standing.goals_against,
                });
                // Reset form
                team.form.clear();

                if prize_money > 0 {
                    team.finance += prize_money;
                    team.season_income += prize_money;
                    team.financial_ledger.push(FinancialTransaction {
                        date: last_fixture_date.clone(),
                        description: prize_money_ledger_description(
                            season,
                            position,
                            position_suffix(position),
                        ),
                        amount: prize_money,
                        kind: FinancialTransactionKind::PrizeMoney,
                    });
                }

                // Refresh the transfer envelope for the new season. Formula
                // matches worldgen (generator/mod.rs:543): 15% of finance.
                // Since `execute_transfer` debits the budget on every buy and
                // no other path adds to it, without this refill the market
                // would freeze after 2-3 seasons as every club drained to
                // zero.
                // Clamp at zero — unlike worldgen (which only ever sees fresh
                // positive finance), end-of-season runs on live state where a
                // heavily indebted club can have negative `finance`. A
                // negative envelope would still be rejected by
                // `make_transfer_bid`, but showing "€-1.2M transfer budget"
                // in the UI reads worse than a hard zero.
                team.transfer_budget = ((team.finance as f64 * 0.15) as i64).max(0);
            }
        }

        crate::reputation::update_team_reputation(game, &division_standings);
    }

    // 5. Record player career entries and reset stats
    for player in game.players.iter_mut() {
        if player.stats.appearances > 0 {
            let team_name = player
                .team_id
                .as_ref()
                .and_then(|tid| game.teams.iter().find(|t| &t.id == tid))
                .map(|t| t.name.clone())
                .unwrap_or_else(free_agent_team_name);
            let team_id = player.team_id.clone().unwrap_or_default();

            player.career.push(domain::player::CareerEntry {
                season,
                team_id,
                team_name,
                appearances: player.stats.appearances,
                goals: player.stats.goals,
                assists: player.stats.assists,
            });
        }
    }

    crate::aging::apply_seasonal_aging(game, game.clock.current_date.date_naive(), season);

    for player in game.players.iter_mut() {
        // Reset stats for next season
        player.stats = PlayerSeasonStats::default();
    }

    // 5b. Convert retired players to unemployed manager/scout candidates, then
    //     ensure the unemployed pools meet the season-end floor.
    convert_retired_players_to_candidates(game);
    crate::generator::replenish_manager_and_scout_market(game);

    // 6. Update manager career stats
    if let Some(standing) = &user_standing {
        let total_matches = standing.won + standing.drawn + standing.lost;
        game.manager.career_stats.matches_managed += total_matches;
        game.manager.career_stats.wins += standing.won;
        game.manager.career_stats.draws += standing.drawn;
        game.manager.career_stats.losses += standing.lost;
        if user_position == 1 {
            game.manager.career_stats.trophies += 1;
        }
        let best = game.manager.career_stats.best_finish;
        if best.is_none() || best.unwrap() > user_position {
            game.manager.career_stats.best_finish = Some(user_position);
        }
        // Update or create career history entry for current team
        let team_name = game
            .teams
            .iter()
            .find(|t| t.id == user_team_id)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let today_str = game.clock.current_date.format("%Y-%m-%d").to_string();
        // Check if there's an existing open entry for this team
        let existing = game
            .manager
            .career_history
            .iter_mut()
            .find(|e| e.team_id == user_team_id && e.end_date.is_none());
        if let Some(entry) = existing {
            entry.matches += total_matches;
            entry.wins += standing.won;
            entry.draws += standing.drawn;
            entry.losses += standing.lost;
            let prev_best = entry.best_league_position;
            if prev_best.is_none() || prev_best.unwrap() > user_position {
                entry.best_league_position = Some(user_position);
            }
        } else {
            game.manager
                .career_history
                .push(domain::manager::ManagerCareerEntry {
                    team_id: user_team_id.clone(),
                    team_name,
                    start_date: today_str,
                    end_date: None,
                    matches: total_matches,
                    wins: standing.won,
                    draws: standing.drawn,
                    losses: standing.lost,
                    best_league_position: Some(user_position),
                });
        }
    }

    // 6b. Evaluate board objectives and adjust satisfaction
    let obj_delta = crate::board_objectives::evaluate_objectives(game);
    let new_sat = (game.manager.satisfaction as i16 + obj_delta as i16).clamp(0, 100) as u8;
    game.manager.satisfaction = new_sat;
    // Clear objectives for next season (will be regenerated on first process_day)
    game.board_objectives.clear();

    // 6c. Clear old news articles from the previous season
    game.news.clear();

    // 6d. Publish the season awards ceremony article (skipped when no marquee winners)
    if let Some(article) = crate::news::season_awards_article(&awards, season, &last_fixture_date) {
        game.news.push(article);
    }

    // 7. Roll every competition over to the next season, applying domestic
    //    promotion/relegation, and keep the legacy `league` slot in sync.
    //    Each competition computes its own next-season start from its stored
    //    season_start_month; `rollover_anchor` is just the global trigger point.
    let next_season = season + 1;
    let rollover_anchor = game.clock.current_date + Duration::days(28);
    let user_division_before =
        user_division(game, &user_team_id).map(|division| (division.id.clone(), division.priority));
    regenerate_competitions_for_new_season(game, next_season, rollover_anchor);
    notify_user_division_change(
        game,
        user_division_before,
        &user_team_id,
        next_season,
        &last_fixture_date,
    );

    let preview_date = game.clock.current_date.to_rfc3339();
    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(crate::news::season_preview_article(
        &team_names,
        &preview_date,
    ));

    // 8. Send end-of-season messages
    let pos_suffix = position_suffix(user_position);

    let user_team_name = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    let existing_ids: std::collections::HashSet<String> =
        game.messages.iter().map(|m| m.id.clone()).collect();

    let payout_msg_id = format!("season_payout_{}", season);
    let user_prize_money = division_prize_money(user_position, user_division_tier);
    if user_prize_money > 0 && !existing_ids.contains(&payout_msg_id) {
        let payout_message = InboxMessage::new(
            payout_msg_id,
            String::new(),
            String::new(),
            String::new(),
            last_fixture_date.clone(),
        )
        .with_category(MessageCategory::Finance)
        .with_priority(MessagePriority::High)
        .with_sender_role("")
        .with_i18n("be.msg.seasonPayout.subject", "be.msg.seasonPayout.body", {
            let mut params = std::collections::HashMap::new();
            params.insert("season".to_string(), season.to_string());
            params.insert("amount".to_string(), user_prize_money.to_string());
            params.insert("position".to_string(), user_position.to_string());
            params
        })
        .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");
        game.messages.push(payout_message);
    }

    let msg_id = format!("season_end_{}", season);
    if !existing_ids.contains(&msg_id) {
        let (body_key, mut i18n_params) = if user_position == 1 {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.champion", p)
        } else if user_position <= 4 {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("position".to_string(), user_position.to_string());
            p.insert("suffix".to_string(), pos_suffix.to_string());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.topFour", p)
        } else if user_position <= summary.total_teams / 2 {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("position".to_string(), user_position.to_string());
            p.insert("suffix".to_string(), pos_suffix.to_string());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.midTable", p)
        } else {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("position".to_string(), user_position.to_string());
            p.insert("suffix".to_string(), pos_suffix.to_string());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.lowerHalf", p)
        };
        i18n_params.insert("season".to_string(), season.to_string());

        let msg = InboxMessage::new(
            msg_id,
            String::new(),
            String::new(),
            String::new(),
            last_fixture_date.clone(),
        )
        .with_category(MessageCategory::BoardDirective)
        .with_priority(MessagePriority::High)
        .with_sender_role("")
        .with_i18n("be.msg.seasonReview.subject", body_key, i18n_params)
        .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");
        game.messages.push(msg);
    }

    let sched_msg_id = format!("new_season_{}", next_season);
    if !existing_ids.contains(&sched_msg_id) {
        let mut sched_params = std::collections::HashMap::new();
        sched_params.insert("season".to_string(), next_season.to_string());
        let sched_msg = InboxMessage::new(
            sched_msg_id,
            String::new(),
            String::new(),
            String::new(),
            last_fixture_date,
        )
        .with_category(MessageCategory::LeagueInfo)
        .with_priority(MessagePriority::Normal)
        .with_sender_role("")
        .with_i18n(
            "be.msg.newSeasonSchedule.subject",
            "be.msg.newSeasonSchedule.body",
            sched_params,
        )
        .with_sender_i18n("be.sender.leagueOffice", "be.role.competitionSecretary");
        game.messages.push(sched_msg);
    }

    crate::season_context::refresh_game_context(game);

    summary
}

// ---------------------------------------------------------------------------
// Retiree conversion
// ---------------------------------------------------------------------------

/// For every retired, unattached player with at least one career entry:
/// * Creates an unemployed manager candidate with deterministic ID `mgr_retired_{player_id}`.
/// * Creates an unattached scout with deterministic ID `staff_retired_scout_{player_id}`.
///
/// Both checks are idempotent — if the ID already exists the entry is not duplicated.
fn convert_retired_players_to_candidates(game: &mut Game) {
    // Snapshot eligible players (avoid holding borrows across mutations).
    struct RetiredSnapshot {
        player_id: String,
        first_name: String,
        last_name: String,
        date_of_birth: String,
        nationality: String,
        ovr: u8,
        career_len: usize,
        vision: u8,
        decisions: u8,
        positioning: u8,
        teamwork: u8,
        leadership: u8,
    }

    let retirees: Vec<RetiredSnapshot> = game
        .players
        .iter()
        .filter(|p| p.retired && p.team_id.is_none() && !p.career.is_empty())
        .map(|p| {
            let mut name_parts = p.full_name.splitn(2, ' ');
            let first_name = name_parts.next().unwrap_or(&p.full_name).to_string();
            let last_name = name_parts.next().unwrap_or("").to_string();
            RetiredSnapshot {
                player_id: p.id.clone(),
                first_name,
                last_name,
                date_of_birth: p.date_of_birth.clone(),
                nationality: p.nationality.clone(),
                ovr: p.ovr,
                career_len: p.career.len(),
                vision: p.attributes.vision,
                decisions: p.attributes.decisions,
                positioning: p.attributes.positioning,
                teamwork: p.attributes.teamwork,
                leadership: p.attributes.leadership,
            }
        })
        .collect();

    if retirees.is_empty() {
        return;
    }

    let existing_mgr_ids: std::collections::HashSet<String> =
        game.managers.iter().map(|m| m.id.clone()).collect();
    let existing_staff_ids: std::collections::HashSet<String> =
        game.staff.iter().map(|s| s.id.clone()).collect();

    let mut new_managers: Vec<Manager> = Vec::new();
    let mut new_scouts: Vec<Staff> = Vec::new();

    for r in &retirees {
        // Manager candidate
        let mgr_id = format!("mgr_retired_{}", r.player_id);
        if !existing_mgr_ids.contains(&mgr_id) {
            let reputation = (200u32)
                .saturating_add((r.ovr as u32) * 6)
                .saturating_add(r.career_len as u32 * 30)
                .clamp(200, 900);

            let mut mgr = Manager::new(
                mgr_id,
                r.first_name.clone(),
                r.last_name.clone(),
                r.date_of_birth.clone(),
                r.nationality.clone(),
            );
            mgr.reputation = reputation;
            mgr.satisfaction = 50;
            mgr.fan_approval = 50;
            // team_id stays None (unemployed)
            new_managers.push(mgr);
        }

        // Scout candidate
        let scout_id = format!("staff_retired_scout_{}", r.player_id);
        if !existing_staff_ids.contains(&scout_id) {
            let judging_ability = ((r.vision as u16 + r.decisions as u16) / 2).min(100) as u8;
            let judging_potential = ((r.positioning as u16 + r.teamwork as u16) / 2).min(100) as u8;
            let coaching = (r.leadership / 2).max(10);

            let mut scout = Staff::new(
                scout_id,
                r.first_name.clone(),
                r.last_name.clone(),
                r.date_of_birth.clone(),
                StaffRole::Scout,
                StaffAttributes {
                    coaching,
                    judging_ability,
                    judging_potential,
                    physiotherapy: 10,
                },
            );
            scout.nationality = r.nationality.clone();
            // team_id stays None (unattached market candidate)
            new_scouts.push(scout);
        }
    }

    game.managers.extend(new_managers);
    game.staff.extend(new_scouts);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct EndOfSeasonSummary {
    pub season: u32,
    pub league_name: String,
    pub champion_id: String,
    pub champion_name: String,
    pub user_position: u32,
    pub user_points: u32,
    pub user_won: u32,
    pub user_drawn: u32,
    pub user_lost: u32,
    pub user_goals_for: u32,
    pub user_goals_against: u32,
    pub golden_boot_player: String,
    pub golden_boot_goals: u32,
    pub poty_player: String,
    pub poty_rating: f64,
    pub total_teams: u32,
    pub season_awards: crate::season_awards::SeasonAwards,
}
