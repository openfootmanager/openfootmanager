use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::PlayStyle;
use rand::{Rng, RngExt};
use uuid::Uuid;

use super::definitions::{NamePool, NamesDefinition};
use crate::nations;
use crate::player_rating::{generate_potential, refresh_player_derived};

// ---------------------------------------------------------------------------
// Helper functions for world generation
// ---------------------------------------------------------------------------

/// Compute a sensible alternate position based on primary position and attributes.
fn compute_alternate_position(primary: &Position, attrs: &PlayerAttributes) -> Option<Position> {
    match primary.to_group_position() {
        Position::Goalkeeper => None,
        Position::Defender => {
            // Defenders with good passing/vision → Midfielder
            if attrs.passing >= 65 && attrs.vision >= 60 {
                Some(Position::Midfielder)
            } else {
                None
            }
        }
        Position::Midfielder => {
            // Midfielders with strong defending/tackling → Defender
            if attrs.defending >= 65 && attrs.tackling >= 60 {
                Some(Position::Defender)
            }
            // Midfielders with good shooting/dribbling → Forward
            else if attrs.shooting >= 65 && attrs.dribbling >= 60 {
                Some(Position::Forward)
            } else {
                None
            }
        }
        Position::Forward => {
            // Forwards with good passing/vision → Midfielder
            if attrs.passing >= 65 && attrs.vision >= 60 {
                Some(Position::Midfielder)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Pick a nationality: 60% the club's own country, 40% from the wider draw.
///
/// When the club's country resolves to no known nation the local weight has
/// nothing to apply to, so the whole draw comes from `available_codes`. That is
/// the #453 fix: an unrecognised country used to mean England, specifically and
/// silently, for 60% of the squad.
pub(super) fn pick_nationality_from_def(
    team_country: &str,
    available_codes: &[String],
    rng: &mut impl Rng,
) -> String {
    let local_code = resolve_nationality_code(team_country);
    let selected_code = match local_code {
        Some(code) if available_codes.is_empty() || rng.random_range(0..100) < 60 => code,
        Some(_) | None if !available_codes.is_empty() => {
            available_codes[rng.random_range(0..available_codes.len())].clone()
        }
        // Nothing local and nothing to draw from: the caller has no nations at
        // all. Better an obviously-unset value than a plausible wrong one.
        _ => String::new(),
    };

    canonicalize_generated_nationality(&selected_code)
}

pub(super) fn canonicalize_generated_nationality(value: &str) -> String {
    match value.trim().to_ascii_uppercase().as_str() {
        // Freshly generated football identities should never persist the ambiguous GB code.
        "GB" => "ENG".to_string(),
        other => other.to_string(),
    }
}

/// Pick a name from the NamesDefinition for a given nationality code.
pub(super) fn pick_name_from_def(
    nationality: &str,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> (String, String) {
    let candidate_codes = match nationality {
        "ENG" | "SCO" | "WAL" | "NIR" => vec![nationality, "GB"],
        _ => vec![nationality],
    };

    for candidate in candidate_codes {
        if let Some(pool) = names_def.pools.get(candidate)
            && !pool.first_names.is_empty()
            && !pool.last_names.is_empty()
        {
            let first = pool.first_names[rng.random_range(0..pool.first_names.len())].clone();
            let last = pool.last_names[rng.random_range(0..pool.last_names.len())].clone();
            return (first, last);
        }
    }

    match fallback_pool(nationality, names_def, rng) {
        Some(pool) => draw_name(pool, rng),
        None => ("Player".to_string(), "Unknown".to_string()),
    }
}

/// A random first/last pair from `pool`. Callers must have checked it is usable.
fn draw_name(pool: &NamePool, rng: &mut impl Rng) -> (String, String) {
    let first = pool.first_names[rng.random_range(0..pool.first_names.len())].clone();
    let last = pool.last_names[rng.random_range(0..pool.last_names.len())].clone();
    (first, last)
}

/// The pool to borrow from when a nationality has none of its own — the common
/// case, since only 17 pools ship against ~210 selectable nations.
///
/// Prefers a pool from the same confederation, then any usable pool. This used to
/// take `pools.keys().min()`, the lexicographically smallest key, which for the
/// shipped set is always `AR`: a Pole, a Nigerian and a Japanese player were
/// all named Ezequiel, deterministically.
///
/// Both sides of the region comparison must resolve to a *declared* region.
/// `region_for_code` answers `europe` for anything it does not recognise, so
/// matching on it would file every unknown key — a package keying its pools
/// `BRA`/`ESP`/`JPN`, say — as European, then hand a Pole a Japanese name
/// while a Brazilian matched none of them. An unknown code on either side falls
/// straight through to "any usable pool" instead, which is merely arbitrary
/// rather than confidently wrong.
///
/// Candidates are sorted before the draw because `pools` is a `HashMap` whose
/// iteration order is randomized per process.
fn fallback_pool<'a>(
    nationality: &str,
    names_def: &'a NamesDefinition,
    rng: &mut impl Rng,
) -> Option<&'a NamePool> {
    let usable = |pool: &NamePool| !pool.first_names.is_empty() && !pool.last_names.is_empty();
    let region_of = |code: &str| nations::nation_by_code(code).map(|nation| nation.region_id);

    let mut candidates: Vec<(&String, &NamePool)> = Vec::new();
    if let Some(region) = region_of(nationality) {
        candidates = names_def
            .pools
            .iter()
            .filter(|(code, pool)| usable(pool) && region_of(code) == Some(region))
            .collect();
    }
    if candidates.is_empty() {
        candidates = names_def
            .pools
            .iter()
            .filter(|(_, pool)| usable(pool))
            .collect();
    }
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by(|left, right| left.0.cmp(right.0));

    Some(candidates[rng.random_range(0..candidates.len())].1)
}

/// Resolve a club's declared country to a nationality code, or `None` when it
/// names no nation the game knows.
///
/// Replaces `country_to_iso`, which matched 17 country names by hand and
/// answered `"ENG"` for everything else — so `"country": "Japan"` filled 60% of
/// every Japanese club with English players, with nothing in the log or the UI
/// to say so. Its length heuristic was a second wrong answer: any 2–3 character
/// string was passed through as though it were a code.
///
/// `None` is the important part. An unresolvable country must stay explicitly
/// unknown so the caller can draw from the whole distribution, rather than
/// being handed a real, specific, wrong nationality.
pub(super) fn resolve_nationality_code(country: &str) -> Option<String> {
    let trimmed = country.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Football identities first: this is what maps "England"/"english"/"eng" —
    // and the UK home nations generally — onto the codes the game uses.
    let normalized = domain::identity::normalize_football_nation_code(trimmed);
    if nations::nation_by_code(&normalized).is_some() {
        return Some(normalized);
    }
    // `GB` is a real football identity but never a generated nationality; the
    // caller canonicalises it to ENG.
    if normalized == "GB" {
        return Some(normalized);
    }

    // Then the catalog's own display names, which is what makes "Japan",
    // "Nigeria" and the other 190-odd nations resolve at all.
    nations::nation_by_name(trimmed).map(|nation| nation.code.to_string())
}

/// Every nationality a generated player can have, repeated in proportion to how
/// often it should come up.
///
/// #452: this used to be `names_def.pools.keys()` — the 17 name-pool keys. A
/// lookup table was doing the job of a population model, so a world contained
/// at most ~16 nationalities (14 of them European) no matter how many countries
/// it had, and no amount of catalog work could change that.
///
/// Weight comes from rank within region in [`nations::NATION_CATALOG`], which is
/// already documented as "strongest footballing traditions first within each
/// region" — so it is a signal the codebase already maintains rather than a new
/// hand-authored table. [`nations::ADDITIONAL_NATIONS`] form a flat low-weight
/// tail: reachable, but rare.
///
/// Deliberately *not* `NationGen.strength`: that exists for only the 16
/// generation nations and is already spoken for by club reputation, so using it
/// here would privilege exactly the nations this issue is about and couple two
/// unrelated models.
///
/// Expanded into a plain `Vec` so callers keep drawing with a uniform index —
/// the weighting lives here, once, instead of at every draw site. Built once:
/// it derives purely from `&'static` catalogs, and the order must be stable or
/// seeded generation stops reproducing.
pub(super) fn nationality_distribution() -> &'static Vec<String> {
    static DISTRIBUTION: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    DISTRIBUTION.get_or_init(|| {
        /// Weight of the strongest nation in a region. Ranks below it step down
        /// by one, so a region's top handful dominate its share without
        /// shutting the rest out.
        const TOP_WEIGHT: usize = 12;
        /// Floor for a World Cup nation, and the flat weight of the tail. Two
        /// tiers rather than one: Europe alone has 26 catalog nations, so a
        /// single floor of 1 made everything past rank 11 exactly as likely as
        /// a merely-selectable nation — Poland would have drawn as often as
        /// Andorra. A qualifying nation should always outrank a non-entrant.
        const CATALOG_FLOOR: usize = 2;
        const TAIL_WEIGHT: usize = 1;

        let mut pool = Vec::new();
        let mut seen_in_region: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();

        for nation in nations::NATION_CATALOG {
            let rank = seen_in_region.entry(nation.region_id).or_insert(0);
            let weight = TOP_WEIGHT.saturating_sub(*rank).max(CATALOG_FLOOR);
            *rank += 1;
            for _ in 0..weight {
                pool.push(nation.code.to_string());
            }
        }
        for nation in nations::ADDITIONAL_NATIONS {
            for _ in 0..TAIL_WEIGHT {
                pool.push(nation.code.to_string());
            }
        }
        pool
    })
}

pub(super) fn play_style_from_str(s: &str) -> PlayStyle {
    match s {
        "Attacking" => PlayStyle::Attacking,
        "Defensive" => PlayStyle::Defensive,
        "Possession" => PlayStyle::Possession,
        "Counter" => PlayStyle::Counter,
        "HighPress" => PlayStyle::HighPress,
        _ => PlayStyle::Balanced,
    }
}

/// Number of slots in a generated squad. The slot layout is `GK 0-1`, `DEF 2-8`,
/// `MID 9-15`, `FWD 16-21` — see [`generate_random_player_from_def`]. This is the
/// single source of truth for squad size; anything that walks or wraps slots
/// should use it rather than repeating the literal.
pub(super) const SQUAD_SLOTS: usize = 22;

/// Minimum number of players per position group a finished squad must keep, in
/// `[GK, DEF, MID, FWD]` order. Trimming generated players off an authored squad
/// must never take a group below these — a club with no goalkeeper is unplayable.
pub(super) const MIN_PLAYERS_PER_GROUP: [(Position, usize); 4] = [
    (Position::Goalkeeper, 2),
    (Position::Defender, 4),
    (Position::Midfielder, 4),
    (Position::Forward, 2),
];

/// Squad slots reserved as youth-aged, one per position group in
/// `[GK, DEF, MID, FWD]` order. Scouted youth recruits target these slots so they
/// generate at a consistent academy age, and senior generation must avoid them.
/// This is the single source of truth shared by the youth-recruit targeting,
/// youth-age generation, and national-team senior remap logic.
pub(super) const YOUTH_RESERVED_SLOTS: [usize; 4] = [1, 8, 15, 21];

/// Candidate youth slots for a (group) position target. A specific group yields
/// its single reserved slot; `None` (or any other position) yields all of them.
pub(super) fn youth_slots_for_target(group: Option<Position>) -> &'static [usize] {
    match group {
        Some(Position::Goalkeeper) => &YOUTH_RESERVED_SLOTS[0..1],
        Some(Position::Defender) => &YOUTH_RESERVED_SLOTS[1..2],
        Some(Position::Midfielder) => &YOUTH_RESERVED_SLOTS[2..3],
        Some(Position::Forward) => &YOUTH_RESERVED_SLOTS[3..4],
        _ => &YOUTH_RESERVED_SLOTS,
    }
}

/// Whether a squad slot is reserved for a youth-aged player.
pub(super) fn is_youth_reserved_slot(slot: usize) -> bool {
    YOUTH_RESERVED_SLOTS.contains(&slot)
}

/// Remap a youth-reserved slot to the adjacent senior slot (same position group)
/// so the player generates at a senior age; non-reserved slots pass through.
pub(super) fn senior_slot(slot: usize) -> usize {
    if is_youth_reserved_slot(slot) {
        slot - 1
    } else {
        slot
    }
}

pub(super) fn generate_random_player_from_def(
    team_id: &str,
    index: usize,
    nationality: &str,
    opening_year: u32,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Player {
    let (first_name, last_name) = pick_name_from_def(nationality, names_def, rng);
    let full_name = format!("{} {}", first_name, last_name);
    let match_name = last_name.clone();

    // Distribute positions: GK:0-1, DEF:2-8, MID:9-15, FWD:16-21
    let position = if index < 2 {
        Position::Goalkeeper
    } else if index < 9 {
        Position::Defender
    } else if index < 16 {
        Position::Midfielder
    } else {
        Position::Forward
    };

    let p_id = Uuid::new_v4().to_string();
    let nationality = nationality.to_string();

    // Reserve one slot per position group (GK + back line + midfield + attack) as
    // youth-aged so scouted youth recruits land at a consistent age across positions
    // and clubs can open with real academy prospects instead of an empty youth squad.
    let age = if is_youth_reserved_slot(index) {
        rng.random_range(17..22)
    } else {
        rng.random_range(17..36)
    };
    let birth_year = opening_year.saturating_sub(age);
    let birth_month = rng.random_range(1..13);
    let birth_day = rng.random_range(1..29);
    let dob = format!("{:04}-{:02}-{:02}", birth_year, birth_month, birth_day);

    let group = position.to_group_position();
    let is_gk = matches!(group, Position::Goalkeeper);
    let is_def = matches!(group, Position::Defender);
    let is_fwd = matches!(group, Position::Forward);

    let attributes = PlayerAttributes {
        pace: rng.random_range(40..95),
        stamina: rng.random_range(40..95),
        strength: rng.random_range(40..95),
        agility: rng.random_range(40..95),
        passing: rng.random_range(40..95),
        shooting: if is_gk {
            rng.random_range(20..50)
        } else {
            rng.random_range(40..95)
        },
        tackling: if is_gk || is_fwd {
            rng.random_range(20..60)
        } else {
            rng.random_range(40..95)
        },
        dribbling: if is_gk {
            rng.random_range(20..50)
        } else {
            rng.random_range(40..95)
        },
        defending: if is_gk {
            rng.random_range(25..55)
        } else if is_def {
            rng.random_range(55..95)
        } else {
            rng.random_range(40..95)
        },
        positioning: rng.random_range(40..95),
        vision: rng.random_range(40..95),
        decisions: rng.random_range(40..95),
        composure: rng.random_range(40..95),
        aggression: rng.random_range(30..90),
        teamwork: rng.random_range(45..95),
        leadership: rng.random_range(30..90),
        handling: if is_gk {
            rng.random_range(50..95)
        } else {
            rng.random_range(10..35)
        },
        reflexes: if is_gk {
            rng.random_range(50..95)
        } else {
            rng.random_range(20..50)
        },
        aerial: if is_gk {
            rng.random_range(50..95)
        } else if is_def {
            rng.random_range(45..90)
        } else {
            rng.random_range(30..75)
        },
    };

    // Size market value and wage from the same position-weighted rating the
    // player will be shown with, so a keeper is priced on keeping.
    let current_year: u32 = opening_year;

    let approx_ovr = crate::player_rating::ovr_from_attributes(&attributes, &position).round() as u32;

    let age_factor = if age <= 23 {
        1.5
    } else if age <= 28 {
        1.2
    } else if age <= 32 {
        0.8
    } else {
        0.4
    };
    let base_value = (approx_ovr as f64).powi(2) * 500.0;
    let market_value = (base_value * age_factor) as u64;
    let wage = (market_value / 200).max(500) as u32;
    let contract_years = if age <= 21 {
        rng.random_range(3..6)
    } else if age <= 27 {
        rng.random_range(2..5)
    } else if age <= 31 {
        rng.random_range(2..4)
    } else if rng.random_range(0..100) < 40 {
        1
    } else {
        2
    };
    let contract_end = format!("{}-06-30", opening_year.saturating_add(contract_years));

    let mut player = Player::new(
        p_id,
        match_name,
        full_name,
        dob,
        nationality,
        position,
        attributes,
    );
    player.team_id = Some(team_id.to_string());
    player.market_value = market_value;
    player.wage = wage;
    player.contract_end = Some(contract_end);
    player.condition = rng.random_range(75..100);
    player.morale = rng.random_range(40..76);

    // ~40% of outfield players get an alternate position based on attributes
    if !is_gk && rng.random_range(0..5) < 2 {
        let alt = compute_alternate_position(&player.position, &player.attributes);
        if let Some(pos) = alt {
            player.alternate_positions.push(pos);
        }
    }

    // Set position-weighted OVR, potential, and traits (Wonderkid included if applicable)
    let player_age = current_year.saturating_sub(birth_year);
    // Pre-generate a potential so Wonderkid trait is assigned correctly on first refresh
    let temp_ovr = {
        use crate::player_rating::natural_ovr;
        natural_ovr(&player).round() as u8
    };
    player.potential = generate_potential(temp_ovr, player_age);
    refresh_player_derived(&mut player, current_year);

    player.jersey_number = jersey_number_for_slot(index);

    player
}

fn jersey_number_for_slot(index: usize) -> Option<u8> {
    let n: u8 = match index {
        0 => 1,
        1 => 13,
        2 => 2,
        3 => 5,
        4 => 6,
        5 => 3,
        6 => 4,
        7 => 12,
        8 => 22,
        9 => 8,
        10 => 7,
        11 => 10,
        12 => 14,
        13 => 11,
        14 => 16,
        15 => 23,
        16 => 9,
        17 => 17,
        18 => 18,
        19 => 19,
        20 => 20,
        21 => 24,
        _ => return None,
    };
    Some(n)
}

pub(super) fn generate_random_staff_from_def(
    team_id: &str,
    role: StaffRole,
    nationality: &str,
    opening_year: u32,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Staff {
    let (first_name, last_name) = pick_name_from_def(nationality, names_def, rng);
    let age = rng.random_range(30..60);
    let birth_year = opening_year.saturating_sub(age);
    let dob = format!(
        "{:04}-{:02}-{:02}",
        birth_year,
        rng.random_range(1..13),
        rng.random_range(1..29)
    );

    let attributes = match &role {
        StaffRole::AssistantManager => StaffAttributes {
            coaching: rng.random_range(50..90),
            judging_ability: rng.random_range(50..85),
            judging_potential: rng.random_range(40..80),
            physiotherapy: rng.random_range(20..50),
        },
        StaffRole::Coach => StaffAttributes {
            coaching: rng.random_range(55..95),
            judging_ability: rng.random_range(40..75),
            judging_potential: rng.random_range(30..70),
            physiotherapy: rng.random_range(20..45),
        },
        StaffRole::Scout => StaffAttributes {
            coaching: rng.random_range(20..50),
            judging_ability: rng.random_range(60..95),
            judging_potential: rng.random_range(55..95),
            physiotherapy: rng.random_range(10..30),
        },
        StaffRole::Physio => StaffAttributes {
            coaching: rng.random_range(10..40),
            judging_ability: rng.random_range(20..50),
            judging_potential: rng.random_range(15..45),
            physiotherapy: rng.random_range(60..95),
        },
    };

    let mut s = Staff::new(
        Uuid::new_v4().to_string(),
        first_name,
        last_name,
        dob,
        role,
        attributes,
    );
    s.nationality = nationality.to_string();
    s.team_id = Some(team_id.to_string());
    s
}

pub(super) fn generate_random_staff_unattached_from_def(
    role: StaffRole,
    nationality: &str,
    opening_year: u32,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Staff {
    let (first_name, last_name) = pick_name_from_def(nationality, names_def, rng);
    let age = rng.random_range(28..55);
    let birth_year = opening_year.saturating_sub(age);
    let dob = format!(
        "{:04}-{:02}-{:02}",
        birth_year,
        rng.random_range(1..13),
        rng.random_range(1..29)
    );

    let attributes = StaffAttributes {
        coaching: rng.random_range(30..80),
        judging_ability: rng.random_range(30..80),
        judging_potential: rng.random_range(25..75),
        physiotherapy: rng.random_range(25..75),
    };

    let mut s = Staff::new(
        Uuid::new_v4().to_string(),
        first_name,
        last_name,
        dob,
        role,
        attributes,
    );
    s.nationality = nationality.to_string();
    s
}

// ---------------------------------------------------------------------------
// Authored staff (world packages)
// ---------------------------------------------------------------------------

/// Convert a hand-authored [`super::package::StaffDef`] into a domain [`Staff`].
/// Fills any missing fields (id, dob, attributes) with sensible random defaults.
pub(super) fn generate_staff_from_authored_def(
    def: &super::package::StaffDef,
    team_id: Option<&str>,
    opening_year: u32,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Staff {
    let nationality = if def.nationality.is_empty() { "ENG" } else { def.nationality.as_str() };
    let first_name = if def.first_name.is_empty() {
        let (f, _) = pick_name_from_def(nationality, names_def, rng);
        f
    } else {
        def.first_name.clone()
    };
    let last_name = if def.last_name.is_empty() {
        let (_, l) = pick_name_from_def(nationality, names_def, rng);
        l
    } else {
        def.last_name.clone()
    };

    let current_year: u32 = opening_year;
    let birth_year = if let Some(dob) = &def.date_of_birth {
        dob.split('-').next().and_then(|y| y.parse::<u32>().ok()).unwrap_or(current_year.saturating_sub(40))
    } else if let Some(age) = def.age {
        current_year.saturating_sub(age)
    } else {
        current_year.saturating_sub(rng.random_range(30..55))
    };
    let dob = def.date_of_birth.clone()
        .unwrap_or_else(|| format!("{birth_year:04}-01-01"));

    let attributes = def.attributes.clone().unwrap_or_else(|| {
        generate_random_staff_from_def(
            team_id.unwrap_or(""),
            def.role.clone(),
            nationality,
            opening_year,
            names_def,
            rng,
        )
        .attributes
    });

    let id = if def.id.is_empty() { Uuid::new_v4().to_string() } else { def.id.clone() };
    let mut s = Staff::new(id, first_name, last_name, dob, def.role.clone(), attributes);
    s.nationality = nationality.to_string();
    s.team_id = team_id.map(|t| t.to_string());
    s.specialization = def.specialization.clone();
    s
}

// ---------------------------------------------------------------------------
// Authored players (world packages)
// ---------------------------------------------------------------------------

fn jitter(base: i32, spread: i32, lo: u8, hi: u8, rng: &mut impl Rng) -> u8 {
    (base + rng.random_range(-spread..=spread)).clamp(lo as i32, hi as i32) as u8
}

/// Build a realistic attribute spread centred on a target `overall`, shaped by
/// position so a goalkeeper's keeping attributes and a defender's defending sit
/// high. Used when a hand-authored player gives an `overall` rather than a full
/// `attributes` block; the resulting position-weighted OVR lands near `overall`.
pub(super) fn attributes_for_overall(
    overall: u8,
    position: &Position,
    rng: &mut impl Rng,
) -> PlayerAttributes {
    let base = overall as i32;
    let group = position.to_group_position();
    let is_gk = matches!(group, Position::Goalkeeper);
    let is_def = matches!(group, Position::Defender);
    let is_fwd = matches!(group, Position::Forward);

    PlayerAttributes {
        pace: jitter(base, 8, 30, 97, rng),
        stamina: jitter(base, 8, 30, 97, rng),
        strength: jitter(base, 8, 30, 97, rng),
        agility: jitter(base, 8, 30, 97, rng),
        passing: jitter(base, 8, 30, 97, rng),
        shooting: if is_gk {
            rng.random_range(20..50)
        } else {
            jitter(base, 8, 30, 97, rng)
        },
        tackling: if is_gk || is_fwd {
            jitter(base - 15, 8, 20, 80, rng)
        } else {
            jitter(base, 8, 30, 97, rng)
        },
        dribbling: if is_gk {
            rng.random_range(20..50)
        } else {
            jitter(base, 8, 30, 97, rng)
        },
        defending: if is_gk {
            rng.random_range(25..55)
        } else if is_def {
            jitter(base + 3, 6, 40, 97, rng)
        } else {
            jitter(base, 8, 30, 97, rng)
        },
        positioning: jitter(base, 8, 30, 97, rng),
        vision: jitter(base, 8, 30, 97, rng),
        decisions: jitter(base, 8, 30, 97, rng),
        composure: jitter(base, 8, 30, 97, rng),
        aggression: jitter(base - 10, 10, 30, 90, rng),
        teamwork: jitter(base, 8, 40, 97, rng),
        leadership: jitter(base - 10, 12, 25, 90, rng),
        handling: if is_gk {
            jitter(base, 8, 40, 97, rng)
        } else {
            rng.random_range(10..35)
        },
        reflexes: if is_gk {
            jitter(base, 8, 40, 97, rng)
        } else {
            rng.random_range(20..50)
        },
        aerial: if is_gk {
            jitter(base, 8, 40, 97, rng)
        } else if is_def {
            jitter(base, 8, 40, 95, rng)
        } else {
            jitter(base - 10, 10, 30, 80, rng)
        },
    }
}

fn resolve_def_name(
    def: &super::package::PlayerDef,
    nationality: &str,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> (String, String) {
    if !def.first_name.is_empty() || !def.last_name.is_empty() {
        (def.first_name.clone(), def.last_name.clone())
    } else if !def.name.is_empty() {
        let mut parts = def.name.splitn(2, ' ');
        let first = parts.next().unwrap_or("").to_string();
        let last = parts.next().unwrap_or("").to_string();
        (first, last)
    } else {
        pick_name_from_def(nationality, names_def, rng)
    }
}

fn resolve_birth_year(
    def: &super::package::PlayerDef,
    opening_year: u32,
    rng: &mut impl Rng,
) -> u32 {
    if let Some(dob) = &def.date_of_birth {
        dob.get(0..4)
            .and_then(|year| year.parse::<u32>().ok())
            .unwrap_or(opening_year.saturating_sub(24))
    } else if let Some(age) = def.age {
        opening_year.saturating_sub(age)
    } else {
        opening_year.saturating_sub(rng.random_range(18..34))
    }
}

/// Convert a hand-authored [`PlayerDef`](super::package::PlayerDef) into a full
/// player for `team_id`. Ability comes from an explicit `attributes` block or is
/// generated around `overall`; identity falls back to the name pools when not
/// given.
pub(super) fn generate_player_from_def(
    def: &super::package::PlayerDef,
    team_id: &str,
    opening_year: u32,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Player {
    let nationality = canonicalize_generated_nationality(&def.nationality);
    let (first_name, last_name) = resolve_def_name(def, &nationality, names_def, rng);
    let full_name = format!("{first_name} {last_name}").trim().to_string();
    let match_name = if last_name.is_empty() {
        full_name.clone()
    } else {
        last_name
    };

    let current_year: u32 = opening_year;
    let birth_year = resolve_birth_year(def, opening_year, rng);
    let dob = def
        .date_of_birth
        .clone()
        .unwrap_or_else(|| format!("{birth_year:04}-01-01"));
    let age = current_year.saturating_sub(birth_year);

    let attributes = def
        .attributes
        .clone()
        .unwrap_or_else(|| attributes_for_overall(def.overall.unwrap_or(65), &def.position, rng));

    let approx_ovr =
        crate::player_rating::ovr_from_attributes(&attributes, &def.position).round() as u32;
    let age_factor = if age <= 23 {
        1.5
    } else if age <= 28 {
        1.2
    } else if age <= 32 {
        0.8
    } else {
        0.4
    };
    let market_value = ((approx_ovr as f64).powi(2) * 500.0 * age_factor) as u64;
    let wage = (market_value / 200).max(500) as u32;
    let contract_years = if age <= 27 {
        rng.random_range(2..6)
    } else {
        rng.random_range(1..4)
    };
    let contract_end = format!("{}-06-30", opening_year.saturating_add(contract_years));

    let id = if def.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        def.id.clone()
    };
    let mut player = Player::new(
        id,
        match_name,
        full_name,
        dob,
        nationality,
        def.position.clone(),
        attributes,
    );
    player.team_id = Some(team_id.to_string());
    // An authored photo is write-only metadata unless it is copied onto the
    // domain player: `PlayerAvatar` renders `media.face`, and the procedural
    // portrait generator stands down whenever it is set.
    player.media.face = def
        .photo
        .as_ref()
        .map(|photo| photo.trim().to_string())
        .filter(|photo| !photo.is_empty());
    player.market_value = market_value;
    player.wage = wage;
    player.contract_end = Some(contract_end);
    player.condition = rng.random_range(75..100);
    player.morale = rng.random_range(40..76);
    if let Some(ref foot_str) = def.footedness {
        player.footedness = match foot_str.as_str() {
            "Left" => domain::player::Footedness::Left,
            "Both" => domain::player::Footedness::Both,
            _ => domain::player::Footedness::Right,
        };
    }
    if def.youth {
        player.squad_role = domain::player::SquadRole::Youth;
    }

    let temp_ovr = {
        use crate::player_rating::natural_ovr;
        natural_ovr(&player).round() as u8
    };
    player.potential = generate_potential(temp_ovr, age);
    refresh_player_derived(&mut player, current_year);
    player
}

/// Generate a random unemployed manager (no team) using the provided name pool.
/// Used to top up the unemployed manager market when below the seasonal floor.
pub(super) fn generate_random_unemployed_manager(
    nationality: &str,
    names_def: &NamesDefinition,
    current_year: u32,
    rng: &mut impl Rng,
) -> domain::manager::Manager {
    let (first_name, last_name) = pick_name_from_def(nationality, names_def, rng);
    let age: u32 = rng.random_range(35..65);
    let birth_year = current_year.saturating_sub(age);
    let dob = format!(
        "{:04}-{:02}-{:02}",
        birth_year,
        rng.random_range(1u32..13u32),
        rng.random_range(1u32..29u32)
    );
    let reputation = rng.random_range(200u32..=700u32);

    let mut mgr = domain::manager::Manager::new(
        Uuid::new_v4().to_string(),
        first_name,
        last_name,
        dob,
        nationality.to_string(),
    );
    mgr.reputation = reputation;
    mgr.satisfaction = 50;
    mgr.fan_approval = 50;
    mgr
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #453: `country_to_iso` recognised 17 country names and answered `"ENG"`
    /// for everything else, so a package with `"country": "Japan"` filled 60% of
    /// every Japanese club with English players — silently.
    #[test]
    fn a_country_name_outside_the_old_hardcoded_list_resolves_to_its_own_code() {
        for (name, expected) in [
            ("Japan", "JP"),
            ("Nigeria", "NG"),
            ("Poland", "PL"),
            ("Mexico", "MX"),
            ("Serbia", "RS"),
        ] {
            assert_eq!(
                resolve_nationality_code(name).as_deref(),
                Some(expected),
                "{name} should resolve to {expected}"
            );
        }
    }

    #[test]
    fn the_country_names_the_old_list_did_know_still_resolve() {
        for (name, expected) in [
            ("England", "ENG"),
            ("Scotland", "SCO"),
            ("Republic of Ireland", "IE"),
            ("Brazil", "BR"),
            ("Sweden", "SE"),
        ] {
            assert_eq!(resolve_nationality_code(name).as_deref(), Some(expected));
        }
    }

    #[test]
    fn a_code_resolves_to_itself() {
        for code in ["JP", "BR", "ENG", "NIR", "GW"] {
            assert_eq!(resolve_nationality_code(code).as_deref(), Some(code));
        }
    }

    /// A package may declare its own country with a slug id (`scaffold` emits
    /// one for any country outside the catalog). That is unresolvable — and the
    /// answer must be "unknown", not a real, specific, wrong nationality.
    #[test]
    fn an_unresolvable_country_is_unknown_rather_than_england() {
        for unknown in ["atlantis", "Fictland", "", "   "] {
            assert_eq!(
                resolve_nationality_code(unknown),
                None,
                "{unknown:?} must not resolve to a real nation"
            );
        }
    }

    /// The length heuristic passed any 2–3 character string straight through as
    /// though it were a code — a different wrong answer for a different input.
    #[test]
    fn a_short_string_that_is_not_a_nation_code_is_not_treated_as_one() {
        assert_eq!(resolve_nationality_code("zz"), None);
        assert_eq!(resolve_nationality_code("xyz"), None);
    }

    #[test]
    fn an_unresolvable_club_country_never_produces_an_english_squad() {
        let mut rng = rand::rng();
        let pool = nationality_distribution();

        let drawn: Vec<String> = (0..400)
            .map(|_| pick_nationality_from_def("atlantis", pool, &mut rng))
            .collect();
        let english = drawn.iter().filter(|code| *code == "ENG").count();

        // With the old code every one of these was ENG. Drawn from the whole
        // distribution, England is one nation among 211 — a handful is fine, a
        // majority is the bug.
        assert!(
            english < drawn.len() / 4,
            "{english}/{} drawn as English for an unresolvable country",
            drawn.len()
        );
    }

    /// #452: the draw used to be over the 17 name-pool keys, so a world could
    /// only ever contain ~16 nationalities however many countries it held.
    #[test]
    fn the_nationality_distribution_spans_the_whole_catalog() {
        let pool = nationality_distribution();
        let distinct: std::collections::HashSet<&String> = pool.iter().collect();

        assert!(
            distinct.len() > 200,
            "the draw should span the catalog, got {} nationalities",
            distinct.len()
        );
        for code in ["JP", "NG", "PL", "MX", "RS"] {
            assert!(
                distinct.contains(&code.to_string()),
                "{code} should be drawable"
            );
        }
    }

    /// Weighted, not uniform — the issue is explicit that "uniform across all
    /// nations would be as wrong as today's behaviour, just differently".
    #[test]
    fn stronger_footballing_nations_are_drawn_more_often() {
        let pool = nationality_distribution();
        let count = |code: &str| pool.iter().filter(|entry| *entry == code).count();

        // Top of its region beats a lower-ranked neighbour, which beats a
        // merely-selectable nation.
        assert!(count("FR") > count("PL"), "FR {} vs PL {}", count("FR"), count("PL"));
        assert!(count("PL") > count("AD"), "PL {} vs AD {}", count("PL"), count("AD"));
        assert!(count("BR") > count("BO"), "BR {} vs BO {}", count("BR"), count("BO"));
    }

    /// The pool is indexed with the RNG, so its order has to be stable or the
    /// same seed stops producing the same world.
    #[test]
    fn the_distribution_is_stable_across_calls() {
        assert_eq!(nationality_distribution(), nationality_distribution());
    }
}
