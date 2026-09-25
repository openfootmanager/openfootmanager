use crate::game::Game;
use crate::player_rating::{
    effective_rating_for_assignment, formation_slots, natural_ovr, positional_fit_for_assignment,
};
use domain::player::Position as DomainPosition;
use engine::{
    BreakSpeed, CounterPressDuration, DefensiveLine, DefensiveShape, MarkingStyle, PlayStyle,
    PlayerData, PlayerRole as EnginePlayerRole, Position, PressingIntensity, TacticsBuildUpStyle,
    TacticsConfig, TacticsPitchWidth, TeamData, Tempo,
};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Domain → Engine conversion with starting XI / bench split
// ---------------------------------------------------------------------------

pub(crate) fn build_team_with_bench(game: &Game, team_id: &str) -> (TeamData, Vec<PlayerData>) {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (name, formation, play_style, tactics, saved_xi_ids) = match team {
        Some(t) => (
            t.name.clone(),
            t.formation.clone(),
            match t.play_style {
                domain::team::PlayStyle::Attacking => PlayStyle::Attacking,
                domain::team::PlayStyle::Defensive => PlayStyle::Defensive,
                domain::team::PlayStyle::Possession => PlayStyle::Possession,
                domain::team::PlayStyle::Counter => PlayStyle::Counter,
                domain::team::PlayStyle::HighPress => PlayStyle::HighPress,
                _ => PlayStyle::Balanced,
            },
            domain_to_engine_tactics(&t.tactics_phase),
            t.starting_xi_ids.as_slice(),
        ),
        None => (
            "Unknown".into(),
            "4-4-2".into(),
            PlayStyle::Balanced,
            TacticsConfig::default(),
            &[] as &[String],
        ),
    };

    // Collect all available (non-injured) players for this team
    let available_players: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.injury.is_none())
        .collect();
    let player_roles = team.map(|t| &t.player_roles);
    // `deployed` is the granular slot the player occupies; `None` for the bench,
    // where the player's own position is used instead. The engine's coarse
    // position is derived from this so a player fielded out of position (e.g. a
    // striker at centre-back) is simulated in the position they actually play.
    let convert_player = |p: &domain::player::Player, deployed: Option<&DomainPosition>| {
        let role = player_roles
            .and_then(|roles| roles.get(&p.id))
            .map(domain_to_engine_role)
            .unwrap_or(EnginePlayerRole::Standard);
        to_engine_player(p, role, deployed)
    };

    // The user manages their own XI by hand (saved_xi_ids); AI clubs are managed
    // by a reputation-driven policy that picks a first-choice XI and rotates for
    // load management. Gate on the user team explicitly, NOT on "saved XI empty",
    // so the human's early-career auto-built XI stays reputation-independent.
    let is_user_team = game.manager.team_id.as_deref() == Some(team_id);
    let mut starting_players = if is_user_team {
        select_starting_xi(saved_xi_ids, &available_players, &formation)
    } else {
        let quality = team_management_quality(game, team);
        // The same club must name the same side however often this is called for
        // one fixture, so the manager's misjudgements are seeded from the club and
        // the date rather than rolled fresh. A new matchday is a new judgement.
        let seed = selection_seed(
            team_id,
            &game.clock.current_date.format("%Y-%m-%d").to_string(),
        );
        let load = fixture_load(game, team_id);
        ai_select_starting_xi(&available_players, &formation, quality, seed, load)
    };
    // Both select_starting_xi and ai_select_starting_xi return a slot-aligned XI
    // (entry i plays formation slot i), so the list index is the deployed slot.
    let slots = formation_slots(&formation);
    fill_from_the_treatment_room(game, team_id, &slots, &mut starting_players);
    let used_ids: HashSet<String> = starting_players
        .iter()
        .map(|player| player.id.clone())
        .collect();
    let starting_xi = starting_players
        .into_iter()
        .enumerate()
        .map(|(slot_index, p)| convert_player(p, slots.get(slot_index)))
        .collect();

    let mut bench_domain: Vec<&domain::player::Player> = available_players
        .into_iter()
        .filter(|player| !used_ids.contains(&player.id))
        .collect();
    bench_domain.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let bench = bench_domain
        .into_iter()
        .map(|p| convert_player(p, None))
        .collect();

    let team_data = TeamData {
        id: team_id.to_string(),
        name,
        formation,
        play_style,
        players: starting_xi,
        tactics,
    };

    (team_data, bench)
}

