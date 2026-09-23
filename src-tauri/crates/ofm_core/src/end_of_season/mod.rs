use crate::game::Game;
use crate::season_awards::compute_division_season_awards;
use chrono::{DateTime, Datelike, Duration, Utc};
use domain::league::{
    CompetitionFormat, CompetitionScope, CompetitionType, FixtureStatus, League, StandingEntry,
};
use domain::manager::Manager;
use domain::message::*;
use domain::player::PlayerSeasonStats;
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::{FinancialTransaction, FinancialTransactionKind, TeamSeasonRecord};

mod berths;
use berths::{
    apply_domestic_berth_promotion_relegation, apply_pyramid_promotion_relegation,
    resolve_domestic_berth_fields,
};
pub use berths::{
    berth_qualified_entrants, competition_has_incoming_berths, continental_qualified_entrants,
    resolve_continental_fields,
};

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
            if !user_leagues.iter().copied().all(is_league_complete) {
                return false;
            }
            // The rest of the user's own pyramid has to be finished too, or the
            // rollover fires while a division below is still playing and the
            // ladder skips the whole country. Brazil is the shipped case: its
            // first division starts in January and its second in March, so at
            // the first division's last matchday the second still had eight
            // rounds to go. Promotion happened every *other* season, and the
            // second division skipped a calendar year each time.
            //
            // Waiting costs nothing on the calendar — the first division's next
            // season still starts on its own date in January. Only tiers that
            // have actually kicked off can block, so a division whose fixtures
            // were never generated cannot strand a career.
            let user_countries: std::collections::BTreeSet<&str> = user_leagues
                .iter()
                .filter_map(|league| league.country_id.as_deref())
                .collect();
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            let countrymen_still_playing = game.competitions.iter().any(|competition| {
                berths::is_ladder_tier(competition)
                    && competition
                        .country_id
                        .as_deref()
                        .is_some_and(|country| user_countries.contains(country))
                    && season_has_started(competition)
                    && !is_league_season_ended(competition)
                    && has_a_fixture_still_to_come(competition, &today)
            });
            return !countrymen_still_playing;
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

/// True when `competition` still has a match the day loop can reach.
///
/// A fixture is played on the day it is dated, and the loop never looks back,
/// so one the clock has already passed is unreachable and its competition will
/// never report its season ended. Such a tier is stale rather than playing, and
/// waiting on it would mean the season could never complete at all: the
/// rollover would refuse forever and the end-of-season screen would never
/// appear — the very failure the country-wide wait exists to prevent.
fn has_a_fixture_still_to_come(competition: &League, today: &str) -> bool {
    competition.fixtures.iter().any(|fixture| {
        fixture.counts_for_league_standings()
            && fixture.status == FixtureStatus::Scheduled
            && fixture.date.as_str() >= today
    })
}

/// Reduce a country's tables to one per division.
///
/// A split-season country plays the same clubs through an Apertura and a
/// Clausura, which are two competitions over one division. Counted separately
/// they were ranked as though the second were a tier below the first, so an
/// Argentine club banked a top-flight prize for one half and a second-division
/// prize for the other — 7,500,000 for a single year — and its career history
/// gained two entries every season.
///
/// The half that finishes last is the one kept: it is the table the club ends
/// its year on. `group` is already ordered by rank, and that order is preserved.
fn collapse_repeated_divisions(group: &mut Vec<&League>) {
    use std::collections::BTreeSet;

    fn roster(league: &League) -> BTreeSet<&str> {
        league.participant_ids.iter().map(String::as_str).collect()
    }
    fn finished_on(league: &League) -> Option<&str> {
        league
            .fixtures
            .iter()
            .filter(|fixture| fixture.status == FixtureStatus::Completed)
            .map(|fixture| fixture.date.as_str())
            .max()
    }

    let mut kept: Vec<&League> = Vec::with_capacity(group.len());
    for league in group.iter().copied() {
        match kept
            .iter()
            .position(|other| !roster(other).is_empty() && roster(other) == roster(league))
        {
            Some(index) if finished_on(league) > finished_on(kept[index]) => kept[index] = league,
            Some(_) => {}
            None => kept.push(league),
        }
    }
    *group = kept;
}

