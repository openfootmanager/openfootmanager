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
        ai_select_starting_xi(&available_players, &formation, quality, seed)
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
    /// And what any of them will accept to get an exhausted player off the pitch.
    const EXHAUSTED_FIT_TOLERANCE: f64 = 20.0;
    /// How far the worst manager can misread a player's standard, either way.
    const MAX_MISJUDGEMENT: f64 = 9.0;

    let slots = formation_slots(formation);
    let mut used_ids: HashSet<String> = HashSet::new();
    // (slot index, chosen player) so the rotation step can re-evaluate per slot.
    let mut selected: Vec<(usize, &'a domain::player::Player)> = Vec::with_capacity(11);

    // Step 1: first-choice XI by condition-free positional fit.
    for (slot_index, slot) in slots.iter().take(11).enumerate() {
        let best = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            .max_by(|left, right| {
                positional_fit_for_assignment(left, slot)
                    .partial_cmp(&positional_fit_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(player) = best else {
            break;
        };
        used_ids.insert(player.id.clone());
        selected.push((slot_index, player));
    }

    // Step 2: load management.
    let rest_threshold = 50.0 + 25.0 * quality; // 50 (poor) .. 75 (elite)
    let fit_tolerance = MIN_FIT_TOLERANCE + (MAX_FIT_TOLERANCE - MIN_FIT_TOLERANCE) * quality; // 6 .. 12
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
mod tests {
    use super::*;
    use domain::player::{Player, PlayerAttributes, Position as DomainPos};

    /// Uniform attributes: `weighted_score` averages attributes, so setting them
    /// all to `v` makes the condition-free positional fit ≈ `v` for any slot, with
    /// the per-slot compatibility/foot penalty identical across players (same
    /// position + footedness) — so it cancels in within-slot comparisons.
    fn attrs(v: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: v,
            stamina: v,
            strength: v,
            agility: v,
            passing: v,
            shooting: v,
            tackling: v,
            dribbling: v,
            defending: v,
            positioning: v,
            vision: v,
            decisions: v,
            composure: v,
            aggression: v,
            teamwork: v,
            leadership: v,
            handling: v,
            reflexes: v,
            aerial: v,
        }
    }

    fn mk(id: &str, attr: u8, condition: u8) -> Player {
        mk_pos(id, DomainPos::CenterBack, attr, condition)
    }

    fn mk_pos(id: &str, position: DomainPos, attr: u8, condition: u8) -> Player {
        let mut p = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-01-01".to_string(),
            "GB".to_string(),
            position,
            attrs(attr),
        );
        p.condition = condition;
        p
    }

    #[test]
    fn management_quality_maps_reputation_to_unit_range() {
        assert_eq!(management_quality(300), 0.0);
        assert_eq!(management_quality(900), 1.0);
        assert!((management_quality(600) - 0.5).abs() < 1e-9);
        assert_eq!(management_quality(100), 0.0); // clamped below
        assert_eq!(management_quality(1200), 1.0); // clamped above
    }

    /// The discriminating test: an elite club must NOT bench a strong (but mildly
    /// tired) starter for a much weaker fresh player. Better clubs field better
    /// teams — the gap-aware tolerance enforces this.
    #[test]
    fn elite_club_keeps_strong_starters_over_fresh_scrubs() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 70)); // strong, mildly tired
        }
        for i in 0..3 {
            squad.push(mk(&format!("weak{i}"), 50, 100)); // weak, fully fresh
        }
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1);

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().all(|p| p.id.starts_with("star")),
            "elite club fielded a weak fresh player over a strong starter: {:?}",
            xi.iter().map(|p| &p.id).collect::<Vec<_>>()
        );
    }

    /// An elite club rotates a tired starter for a *comparable* fresh deputy
    /// (within the fit tolerance), but still leaves the much-weaker scrubs benched.
    #[test]
    fn elite_club_rotates_tired_starter_for_comparable_fresh_player() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 65)); // strong, tired
        }
        squad.push(mk("deputy", 72, 100)); // comparable, fresh
        for i in 0..2 {
            squad.push(mk(&format!("weak{i}"), 50, 100)); // scrub, fresh
        }
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1);

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().any(|p| p.id == "deputy"),
            "comparable fresh deputy should rotate in for a tired star"
        );
        assert_eq!(
            xi.iter().filter(|p| p.id.starts_with("star")).count(),
            10,
            "exactly one tired star should be rested"
        );
        assert!(
            xi.iter().all(|p| !p.id.starts_with("weak")),
            "scrubs are too far below tolerance to be rotated in"
        );
    }

    /// Same squad as above, but a poorly-run club: it is slow to notice fatigue and
    /// rides a *mildly* tired XI. This is the reputation gradient, and it survives —
    /// what it is not allowed to do is ride an exhausted one, which is the test
    /// below.
    #[test]
    fn low_reputation_club_rides_mildly_tired_starters() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 65));
        }
        squad.push(mk("deputy", 72, 100));
        for i in 0..2 {
            squad.push(mk(&format!("weak{i}"), 50, 100));
        }
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", management_quality(300), 1); // q = 0

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().all(|p| p.id.starts_with("star")),
            "a low-reputation club should ride its mildly tired first XI, not rotate"
        );
    }

    /// The behaviour this slice exists to change. A manager may be slow, or a poor
    /// judge of who the better player is; no manager knowingly sends out a player
    /// who can barely run when a rested deputy is available. Under the old
    /// `fit_tolerance = 12 × quality` the worst manager accepted a quality drop of
    /// exactly zero, so it could never rotate at all.
    #[test]
    fn even_the_worst_manager_rests_an_exhausted_starter() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 22)); // first choice, spent
        }
        squad.push(mk("deputy", 66, 95)); // clearly worse, but able to play
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", management_quality(300), 1); // q = 0

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().any(|p| p.id == "deputy"),
            "an exhausted XI must be broken up even by the worst manager: {:?}",
            xi.iter().map(|p| &p.id).collect::<Vec<_>>()
        );
    }

    /// A universal floor is not a licence to field anybody. Even exhausted, a club
    /// does not replace its best player with someone hopeless.
    #[test]
    fn an_exhausted_starter_is_not_replaced_by_a_hopeless_deputy() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 22));
        }
        squad.push(mk("hopeless", 30, 100));
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", management_quality(300), 1);

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().all(|p| p.id != "hopeless"),
            "a 30-rated player is not an adequate deputy at any fatigue level"
        );
    }

    /// Quality is judgement, not willingness. Given two deputies of nearly equal
    /// standard, an elite manager always identifies the better one; a poor manager
    /// gets it wrong some of the time. Fixed seeds, so this is deterministic.
    #[test]
    fn a_poor_manager_misjudges_which_deputy_is_better() {
        let pick_deputy = |quality: f64, seed: u64| -> String {
            let mut squad = Vec::new();
            for i in 0..11 {
                squad.push(mk(&format!("star{i}"), 80, 22));
            }
            squad.push(mk("better", 70, 95));
            squad.push(mk("worse", 66, 95));
            let refs: Vec<&Player> = squad.iter().collect();
            let xi = ai_select_starting_xi(&refs, "4-4-2", quality, seed);
            xi.iter()
                .find(|p| !p.id.starts_with("star"))
                .map(|p| p.id.clone())
                .unwrap_or_default()
        };

        let seeds: Vec<u64> = (0..24).collect();
        let elite: Vec<String> = seeds.iter().map(|s| pick_deputy(1.0, *s)).collect();
        let poor: Vec<String> = seeds.iter().map(|s| pick_deputy(0.0, *s)).collect();

        // Stated first and deliberately: without it, a manager that never rotated
        // at all would satisfy "sometimes picks the worse one" by picking nobody,
        // which is how the old behaviour would have passed this test.
        assert!(
            poor.iter().all(|d| d == "better" || d == "worse"),
            "a poor manager must still name a deputy for an exhausted XI: {poor:?}"
        );
        assert!(
            elite.iter().all(|d| d == "better"),
            "an elite manager should always spot the better deputy: {elite:?}"
        );
        assert!(
            poor.iter().any(|d| d == "worse"),
            "a poor manager should sometimes pick the worse deputy; got it right every time"
        );
    }

    /// The seed is the club and the date, so one fixture always produces one XI
    /// however many times the builder is asked for it.
    #[test]
    fn the_same_seed_always_names_the_same_side() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 22));
        }
        squad.push(mk("a", 70, 95));
        squad.push(mk("b", 69, 95));
        let refs: Vec<&Player> = squad.iter().collect();

        let first = ai_select_starting_xi(&refs, "4-4-2", 0.0, 7);
        let second = ai_select_starting_xi(&refs, "4-4-2", 0.0, 7);

        let ids = |xi: &Vec<&Player>| xi.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&first), ids(&second));
    }

    /// Regression: load-management rotation must not skew the formation's
    /// position distribution. A tired XI plus one fresh midfielder must still
    /// field 1 GK / 4 DEF / 4 MID / 2 FWD — never a defender short (which
    /// rendered only 10 players on the pitch).
    #[test]
    fn rotation_preserves_formation_position_distribution() {
        use DomainPos::{
            CenterBack, CentralMidfielder, Forward, Goalkeeper, LeftBack, LeftMidfielder,
            RightBack, RightMidfielder, Striker,
        };
        let squad = vec![
            mk_pos("gk", Goalkeeper, 75, 65),
            mk_pos("d1", CenterBack, 75, 65),
            mk_pos("d2", CenterBack, 75, 65),
            mk_pos("d3", LeftBack, 75, 65),
            mk_pos("d4", RightBack, 75, 65),
            mk_pos("m1", CentralMidfielder, 75, 65),
            mk_pos("m2", CentralMidfielder, 75, 65),
            mk_pos("m3", LeftMidfielder, 75, 65),
            mk_pos("m4", RightMidfielder, 75, 65),
            mk_pos("f1", Striker, 75, 65),
            mk_pos("f2", Striker, 75, 65),
            // Fresh midfielder load management will want to bring in.
            mk_pos("m_fresh", CentralMidfielder, 75, 100),
        ];
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1); // elite: rotates eagerly

        assert_eq!(xi.len(), 11);
        let group_count = |group: DomainPos| {
            xi.iter()
                .filter(|p| p.position.to_group_position() == group)
                .count()
        };
        assert_eq!(group_count(Goalkeeper), 1, "exactly one keeper");
        assert_eq!(
            group_count(DomainPos::Defender),
            4,
            "must field four defenders"
        );
        assert_eq!(
            group_count(DomainPos::Midfielder),
            4,
            "must field four midfielders"
        );
        assert_eq!(group_count(Forward), 2, "must field two forwards");
    }

    /// With fewer available players than formation slots, the slot-aligned fill
    /// leaves gaps; the function must fall back to a contiguous selection rather
    /// than flatten()ing those gaps (which would shift starters into wrong slots).
    #[test]
    fn select_starting_xi_falls_back_when_fewer_players_than_slots() {
        let squad: Vec<Player> = (0..9)
            .map(|i| mk_pos(&format!("p{i}"), DomainPos::CenterBack, 70, 100))
            .collect();
        let refs: Vec<&Player> = squad.iter().collect();
        let saved: Vec<String> = (0..9).map(|i| format!("p{i}")).collect();

        let xi = select_starting_xi(&saved, &refs, "4-4-2");

        // Every available player fielded once, with no slot-misaligning gaps.
        assert_eq!(xi.len(), 9);
        let unique: HashSet<&String> = xi.iter().map(|p| &p.id).collect();
        assert_eq!(unique.len(), 9);
    }

    /// When a saved starter is unavailable the user XI compacts; the surviving
    /// starters must keep their real saved slot (matching deployed_position /
    /// the UI), not shift up into the vacated slot. Regression for an injured
    /// keeper turning an outfielder into the engine's goalkeeper.
    #[test]
    fn user_team_starter_keeps_saved_slot_when_xi_compacts() {
        use crate::clock::GameClock;
        use chrono::{TimeZone, Utc};
        use domain::manager::Manager;
        use domain::team::Team;

        let mut players: Vec<Player> = (1..=11)
            .map(|i| mk_pos(&format!("p{i}"), DomainPos::CenterBack, 70, 100))
            .collect();
        // p1 is a natural Forward saved into the LEFT-BACK slot (index 1).
        players[0] = mk_pos("p1", DomainPos::Forward, 70, 100);
        for player in players.iter_mut() {
            player.team_id = Some("user".to_string());
        }

        let mut team = Team::new(
            "user".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            25_000,
        );
        team.formation = "4-4-2".to_string();
        // Slot 0 (GK) references a player that no longer exists, so
        // select_starting_xi drops it and the survivors compact.
        team.starting_xi_ids = std::iter::once("ghost-gk".to_string())
            .chain((1..=10).map(|i| format!("p{i}")))
            .collect();

        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());

        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let game = Game::new(clock, manager, vec![team], players, vec![], vec![]);

        let (team_data, _bench) = build_team_with_bench(&game, "user");

        let p1 = team_data
            .players
            .iter()
            .find(|p| p.id == "p1")
            .expect("p1 should be fielded");
        // Saved at the left-back slot -> simulated as a defender, never shoved
        // into the vacated goalkeeper slot.
        assert_eq!(p1.position, Position::Defender);
        assert_ne!(p1.position, Position::Goalkeeper);

        // The vacated goalkeeper slot must be refilled, so the XI still fields
        // exactly one keeper (otherwise the engine's goalkeeper rating collapses
        // to its empty-set fallback).
        let keepers = team_data
            .players
            .iter()
            .filter(|p| p.position == Position::Goalkeeper)
            .count();
        assert_eq!(keepers, 1, "the XI must still contain a goalkeeper");
    }
}