/// Make a short XI up to the number of slots the formation asks for, drawing on
/// players who are carrying an injury.
///
/// Selection proper only ever considers fit players, so this runs on what it
/// leaves behind: it can add nobody a manager would have picked anyway. It
/// exists because a side has to be put out. Handed a side with nobody in it the
/// engine indexes a player who is not there and brings the whole day down; and
/// handed a short one it never counts the missing men, since an absent position
/// group falls back to a fixed rating and absent roles borrow whoever is left.
/// So neither fielding nobody nor fielding four is a thing the simulation can
/// be trusted to punish. A club that cannot name eleven fit players plays its
/// walking wounded instead, least serious knock first.
///
/// A club with no registered players at all still comes back empty. That is a
/// broken save rather than an injury crisis, and papering over it here would
/// only hide it.
fn fill_from_the_treatment_room<'a>(
    game: &'a Game,
    team_id: &str,
    slots: &[DomainPosition],
    starting_players: &mut Vec<&'a domain::player::Player>,
) {
    let wanted = slots.len().min(11);
    if starting_players.len() >= wanted {
        return;
    }

    let mut used: HashSet<&str> = starting_players.iter().map(|p| p.id.as_str()).collect();
    let already_named = starting_players.len();
    for slot in slots.iter().take(wanted).skip(already_named) {
        let best = game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team_id))
            .filter(|p| p.injury.is_some() && !used.contains(p.id.as_str()))
            .min_by(|left, right| {
                let days = |p: &domain::player::Player| {
                    p.injury.as_ref().map_or(0, |injury| injury.days_remaining)
                };
                days(left).cmp(&days(right)).then_with(|| {
                    effective_rating_for_assignment(right, slot)
                        .partial_cmp(&effective_rating_for_assignment(left, slot))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            });

        let Some(player) = best else {
            break; // The treatment room is empty too — field what we have.
        };
        used.insert(player.id.as_str());
        starting_players.push(player);
    }

    reseat_by_position(slots, starting_players);
}

/// Put a topped-up eleven back into the slots its players actually fit.
///
/// Selection filled the formation from the front with the fit players it had,
/// so the vacancies a top-up fills are whatever slots were left over at the
/// back — in a 4-4-2, up front. Appending there would play an injured keeper at
/// centre-forward with an outfielder in goal in his place. Who plays is already
/// decided; this only decides where, slot by slot, by the same condition-free
/// fit the AI's first-choice eleven is picked on.
fn reseat_by_position(
    slots: &[DomainPosition],
    starting_players: &mut Vec<&domain::player::Player>,
) {
    let mut unseated = std::mem::take(starting_players);
    for slot in slots.iter().take(unseated.len()) {
        let best = unseated
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                positional_fit_for_assignment(left, slot)
                    .partial_cmp(&positional_fit_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index);
        if let Some(index) = best {
            starting_players.push(unseated.remove(index));
        }
    }
}