/// A division whose season has just been played out: its final table, how far
/// down its own pyramid it sits, and the season it belongs to.
struct FinishedDivision {
    standings: Vec<StandingEntry>,
    tier: u32,
    season: u32,
}

/// Final standings for every league-table competition that has finished,
/// falling back to the legacy single league.
///
/// Tier is the rank by `priority` among *all* the leagues sharing a country,
/// not just the finished ones. Ranking the finished ones alone made a second
/// division the top flight whenever the first was still playing, and prize
/// money is halved per tier — so its champion banked a top-flight 5,000,000
/// instead of 2,500,000.
///
/// Each division also carries its own season. The caller used to stamp every
/// record with the *user's* season, which mislabels a foreign league running on
/// another calendar: its 2034 results were recorded as 2035.
fn division_standings_with_tiers(game: &Game) -> Vec<FinishedDivision> {
    use std::collections::BTreeMap;

    if game.competitions.is_empty() {
        return game
            .league
            .iter()
            .map(|league| FinishedDivision {
                standings: league.sorted_standings(),
                tier: 0,
                season: league.season,
            })
            .collect();
    }

    let mut by_country: BTreeMap<&str, Vec<&League>> = BTreeMap::new();
    let mut standalone: Vec<&League> = Vec::new();
    // The same predicate the ladder admits on. A cup or a regional side
    // competition can be scored as a table and share a country, but it is not a
    // rung of the pyramid: ranked as one it takes a tier number, and because it
    // often shares a roster with the real division it could win the collapse
    // below and hand that division's prize money and history to whoever
    // happened to win it.
    for league in game
        .competitions
        .iter()
        .filter(|competition| berths::is_ladder_tier(competition))
    {
        match league.country_id.as_deref() {
            Some(country) => by_country.entry(country).or_default().push(league),
            None => standalone.push(league),
        }
    }

    let mut divisions: Vec<FinishedDivision> = Vec::new();
    for mut group in by_country.into_values() {
        group.sort_by_key(|league| league.priority);
        collapse_repeated_divisions(&mut group);
        for (tier, league) in group.into_iter().enumerate() {
            if !is_league_season_ended(league) {
                continue; // ranked, but nothing to pay out or record yet
            }
            divisions.push(FinishedDivision {
                standings: league.sorted_standings(),
                tier: tier as u32,
                season: league.season,
            });
        }
    }
    for league in standalone {
        if !is_league_season_ended(league) {
            continue;
        }
        divisions.push(FinishedDivision {
            standings: league.sorted_standings(),
            tier: 0,
            season: league.season,
        });
    }
    divisions
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
    let domestic_berth_fields = resolve_domestic_berth_fields(game);
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
    apply_domestic_berth_promotion_relegation(game, &domestic_berth_fields);

    // Re-seed continental competitions with this season's qualified entrants
    // before regeneration resets their brackets. Done as a separate pass so
    // cups that haven't started yet (no fixtures) still get new participants
    // even when the completeness guard below would otherwise skip them.
    //
    // A competition that is mid-season is the one case that must be left
    // alone: it keeps its own fixtures and standings, so handing it next
    // season's field leaves a table scoring clubs that are no longer in it and
    // fixtures between clubs it no longer lists.
    for competition in game.competitions.iter_mut() {
        if let Some(entrants) = continental_entrants.get(&competition.id)
            && entrants.len() >= 2
            && (competition.fixtures.is_empty() || is_competition_complete(competition))
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
        // Never behind the season just played. The next season is normally the
        // calendar year of the competition's own next start date, but a save
        // whose clock and competition seasons disagree could otherwise regress
        // a competition stamped 2030 back to 2026 and replay years of history.
        let comp_next_season = (comp_next_start.year() as u32).max(competition.season + 1);

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
    refresh_user_competition_scope(game);
    game.sync_legacy_league();
}

/// Put the user's competitions back in scope after the ladder has moved their
/// club between divisions.
///
/// `active_competition_ids` is the simulation scope chosen when the career
/// started, and an empty list means no filter at all. Rollover already drops
/// the ids of retired competitions, but nothing ever added the division a
/// promoted or relegated club now plays in. The day loop skips competitions
/// out of scope, so it could not see the user's own fixtures, fell through to
/// the legacy `game.league` mirror, and ran their match against whichever
/// competition happened to sort first — an English manager relegated to the
/// second division was sent to an Argentine fixture.
///
/// `resolve_simulation_scope` applies this same rule when the career starts;
/// this keeps it true for the rest of it.
fn refresh_user_competition_scope(game: &mut Game) {
    if game.active_competition_ids.is_empty() {
        return;
    }
    let Some(team_id) = game.manager.team_id.clone() else {
        return;
    };
    let joined: Vec<String> = game
        .competitions
        .iter()
        .filter(|competition| {
            competition.participant_ids.contains(&team_id)
                && !game.active_competition_ids.contains(&competition.id)
        })
        .map(|competition| competition.id.clone())
        .collect();
    game.active_competition_ids.extend(joined);
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
    crate::schedule::append_other_preseason_friendlies(&mut game.competitions, &reserved_dates);

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

    // Rank decides direction, but two divisions can share a rank — a berth
    // moves clubs into its target, and nothing makes a target outrank its
    // feeder. Treat a move along a berth as the promotion it is, rather than
    // telling a champion they have been relegated.
    let new_division_id = new_division.id.clone();
    let promoted_by_berth = game
        .competitions
        .iter()
        .find(|competition| competition.id == old_division_id)
        .is_some_and(|old_division| {
            old_division.berths.iter().any(|berth| {
                berth.target == new_division_id
                    && matches!(berth.rule, domain::league::BerthRule::PositionRange { .. })
            })
        });
    let promoted = new_division.priority < old_priority || promoted_by_berth;
    let division_name = new_division.name.clone();
    let kind = if promoted { "promotion" } else { "relegation" };
    let msg_id = format!("{kind}_{next_season}");
    if crate::inbox::already_emitted(game, &msg_id) {
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
    crate::inbox::emit(game, message);
}

/// Process end-of-season: record history, compute awards, reset stats, generate next season.
/// Returns a summary struct for the frontend to display.
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
        .find(|division| {
            division
                .standings
                .iter()
                .any(|entry| entry.team_id == user_team_id)
        })
        .map(|division| division.tier)
        .unwrap_or(0);
    let mut user_prize_posted = false;
    for division in divisions {
        let (division_standings, tier, season) =
            (division.standings, division.tier, division.season);
        for (idx, standing) in division_standings.iter().enumerate() {
            let position = (idx + 1) as u32;
            let prize_money = division_prize_money(position, tier);
            let team_id = standing.team_id.clone();
            let prize_posted = if prize_money > 0 {
                let date = chrono::NaiveDate::parse_from_str(&last_fixture_date, "%Y-%m-%d")
                    .unwrap_or_else(|_| game.clock.current_date.date_naive());
                match crate::finances::post(
                    game,
                    &team_id,
                    prize_money,
                    crate::finances::CashKind::PrizeMoney,
                    date,
                ) {
                    Ok(_) => true,
                    Err(err) => {
                        log::error!("end-of-season prize post failed for {team_id}: {err}");
                        false
                    }
                }
            } else {
                false
            };
            if prize_posted && team_id == user_team_id {
                user_prize_posted = true;
            }
            if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
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
                team.form.clear();

                if prize_posted {
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

    // Each check reads the live ledger rather than a snapshot: the three emits
    // below are interleaved with the checks, so a snapshot would go stale.
    let payout_msg_id = format!("season_payout_{}", season);
    let user_prize_money = division_prize_money(user_position, user_division_tier);
    if user_prize_posted && !crate::inbox::already_emitted(game, &payout_msg_id) {
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
        crate::inbox::emit(game, payout_message);
    }

    let msg_id = format!("season_end_{}", season);
    if !crate::inbox::already_emitted(game, &msg_id) {
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
        crate::inbox::emit(game, msg);
    }

    let sched_msg_id = format!("new_season_{}", next_season);
    if !crate::inbox::already_emitted(game, &sched_msg_id) {
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
        crate::inbox::emit(game, sched_msg);
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