fn select_starting_xi<'a>(
    saved_xi_ids: &[String],
    available_players: &[&'a domain::player::Player],
    formation: &str,
) -> Vec<&'a domain::player::Player> {
    let players_by_id: HashMap<&str, &domain::player::Player> = available_players
        .iter()
        .map(|player| (player.id.as_str(), *player))
        .collect();

    // Count the distinct saved starters that are still available.
    let mut seen_saved = HashSet::new();
    let valid_saved = saved_xi_ids
        .iter()
        .filter(|id| players_by_id.contains_key(id.as_str()) && seen_saved.insert((*id).clone()))
        .count();

    // Too few of the saved XI remain valid — rebuild a fresh, slot-aligned XI.
    if valid_saved < 8 {
        return auto_select_starting_xi(available_players, formation);
    }

    let slots = formation_slots(formation);
    let slot_count = slots.len().min(11);
    let mut chosen: Vec<Option<&domain::player::Player>> = vec![None; slot_count];
    let mut used_ids: HashSet<String> = HashSet::new();

    // Pass 1: keep each available saved starter at the slot it was saved in, so
    // the result is indexed by slot (chosen[i] plays formation slot i).
    for (slot_index, chosen_slot) in chosen.iter_mut().enumerate() {
        if let Some(player) = saved_xi_ids
            .get(slot_index)
            .and_then(|id| players_by_id.get(id.as_str()))
            && used_ids.insert(player.id.clone())
        {
            *chosen_slot = Some(*player);
        }
    }

    // Pass 2: fill any slot vacated by an unavailable saved starter (e.g. an
    // injured goalkeeper) with the best-fit remaining player FOR THAT SLOT, so
    // the lineup never loses a position and stays slot-aligned.
    for (slot_index, chosen_slot) in chosen.iter_mut().enumerate() {
        if chosen_slot.is_some() {
            continue;
        }
        let slot = &slots[slot_index];
        let best = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            .max_by(|left, right| {
                effective_rating_for_assignment(left, slot)
                    .partial_cmp(&effective_rating_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some(player) = best {
            used_ids.insert(player.id.clone());
            *chosen_slot = Some(player);
        }
    }

    // If a slot could not be filled (fewer available players than slots), fall
    // back to a contiguous slot-aligned selection. flatten()ing a list with a
    // gap would shift later starters into earlier slots and break the caller's
    // index->slot mapping.
    if chosen.iter().any(Option::is_none) {
        return auto_select_starting_xi(available_players, formation);
    }

    chosen.into_iter().map(Option::unwrap).collect()
}

fn auto_select_starting_xi<'a>(
    available_players: &[&'a domain::player::Player],
    formation: &str,
) -> Vec<&'a domain::player::Player> {
    let slots = formation_slots(formation);
    let mut used_ids = HashSet::new();
    let mut starting_xi = Vec::with_capacity(11);

    for slot in slots.iter().take(11) {
        let best_player = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            .max_by(|left, right| {
                effective_rating_for_assignment(left, slot)
                    .partial_cmp(&effective_rating_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(player) = best_player else {
            break;
        };

        used_ids.insert(player.id.clone());
        starting_xi.push(player);
    }

    starting_xi
}

/// Maps a club's reputation to a 0.0–1.0 management-quality score. Generated club
/// reputations span roughly 300 (lower divisions) to 900 (elite). Quality drives
/// how proactively the AI rotates for player freshness.
fn management_quality(reputation: u32) -> f64 {
    (((reputation as f64) - 300.0) / 600.0).clamp(0.0, 1.0)
}

/// Maps a manager's overall rating (≈30–95) to a 0.0–1.0 management-quality score.
fn management_quality_from_rating(rating: u8) -> f64 {
    ((f64::from(rating) - 30.0) / 65.0).clamp(0.0, 1.0)
}

/// Resolves the AI management quality for a team: the linked manager's rating when
/// one is hired (manager-specific skill), otherwise the club's reputation as a
/// proxy. Defaults to a mid value when the team can't be found.
fn team_management_quality(game: &Game, team: Option<&domain::team::Team>) -> f64 {
    let Some(team) = team else {
        return management_quality(500);
    };

    if let Some(manager_id) = &team.manager_id
        && let Some(manager) = game.managers.iter().find(|m| &m.id == manager_id)
    {
        return management_quality_from_rating(manager.rating());
    }

    management_quality(team.reputation)
}

/// FNV-1a, hand-rolled rather than reached for from the standard library.
/// `DefaultHasher`'s output is explicitly not promised to stay the same across
/// Rust releases, and an AI team sheet that changed when the toolchain moved
/// would make a saved season impossible to reproduce.
fn stable_hash(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325 ^ seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Whether this club plays again soon after the match it is picking a side for.
///
/// This is what makes resting a merely tired player worth fielding a weaker one.
/// On a normal week the next match is seven days off and a tired first eleven
/// recovers by then; on a congested run it is three or four, and it does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FixtureLoad {
    Normal,
    Congested,
}

/// A club that plays again within this many days of a match is on a congested
/// run. Midweek to weekend is three or four days; weekend to weekend is seven.
///
/// Deliberately not training's congestion rule ("two fixtures in the coming
/// week"). That is counted the morning *after* a match, when the day's fixture
/// has already been played; asked at kick-off it would count today's match as
/// well, and every ordinary weekend-to-weekend season would read as congested.
const CONGESTED_WITHIN_DAYS: i64 = 4;

/// See [`FixtureLoad`]. Compares ISO dates as strings — they sort and match
/// correctly that way — so nothing is parsed per fixture: this runs twice for
/// every match in the world, and a populated world holds tens of thousands of
/// fixtures.
pub(crate) fn fixture_load(game: &Game, team_id: &str) -> FixtureLoad {
    use domain::league::FixtureStatus;

    let today = game.clock.current_date;
    let soon: Vec<String> = (1..=CONGESTED_WITHIN_DAYS)
        .map(|days| {
            (today + chrono::Duration::days(days))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect();
    // Both collections, not the usual "competitions, else the legacy slot". A
    // side is picked *during* simulation, and `simulate_competition_day_with_capture`
    // moves the competition being played out of `game.competitions` into
    // `game.league` for the length of it, leaving an empty default behind. Reading
    // `competitions` alone would miss that competition's own next round — a
    // league's midweek fixture — which is the commonest congested run there is.
    // Outside simulation the legacy slot mirrors one of the competitions, and a
    // fixture seen twice cannot change an `any`.
    let plays_again_soon = game
        .competitions
        .iter()
        .chain(game.league.iter())
        .flat_map(|competition| competition.fixtures.iter())
        .filter(|fixture| fixture.status == FixtureStatus::Scheduled)
        .filter(|fixture| soon.contains(&fixture.date))
        .any(|fixture| fixture.home_team_id == team_id || fixture.away_team_id == team_id);

    if plays_again_soon {
        FixtureLoad::Congested
    } else {
        FixtureLoad::Normal
    }
}

/// One club's judgement for one matchday. Derived rather than rolled, so asking
/// for the same fixture's lineup twice gives the same answer — the builder runs
/// on both match paths and must not name a different side each time.
fn selection_seed(team_id: &str, date: &str) -> u64 {
    stable_hash(date.as_bytes(), stable_hash(team_id.as_bytes(), 0))
}

/// How wrong this manager is about this player today, as −1.0 ..= 1.0.
fn judgement_noise(seed: u64, player_id: &str) -> f64 {
    let hash = stable_hash(player_id.as_bytes(), seed);
    let unit = (hash >> 11) as f64 / (1u64 << 53) as f64;
    unit * 2.0 - 1.0
}

/// The eleven a club would pick if nobody were tired: for each formation slot in
/// turn, the best condition-free positional fit not already chosen, returned in
/// slot order.
///
/// Step one of the AI's team selection, and also what AI training reads to judge
/// how fresh its first eleven is — the two must agree on who that eleven is, or
/// the controller steers on players who never start (a spare keeper, most
/// often). Ties break on id so the answer does not depend on the order the
/// squad happens to be stored in.
pub(crate) fn first_choice_eleven<'a>(
    available_players: &[&'a domain::player::Player],
    formation: &str,
) -> Vec<&'a domain::player::Player> {
    let mut chosen: Vec<&'a domain::player::Player> = Vec::with_capacity(11);
    for slot in formation_slots(formation).iter().take(11) {
        let best = available_players
            .iter()
            .copied()
            .filter(|player| !chosen.iter().any(|picked| picked.id == player.id))
            .max_by(|left, right| {
                positional_fit_for_assignment(left, slot)
                    .partial_cmp(&positional_fit_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.id.cmp(&left.id))
            });
        let Some(player) = best else {
            break;
        };
        chosen.push(player);
    }
    chosen
}

/// Reputation-aware AI lineup selection.
///
/// Step 1 picks the first-choice XI purely on condition-free positional fit, so a
/// club always fields its best players when fresh. Step 2 rests the tired ones.
///
/// What management quality buys is **judgement, not willingness**. A weak manager
/// notices fatigue later (`rest_threshold`) and is a worse judge of who can
/// deputise (`misjudgement`) — but every manager wants to win, so none of them
/// deliberately sends out a player who cannot run. Two rules keep that honest:
///
/// - The quality drop a manager will accept in order to rest someone is bounded
///   below. It used to be `12 × quality`, which handed the worst manager a
///   tolerance of exactly zero: since step 1 has already taken the best player
///   for the slot, no remaining deputy can match them, so rotation was
///   arithmetically impossible however exhausted the XI became. That was not a
///   gradient, it was self-harm.
/// - Below `EXHAUSTED` there is no gradient at all. A player that spent is
///   visibly unfit to start and comes out for anyone adequate.
///
/// Where judgement does bite is *which* deputy gets the shirt: the adequacy gate
/// is applied to a player's real standard, so no manager talks themselves into
/// fielding someone hopeless, but the choice between adequate options is made on
/// what the manager believes, which a poor one gets wrong.
fn ai_select_starting_xi<'a>(
    available_players: &[&'a domain::player::Player],
    formation: &str,
    quality: f64,
    seed: u64,
    load: FixtureLoad,
) -> Vec<&'a domain::player::Player> {
    /// A rotation candidate must be at least this fresh to be worth considering.
    const FRESH_FLOOR: f64 = 60.0;
    /// And meaningfully fresher than the starter it would replace.
    const MIN_FRESHNESS_GAIN: i16 = 10;
    /// Below this a player is not fit to start, whatever their manager makes of
    /// the alternatives. This is the floor the quality gradient stands on.
    const EXHAUSTED: f64 = 45.0;
    /// Quality drop a manager will accept to rest someone, worst and best. Never
    /// zero: a manager who will accept no drop can never rotate at all.
    const MIN_FIT_TOLERANCE: f64 = 6.0;
    const MAX_FIT_TOLERANCE: f64 = 12.0;
    /// On a congested run every manager rests earlier, and accepts a little more
    /// of a drop to do it — a tired first eleven asked to play twice in four days
    /// arrives spent at the second match however good it is. Manager quality
    /// still shades both: a better one rests sooner. See [`FixtureLoad`].
    const CONGESTED_REST_THRESHOLD: f64 = 85.0;
    const CONGESTED_REST_BY_QUALITY: f64 = 10.0;
    const CONGESTED_MIN_FIT_TOLERANCE: f64 = 10.0;
    /// And what any of them will accept to get an exhausted player off the pitch.
    const EXHAUSTED_FIT_TOLERANCE: f64 = 20.0;
    /// How far the worst manager can misread a player's standard, either way.
    const MAX_MISJUDGEMENT: f64 = 9.0;

    let slots = formation_slots(formation);

    // Step 1: first-choice XI by condition-free positional fit.
    // (slot index, chosen player) so the rotation step can re-evaluate per slot.
    let mut selected: Vec<(usize, &'a domain::player::Player)> =
        first_choice_eleven(available_players, formation)
            .into_iter()
            .enumerate()
            .collect();
    let mut used_ids: HashSet<String> = selected
        .iter()
        .map(|(_, player)| player.id.clone())
        .collect();

    // Step 2: load management.
    let (rest_threshold, min_tolerance) = match load {
        // 50 (poor) .. 75 (elite); tolerance 6 .. 12.
        FixtureLoad::Normal => (50.0 + 25.0 * quality, MIN_FIT_TOLERANCE),
        // 85 (poor) .. 95 (elite); tolerance 10 .. 12.
        FixtureLoad::Congested => (
            CONGESTED_REST_THRESHOLD + CONGESTED_REST_BY_QUALITY * quality,
            CONGESTED_MIN_FIT_TOLERANCE,
        ),
    };
    let fit_tolerance = min_tolerance + (MAX_FIT_TOLERANCE - min_tolerance) * quality;
    let misjudgement = MAX_MISJUDGEMENT * (1.0 - quality); // 9 (poor) .. 0 (elite)

    // What the manager believes a player is worth in this slot, which is the
    // truth only for the very best of them. Stable per player per matchday, so a
    // manager holds one opinion for the whole team sheet rather than a fresh one
    // per comparison.
    let perceived_fit = |player: &domain::player::Player, slot: &DomainPosition| {
        positional_fit_for_assignment(player, slot)
            + judgement_noise(seed, &player.id) * misjudgement
    };

    for entry in selected.iter_mut() {
        let slot = &slots[entry.0];
        let starter = entry.1;

        let condition = f64::from(starter.condition);
        if condition >= rest_threshold {
            continue; // Fresh enough — no reason to rotate.
        }
        // A spent player comes out for anyone adequate; a merely tired one only
        // for someone close to their own standard.
        let tolerance = if condition < EXHAUSTED {
            EXHAUSTED_FIT_TOLERANCE
        } else {
            fit_tolerance
        };

        let starter_fit = positional_fit_for_assignment(starter, slot);
        let starter_group = starter.position.to_group_position();
        let fresh_alternative = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            // Only rotate within the same position group, so load management never
            // skews the formation's distribution (e.g. fielding a 5th midfielder
            // in place of a defender, which would leave the XI a man short).
            .filter(|player| player.position.to_group_position() == starter_group)
            .filter(|player| f64::from(player.condition) >= FRESH_FLOOR)
            .filter(|player| {
                i16::from(player.condition) - i16::from(starter.condition) >= MIN_FRESHNESS_GAIN
            })
            // Adequacy is measured against what the deputy can actually do. A
            // manager may misjudge which of two capable players is better; none of
            // them mistakes a reserve-team player for a first-choice one.
            .filter(|player| positional_fit_for_assignment(player, slot) >= starter_fit - tolerance)
            // The choice between adequate options, though, is the manager's read.
            .max_by(|left, right| {
                perceived_fit(left, slot)
                    .partial_cmp(&perceived_fit(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(fresh) = fresh_alternative {
            used_ids.remove(&starter.id);
            used_ids.insert(fresh.id.clone());
            entry.1 = fresh;
        }
    }

    selected.into_iter().map(|(_, player)| player).collect()
}

pub(crate) fn domain_to_engine_role(role: &domain::team::PlayerRole) -> EnginePlayerRole {
    match role {
        domain::team::PlayerRole::Standard => EnginePlayerRole::Standard,
        domain::team::PlayerRole::BallPlayingKeeper => EnginePlayerRole::BallPlayingKeeper,
        domain::team::PlayerRole::SweeperKeeper => EnginePlayerRole::SweeperKeeper,
        domain::team::PlayerRole::Stopper => EnginePlayerRole::Stopper,
        domain::team::PlayerRole::CoverCB => EnginePlayerRole::CoverCB,
        domain::team::PlayerRole::BallPlayingCB => EnginePlayerRole::BallPlayingCB,
        domain::team::PlayerRole::AttackingFB => EnginePlayerRole::AttackingFB,
        domain::team::PlayerRole::DefensiveFB => EnginePlayerRole::DefensiveFB,
        domain::team::PlayerRole::InvertedFB => EnginePlayerRole::InvertedFB,
        domain::team::PlayerRole::WingBack => EnginePlayerRole::WingBack,
        domain::team::PlayerRole::AnchorMan => EnginePlayerRole::AnchorMan,
        domain::team::PlayerRole::BallWinner => EnginePlayerRole::BallWinner,
        domain::team::PlayerRole::DeepLyingPlaymaker => EnginePlayerRole::DeepLyingPlaymaker,
        domain::team::PlayerRole::BoxToBox => EnginePlayerRole::BoxToBox,
        domain::team::PlayerRole::Carrilero => EnginePlayerRole::Carrilero,
        domain::team::PlayerRole::Mezzala => EnginePlayerRole::Mezzala,
        domain::team::PlayerRole::AdvancedPlaymaker => EnginePlayerRole::AdvancedPlaymaker,
        domain::team::PlayerRole::ShadowStriker => EnginePlayerRole::ShadowStriker,
        domain::team::PlayerRole::WideForward => EnginePlayerRole::WideForward,
        domain::team::PlayerRole::InsideForward => EnginePlayerRole::InsideForward,
        domain::team::PlayerRole::InvertedWinger => EnginePlayerRole::InvertedWinger,
        domain::team::PlayerRole::Poacher => EnginePlayerRole::Poacher,
        domain::team::PlayerRole::TargetMan => EnginePlayerRole::TargetMan,
        domain::team::PlayerRole::DeepLyingForward => EnginePlayerRole::DeepLyingForward,
        domain::team::PlayerRole::False9 => EnginePlayerRole::False9,
        domain::team::PlayerRole::PressingForward => EnginePlayerRole::PressingForward,
        domain::team::PlayerRole::CompleteForward => EnginePlayerRole::CompleteForward,
    }
}

pub(crate) fn domain_to_engine_tactics(t: &domain::team::TacticsPhaseSettings) -> TacticsConfig {
    TacticsConfig {
        pressing_intensity: match t.pressing_intensity {
            domain::team::PressingIntensity::Passive => PressingIntensity::Passive,
            domain::team::PressingIntensity::Medium => PressingIntensity::Medium,
            domain::team::PressingIntensity::Aggressive => PressingIntensity::Aggressive,
        },
        defensive_line: match t.defensive_line {
            domain::team::DefensiveLine::VeryLow => DefensiveLine::VeryLow,
            domain::team::DefensiveLine::Low => DefensiveLine::Low,
            domain::team::DefensiveLine::Medium => DefensiveLine::Medium,
            domain::team::DefensiveLine::High => DefensiveLine::High,
        },
        width: match t.width {
            domain::team::PitchWidth::Narrow => TacticsPitchWidth::Narrow,
            domain::team::PitchWidth::Normal => TacticsPitchWidth::Normal,
            domain::team::PitchWidth::Wide => TacticsPitchWidth::Wide,
        },
        build_up_style: match t.build_up_style {
            domain::team::BuildUpStyle::Short => TacticsBuildUpStyle::Short,
            domain::team::BuildUpStyle::Mixed => TacticsBuildUpStyle::Mixed,
            domain::team::BuildUpStyle::Long => TacticsBuildUpStyle::Long,
        },
        marking_style: match t.marking_style {
            domain::team::MarkingStyle::Zonal => MarkingStyle::Zonal,
            domain::team::MarkingStyle::Mixed => MarkingStyle::Mixed,
            domain::team::MarkingStyle::ManToMan => MarkingStyle::ManToMan,
        },
        tempo: match t.tempo {
            domain::team::Tempo::Patient => Tempo::Patient,
            domain::team::Tempo::Direct => Tempo::Direct,
        },
        defensive_shape: match t.defensive_shape {
            domain::team::DefensiveShape::Stretched => DefensiveShape::Stretched,
            domain::team::DefensiveShape::Normal => DefensiveShape::Normal,
            domain::team::DefensiveShape::Compact => DefensiveShape::Compact,
        },
        counter_press_duration: match t.counter_press_duration {
            domain::team::CounterPressDuration::None => CounterPressDuration::None,
            domain::team::CounterPressDuration::Short => CounterPressDuration::Short,
            domain::team::CounterPressDuration::Long => CounterPressDuration::Long,
        },
        break_speed: match t.break_speed {
            domain::team::BreakSpeed::Slow => BreakSpeed::Slow,
            domain::team::BreakSpeed::Medium => BreakSpeed::Medium,
            domain::team::BreakSpeed::Fast => BreakSpeed::Fast,
        },
    }
}

fn to_engine_player(
    p: &domain::player::Player,
    role: EnginePlayerRole,
    deployed: Option<&DomainPosition>,
) -> PlayerData {
    // Fall back to the player's natural position (not `p.position`, which on
    // legacy saves can still hold a stale coarse bucket written by the old
    // set_formation stat-ranking).
    let group = deployed
        .cloned()
        .unwrap_or_else(|| p.natural_position.clone())
        .to_group_position();
    let pos = match group {
        DomainPosition::Goalkeeper => Position::Goalkeeper,
        DomainPosition::Defender => Position::Defender,
        DomainPosition::Midfielder => Position::Midfielder,
        DomainPosition::Forward => Position::Forward,
        _ => Position::Midfielder,
    };

    PlayerData {
        id: p.id.clone(),
        name: p.match_name.clone(),
        position: pos,
        ovr: p.ovr,
        condition: p.condition,
        fitness: p.fitness,
        pace: p.attributes.pace,
        stamina: p.attributes.stamina,
        strength: p.attributes.strength,
        agility: p.attributes.agility,
        passing: p.attributes.passing,
        shooting: p.attributes.shooting,
        tackling: p.attributes.tackling,
        dribbling: p.attributes.dribbling,
        defending: p.attributes.defending,
        positioning: p.attributes.positioning,
        vision: p.attributes.vision,
        decisions: p.attributes.decisions,
        composure: p.attributes.composure,
        aggression: p.attributes.aggression,
        teamwork: p.attributes.teamwork,
        leadership: p.attributes.leadership,
        handling: p.attributes.handling,
        reflexes: p.attributes.reflexes,
        aerial: p.attributes.aerial,
        traits: p.traits.iter().map(|t| format!("{:?}", t)).collect(),
        role,
    }
}

/// Auto-select set-piece takers from a set of player IDs.
/// Returns (captain_id, penalty_taker_id, free_kick_taker_id, corner_taker_id).
pub fn auto_select_set_pieces(
    game: &Game,
    player_ids: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let players: Vec<&domain::player::Player> = player_ids
        .iter()
        .filter_map(|id| game.players.iter().find(|p| &p.id == id))
        .collect();

    if players.is_empty() {
        return (None, None, None, None);
    }

    // Captain: highest leadership + teamwork
    let captain = players
        .iter()
        .max_by_key(|p| (p.attributes.leadership as u16) + (p.attributes.teamwork as u16))
        .map(|p| p.id.clone());

    // Penalty taker: highest shooting + composure (exclude GK)
    let penalty = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| (p.attributes.shooting as u16) + (p.attributes.composure as u16))
        .map(|p| p.id.clone());

    // Free kick taker: highest passing + vision + shooting (exclude GK)
    let free_kick = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            (p.attributes.passing as u16)
                + (p.attributes.vision as u16)
                + (p.attributes.shooting as u16) / 2
        })
        .map(|p| p.id.clone());

    // Corner taker: highest passing + vision (exclude GK, prefer different from FK)
    let corner = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            let base = (p.attributes.passing as u16) + (p.attributes.vision as u16);
            // Small penalty if same as free kick taker to encourage variety
            if free_kick.as_ref() == Some(&p.id) {
                base.saturating_sub(5)
            } else {
                base
            }
        })
        .map(|p| p.id.clone());

    (captain, penalty, free_kick, corner)
}

#[cfg(test)]
mod tests;
