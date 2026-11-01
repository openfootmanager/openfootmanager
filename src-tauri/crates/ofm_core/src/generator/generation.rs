use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::PlayStyle;
use rand::{Rng, RngExt};

use super::authored_player::{is_valid_date_of_birth, resolve_authored_contract};
use super::definitions::{NamePool, NamesDefinition};
use crate::nations;
use crate::player_rating::{
    generate_potential, move_attributes_to, natural_ovr, ovr_from_attributes,
    refresh_player_derived, roll_growth_headroom,
};

pub(super) fn standard_available_staff_roles() -> [StaffRole; 12] {
    [
        StaffRole::Coach,
        StaffRole::Scout,
        StaffRole::Physio,
        StaffRole::Coach,
        StaffRole::AssistantManager,
        StaffRole::Scout,
        StaffRole::Physio,
        StaffRole::Coach,
        StaffRole::Coach,
        StaffRole::Physio,
        StaffRole::Scout,
        StaffRole::AssistantManager,
    ]
}

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
    /// Share of a squad drawn from the club's own country.
    const LOCAL_SHARE_PERCENT: u32 = 60;

    // The catalog first, then the world's own list. A package may declare
    // countries the catalog has never heard of — that is the point of authoring
    // one — and for a club in such a country the id *is* the nationality. Only a
    // country neither the catalog nor this world recognises is unresolvable.
    let local = resolve_nationality_code(team_country)
        .or_else(|| declared_code(team_country, available_codes));

    // Nothing to draw from: the club's own country is the only answer available,
    // and when that is unknown too there is genuinely none to give.
    if available_codes.is_empty() {
        return canonicalize_generated_nationality(local.as_deref().unwrap_or_default());
    }

    // An unresolvable country skips the local roll entirely rather than losing
    // it — the whole draw comes from the wider pool.
    let selected = match local {
        Some(code) if rng.random_range(0..100) < LOCAL_SHARE_PERCENT => code,
        _ => available_codes[rng.random_range(0..available_codes.len())].clone(),
    };

    canonicalize_generated_nationality(&selected)
}

/// Match a club's country against the nationalities this world actually draws
/// from, for countries the shipped catalog does not contain.
///
/// `available_codes` is the world's own nationality list, so a package country
/// reaches here only if [`super::build_world_data_from_package`] put it there —
/// which it does for every country the package declares. Comparing against that
/// list rather than accepting any unknown string keeps an outright typo
/// unresolvable, which is what #453 was about.
fn declared_code(team_country: &str, available_codes: &[String]) -> Option<String> {
    let trimmed = team_country.trim();
    if trimmed.is_empty() {
        return None;
    }
    available_codes
        .iter()
        .find(|code| code.eq_ignore_ascii_case(trimmed))
        .cloned()
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
/// An id for something being generated, drawn from the generator it was made with rather
/// than minted at random: a world generated from a seed is then the same world, ids and all,
/// and what keys off those ids (a match's dice, a player's growth) replays too. It has the
/// shape of a v4 UUID, so everything that treats ids as opaque strings is unaffected.
pub(super) fn seeded_id(rng: &mut impl Rng) -> String {
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    uuid::Builder::from_random_bytes(bytes)
        .into_uuid()
        .to_string()
}

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
/// Weight has two factors, because a nation's standing has two parts and the
/// catalog only records one of them.
///
/// *Rank within region* comes from [`nations::NATION_CATALOG`], already
/// documented as "strongest footballing traditions first within each region" —
/// a signal the codebase maintains anyway.
///
/// *Region depth* has to be declared, and [`REGION_WEIGHT`] declares it. Rank
/// alone is a **rank among unequal fields**: it makes the top of every region
/// equal, so Costa Rica draws as often as Brazil and France, New Zealand
/// outranks Spain, and a world ends up with more Central American players than
/// South American ones. Multiplying restores the comparison the catalog cannot
/// express, and keeps the ordering rank already gets right inside a region.
///
/// [`nations::ADDITIONAL_NATIONS`] form a flat low-weight tail: reachable, but
/// rare, and deliberately not region-scaled — they are outside the World Cup
/// pool, which is the only claim being made about them.
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
            let within_region = TOP_WEIGHT.saturating_sub(*rank).max(CATALOG_FLOOR);
            let weight = within_region * region_weight(nation.region_id);
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

/// The catalog distribution, plus the countries a world declares for itself.
///
/// A package country is absent from both catalogs by definition, so the static
/// pool cannot contain it and a club in one used to draw its whole squad from
/// elsewhere. Declared countries are appended at [`DECLARED_WEIGHT`] — heavy
/// enough that they actually appear in neighbouring squads, light enough that
/// declaring a handful does not drown out the rest of the world.
///
/// A declared id that *is* already a catalog code (a package covering real
/// nations, the common case) is skipped rather than added twice: it is already
/// in the pool at its proper weight, and appending would quietly promote it.
pub(super) fn nationality_distribution_including<'a>(
    declared: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    /// Pool entries per declared country. Matches a mid-table catalog nation:
    /// present and drawable, not dominant.
    const DECLARED_WEIGHT: usize = 12;

    let mut pool = nationality_distribution().clone();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in declared {
        let code = canonicalize_generated_nationality(id);
        if code.is_empty() || nations::nation_by_code(&code).is_some() || !seen.insert(code.clone())
        {
            continue;
        }
        for _ in 0..DECLARED_WEIGHT {
            pool.push(code.clone());
        }
    }
    pool
}

/// How much of the world's football a region accounts for, as a multiplier on
/// rank within it.
///
/// Hand-authored on purpose: no existing field carries it. `NationDef` records
/// a code, a name and a region, and the catalog's ordering is explicitly
/// *within* each region — so there is nothing to derive a cross-region
/// comparison from, and inventing one from list length would say only that
/// Africa has many federations, not that it supplies many players.
///
/// The numbers are deliberately coarse. They are a first approximation of where
/// professional players come from, not a ranking of national teams, and they
/// should be tuned against generated worlds rather than argued about in the
/// abstract.
fn region_weight(region_id: &str) -> usize {
    match region_id {
        "europe" => 6,
        "south-america" => 5,
        "africa" => 3,
        "asia" => 2,
        "north-america" | "central-america" => 2,
        "oceania" => 1,
        // `region_for_code` defaults unknown codes to europe, so an unrecognised
        // region here means the catalog gained one this table has not been told
        // about. Weight it as a modest region rather than silently as Europe.
        _ => 2,
    }
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

/// The squad floor lives in `squad_floor`; trimming generated players off an
/// authored squad must never take a group below it — a club with no goalkeeper
/// is unplayable.
pub(super) use crate::squad_floor::MIN_PLAYERS_PER_GROUP;

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

/// The position group a generated squad slot holds: GK 0-1, DEF 2-8, MID 9-15,
/// FWD 16-21.
pub(super) fn position_for_slot(index: usize) -> Position {
    if index < 2 {
        Position::Goalkeeper
    } else if index < 9 {
        Position::Defender
    } else if index < 16 {
        Position::Midfielder
    } else {
        Position::Forward
    }
}

/// The first squad slot that generates a senior player of this group.
pub(super) fn senior_slot_for(group: &Position) -> usize {
    (0..SQUAD_SLOTS)
        .find(|slot| position_for_slot(*slot) == *group && !is_youth_reserved_slot(*slot))
        .unwrap_or(0)
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

/// The mean rating of the attributes the generator rolls.
const ROLLED_RATING_MEAN: f64 = 67.6;
/// Where the generated ceilings centre. Young players open below their ceiling, so the ceilings
/// sit above the mean rating a world should open at (about 68).
///
/// 70.1 since the ceiling follows a falling player's rating (#869): that rule takes the room
/// veterans used to train into, and S by rating settled at 0.8 to 0.9% of seniors, under its 1%
/// floor. +0.5 on the centre (it was 69.6) brings the shipped world back to S 1.0 to 1.8% and A 5.9
/// to 8.2% in every one of 20 seasons on two seeds. At the opening of a new world that raises S by
/// rating from 1.0% to 1.2%, A from 5.2% to 6.5% and the mean senior rating from 67.7 to 68.2;
/// wonderkids stay at 0.3%.
const CEILING_MEAN: f64 = 70.1;
/// How much of the rolled spread the ceilings keep. Lifting the centre alone would push the S tier
/// past its 1 to 3% target, so the bell is narrowed as it is lifted.
const CEILING_SPREAD: f64 = 0.80;

/// The spread (standard deviation) of the ceilings a world holds at 25, once every ceiling has
/// stopped drifting.
const CEILING_SD: f64 = 5.0;

/// The ceiling a rolled rating becomes for a player of `age`: the same ranking, on the bell the
/// world is meant to have at 25. A young player's ceiling will still drift (`development`), so he
/// is drawn from a bell narrower by the variance that drift will add; by 25 everyone's ceiling
/// follows one bell, and a teenager's ceiling stays a good guide to where he ends up.
///
/// The centre is not raised for the drift's small pull down: a player closes the gap to his
/// ceiling by 19 to 21, after which his rating holds the ceiling up, so the pull is mostly absorbed
/// (measured on the shipped world: ceilings of 72.2 at 16 are 72.4 at 25).
fn shaped_ceiling(rolled: f64, age: u32) -> u8 {
    let (_, drift_variance) = crate::development::ceiling_drift_still_to_come(age);
    let narrowing = (1.0 - drift_variance / CEILING_SD.powi(2)).max(0.0).sqrt();
    (CEILING_MEAN + narrowing * CEILING_SPREAD * (rolled - ROLLED_RATING_MEAN))
        .round()
        .clamp(1.0, 99.0) as u8
}

/// A generated player for squad slot `index`. `age` fixes his age; `None`
/// draws it from the slot (17–21 for a youth-reserved slot, 17–35 otherwise),
/// exactly as before the parameter existed, so every seeded world — all of
/// which pass `None` — is unchanged by it.
pub(super) fn generate_random_player_from_def(
    team_id: &str,
    index: usize,
    nationality: &str,
    opening_year: u32,
    age: Option<u32>,
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Player {
    let (first_name, last_name) = pick_name_from_def(nationality, names_def, rng);
    let full_name = format!("{} {}", first_name, last_name);
    let match_name = last_name.clone();

    let position = position_for_slot(index);

    let p_id = seeded_id(rng);
    let nationality = nationality.to_string();

    // Reserve one slot per position group (GK + back line + midfield + attack) as
    // youth-aged so scouted youth recruits land at a consistent age across positions
    // and clubs can open with real academy prospects instead of an empty youth squad.
    let age = age.unwrap_or_else(|| {
        if is_youth_reserved_slot(index) {
            rng.random_range(17..22)
        } else {
            rng.random_range(17..36)
        }
    });
    let birth_year = opening_year.saturating_sub(age);
    let birth_month = rng.random_range(1..13);
    let birth_day = rng.random_range(1..29);
    let dob = format!("{:04}-{:02}-{:02}", birth_year, birth_month, birth_day);

    let group = position.to_group_position();
    let is_gk = matches!(group, Position::Goalkeeper);
    let is_def = matches!(group, Position::Defender);
    let is_fwd = matches!(group, Position::Forward);

    let mut attributes = PlayerAttributes {
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

    // The attributes just rolled are the player at his best: they set his ceiling, and a player
    // who has years of growth left is lowered from it by the headroom for his age. Ceiling
    // first, rating second, so the whole world's ceilings follow one bell and a teenager is an
    // earlier version of a prime-age player rather than a better one.
    let ceiling = shaped_ceiling(ovr_from_attributes(&attributes, &position), age);
    let headroom = roll_growth_headroom(age, rng);
    move_attributes_to(&mut attributes, &position, ceiling.saturating_sub(headroom));
    // Size market value and wage from the same position-weighted rating the
    // player will be shown with, so a keeper is priced on keeping.
    let current_year: u32 = opening_year;

    let approx_ovr = ovr_from_attributes(&attributes, &position).round() as u32;

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
    let wage = (market_value / 200).max(crate::contracts::MINIMUM_DEFAULT_WAGE) as u32;
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
    player.stage_wage(wage);
    player.stage_contract_end(Some(contract_end));
    player.condition = rng.random_range(75..100);
    player.morale = rng.random_range(40..76);

    // ~40% of outfield players get an alternate position based on attributes
    if !is_gk && rng.random_range(0..5) < 2 {
        let alt = compute_alternate_position(&player.position, &player.attributes);
        if let Some(pos) = alt {
            player.alternate_positions.push(pos);
        }
    }

    // Set position-weighted OVR and traits (Wonderkid included if applicable) around the ceiling.
    player.potential = (natural_ovr(&player).round() as u8)
        .saturating_add(headroom)
        .min(99);
    refresh_player_derived(
        &mut player,
        domain::age::season_midpoint(current_year as i32),
    );

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

    let mut s = Staff::new(seeded_id(rng), first_name, last_name, dob, role, attributes);
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

    let mut s = Staff::new(seeded_id(rng), first_name, last_name, dob, role, attributes);
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
    let nationality = if def.nationality.is_empty() {
        "ENG"
    } else {
        def.nationality.as_str()
    };
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
    // Validation refuses an unreadable date of birth; one that reaches here anyway counts as
    // not written, never as a guessed year.
    let authored_dob = def
        .date_of_birth
        .as_deref()
        .filter(|dob| is_valid_date_of_birth(dob));
    let birth_year = if let Some(year) = authored_dob.and_then(birth_year_of) {
        year
    } else if let Some(age) = def.age {
        current_year.saturating_sub(age)
    } else {
        current_year.saturating_sub(rng.random_range(30..55))
    };
    let dob = authored_dob
        .map(str::to_string)
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

    let id = if def.id.is_empty() {
        seeded_id(rng)
    } else {
        def.id.clone()
    };
    let mut s = Staff::new(id, first_name, last_name, dob, def.role.clone(), attributes);
    s.nationality = nationality.to_string();
    s.team_id = team_id.map(|t| t.to_string());
    s.specialization = def.specialization.clone();
    s.set_known_as(def.known_as.clone());
    s
}

// ---------------------------------------------------------------------------
// Authored players (world packages)
// ---------------------------------------------------------------------------

/// `base` plus or minus `spread`, kept within `lo..=hi`. The floor gives way to a base so low
/// that the spread reaches under it, but never below 1: an attribute of 0 reads as missing.
fn jitter(base: i32, spread: i32, lo: u8, hi: u8, rng: &mut impl Rng) -> u8 {
    let lo = i32::from(lo).min((base - spread).max(1));
    (base + rng.random_range(-spread..=spread)).clamp(lo, i32::from(hi)) as u8
}

/// How many times the attribute centre is corrected against the real rating.
const OVERALL_CORRECTION_STEPS: usize = 8;

/// Attributes whose rating, as `player_rating::ovr_from_attributes` computes it, lands on
/// `overall`.
///
/// Jittering every attribute around `overall` does not give an `ovr` of `overall`: the rating
/// subtracts a penalty for a weak critical attribute, so a low target came out far from itself.
/// Rather than invert that formula here (a second copy of the rating rule), the centre is
/// moved by the remaining difference, computed with the real rating, until it lands within one
/// point. The same seed is replayed each time so only the centre moves, and the result is
/// deterministic for a seeded `rng`.
fn attributes_matching_overall(
    overall: u8,
    position: &Position,
    rng: &mut impl Rng,
) -> PlayerAttributes {
    let seed: u64 = rng.random();
    let target = i32::from(overall);
    let mut centre = target;
    let mut closest: Option<(i32, PlayerAttributes)> = None;
    for _ in 0..OVERALL_CORRECTION_STEPS {
        let mut replay = crate::seed::rng_from_u64(seed);
        let attributes = attributes_for_overall(centre.clamp(1, 99) as u8, position, &mut replay);
        let rated = crate::player_rating::ovr_from_attributes(&attributes, position).round() as i32;
        let off = target - rated;
        if closest.as_ref().is_none_or(|(best, _)| off.abs() < *best) {
            closest = Some((off.abs(), attributes));
        }
        if off.abs() <= 1 {
            break;
        }
        centre += off;
    }
    closest
        .map(|(_, attributes)| attributes)
        .unwrap_or_else(|| {
            attributes_for_overall(overall, position, &mut crate::seed::rng_from_u64(seed))
        })
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

/// The year of a `YYYY-MM-DD` date of birth.
fn birth_year_of(dob: &str) -> Option<u32> {
    dob.get(0..4)?.parse().ok()
}

/// The year an authored player was born and the date of birth to store: the authored date when
/// it is a real one, else the authored age, else a rolled one. An unreadable date is refused by
/// package validation; one that reaches here anyway counts as not written, never as a guess.
fn resolve_birth(
    def: &super::package::PlayerDef,
    opening_year: u32,
    rng: &mut impl Rng,
) -> (u32, String) {
    let authored = def
        .date_of_birth
        .as_deref()
        .filter(|dob| is_valid_date_of_birth(dob));
    if let Some((dob, year)) = authored.and_then(|dob| Some((dob, birth_year_of(dob)?))) {
        return (year, dob.to_string());
    }
    let year = match def.age {
        Some(age) => opening_year.saturating_sub(age),
        None => opening_year.saturating_sub(rng.random_range(18..34)),
    };
    (year, format!("{year:04}-01-01"))
}

/// Convert a hand-authored [`PlayerDef`](super::package::PlayerDef) into a full
/// player for `team_id`. Ability comes from an explicit `attributes` block or is
/// generated around `overall`; identity falls back to the name pools when not
/// given.
/// The ability an authored player gets when they declare neither `overall` nor
/// an `attributes` block.
///
/// Shared with `package`'s potential validation on purpose: that check has to
/// compare against the ability generation will actually use, and a second copy
/// of this number would let a package validate and then be generated against a
/// different floor.
pub(super) const DEFAULT_AUTHORED_OVERALL: u8 = 65;

/// Pull an engine-invented attribute spread down until it sits at or below an
/// authored ceiling.
///
/// `attributes_for_overall` jitters around its target and clamps every attribute
/// to a floor of 30, so the ovr it produces lands *near* the authored `overall`
/// rather than on it — and below 30 it cannot land there at all. Since
/// `refresh_player_derived` floors potential at ovr, that quietly raised a
/// ceiling the author had written: measured at 26-32% for mid-range abilities,
/// and every single time below the clamp floor.
///
/// The rule this keeps: **the engine may bound what the engine invented, and
/// never touches what the author wrote.** It is applied only to a spread
/// synthesized from `overall`; an explicit `attributes` block is left exactly as
/// written, and a ceiling below what that block is worth is reported by package
/// validation instead.
///
/// Converges in a pass or two — ovr is a weighted mean of the attributes minus a
/// penalty, so subtracting the overshoot from every attribute overshoots
/// downward at worst. The iteration cap is a belt-and-braces stop, not a
/// expectation of slow convergence.
fn bound_attributes_by_ceiling(
    attributes: &mut PlayerAttributes,
    position: &Position,
    ceiling: u8,
) {
    for _ in 0..4 {
        let ovr = crate::player_rating::ovr_from_attributes(attributes, position).round() as u8;
        let Some(overshoot) = ovr.checked_sub(ceiling).filter(|delta| *delta > 0) else {
            return;
        };
        for slot in [
            &mut attributes.pace,
            &mut attributes.stamina,
            &mut attributes.strength,
            &mut attributes.agility,
            &mut attributes.passing,
            &mut attributes.shooting,
            &mut attributes.tackling,
            &mut attributes.dribbling,
            &mut attributes.defending,
            &mut attributes.positioning,
            &mut attributes.vision,
            &mut attributes.decisions,
            &mut attributes.composure,
            &mut attributes.aggression,
            &mut attributes.teamwork,
            &mut attributes.leadership,
            &mut attributes.handling,
            &mut attributes.reflexes,
            &mut attributes.aerial,
        ] {
            *slot = slot.saturating_sub(overshoot).max(1);
        }
    }
}

/// What the player is called on the pitch.
///
/// The authored `name` without the leading words it shares with the first name, so "Alfredo Di
/// Stefano" is "Di Stefano" and "Nicolás de la Cruz" keeps its particles; a `name` that shares
/// none ("Pelé", "Kaká") is the whole `name`. With no `name`, the last name (or the full name
/// when there is none), as before. `firstName`/`lastName` stay the real name.
fn authored_match_name(name: &str, first_name: &str, last_name: &str, full_name: &str) -> String {
    let name_words: Vec<&str> = name.split_whitespace().collect();
    if name_words.is_empty() {
        return if last_name.is_empty() {
            full_name.to_string()
        } else {
            last_name.to_string()
        };
    }
    let shared = name_words
        .iter()
        .zip(first_name.split_whitespace())
        .take_while(|(called, real)| called.to_lowercase() == real.to_lowercase())
        .count();
    // A name that is only the first name stays whole rather than becoming empty.
    if shared == 0 || shared == name_words.len() {
        name_words.join(" ")
    } else {
        name_words[shared..].join(" ")
    }
}

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
    let match_name = authored_match_name(&def.name, &first_name, &last_name, &full_name);

    let current_year: u32 = opening_year;
    let (birth_year, dob) = resolve_birth(def, opening_year, rng);
    let age = current_year.saturating_sub(birth_year);

    let mut attributes = def.attributes.clone().unwrap_or_else(|| {
        attributes_matching_overall(
            def.overall.unwrap_or(DEFAULT_AUTHORED_OVERALL),
            &def.position,
            rng,
        )
    });

    // Only a spread this function invented, and only when a ceiling was authored.
    // Bounding it here rather than after `Player::new` keeps market value and
    // wage sized from the attributes the player actually ends up with.
    if let (Some(ceiling), None) = (def.potential, def.attributes.as_ref()) {
        bound_attributes_by_ceiling(&mut attributes, &def.position, ceiling);
    }

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
    let generated_value = ((approx_ovr as f64).powi(2) * 500.0 * age_factor) as u64;
    // An authored figure wins. An omitted wage is sized from the value the player
    // ends up with rather than the one the author replaced, and saturates because an
    // authored value can be far larger than any generated one.
    let market_value = def.value.unwrap_or(generated_value);
    let wage = def.wage.unwrap_or_else(|| {
        u32::try_from((market_value / 200).max(crate::contracts::MINIMUM_DEFAULT_WAGE))
            .unwrap_or(u32::MAX)
    });
    // Rolled whether or not a contract was authored, so a package that authors none
    // draws exactly the random numbers it always did and generates the same players.
    let contract_years = if age <= 27 {
        rng.random_range(2..6)
    } else {
        rng.random_range(1..4)
    };
    let generated_contract_end = format!("{}-06-30", opening_year.saturating_add(contract_years));
    // Validation is what tells an author their contract cannot be resolved. Generation
    // never panics on one and falls back to the roll, as it would for no contract.
    let authored_contract =
        resolve_authored_contract(def, opening_year, contract_years).unwrap_or_default();
    let contract_date = |date: chrono::NaiveDate| date.format("%Y-%m-%d").to_string();

    let id = if def.id.is_empty() {
        seeded_id(rng)
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
    player.stage_wage(wage);
    player.stage_contract_start(authored_contract.start.map(contract_date));
    player.stage_contract_end(Some(
        authored_contract
            .end
            .map(contract_date)
            .unwrap_or(generated_contract_end),
    ));
    player.condition = def.condition.unwrap_or_else(|| rng.random_range(75..100));
    player.morale = def.morale.unwrap_or_else(|| rng.random_range(40..76));
    if let Some(ref foot_str) = def.footedness {
        player.footedness = match foot_str.as_str() {
            "Left" => domain::player::Footedness::Left,
            "Both" => domain::player::Footedness::Both,
            _ => domain::player::Footedness::Right,
        };
    }
    if let Some(weak_foot) = def.weak_foot {
        player.weak_foot = weak_foot;
    }
    if !def.alternate_positions.is_empty() {
        player.alternate_positions = def.alternate_positions.clone();
    }
    // A club the package does not define has no id, so the domain's empty-string
    // convention for "no team" stands in; the name is what the profile shows.
    player.career = def
        .career_history
        .iter()
        .map(|entry| domain::player::CareerEntry {
            season: entry.season,
            team_id: entry.team_id.clone().unwrap_or_default(),
            team_name: entry.team_name.clone(),
            appearances: entry.appearances,
            goals: entry.goals,
            assists: entry.assists,
        })
        .collect();
    if def.youth {
        player.squad_role = domain::player::SquadRole::Youth;
    }

    let temp_ovr = {
        use crate::player_rating::natural_ovr;
        natural_ovr(&player).round() as u8
    };
    // An authored ceiling is the author's to set; only roll one when they did
    // not. `refresh_player_derived` then leaves a non-zero potential alone,
    // floored at current ovr — so nothing further is needed to make it stick.
    player.potential = def
        .potential
        .unwrap_or_else(|| generate_potential(temp_ovr, age, rng));
    refresh_player_derived(
        &mut player,
        domain::age::season_midpoint(current_year as i32),
    );
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
        seeded_id(rng),
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

    /// Build a `PlayerDef` from JSON rather than a struct literal.
    ///
    /// Deliberate: `potential` is new, so a struct literal naming it would not
    /// compile on a tree that predates the field — and this project requires a
    /// regression test to be *run* against the unfixed code. `PlayerDef` has no
    /// `deny_unknown_fields`, so the old tree ignores the key and these tests
    /// fail on behaviour rather than failing to build.
    fn player_def_from_json(value: serde_json::Value) -> super::super::package::PlayerDef {
        serde_json::from_value(value).expect("the fixture deserializes")
    }

    fn generate_from_json(value: serde_json::Value, opening_year: u32) -> Player {
        let names_def = super::super::definitions::default_names_definition();
        let mut rng = rand::rng();
        generate_player_from_def(
            &player_def_from_json(value),
            "club-id",
            opening_year,
            &names_def,
            &mut rng,
        )
    }

    /// A fixed-seed league's worth of generated players, 23 slots a club, with the age each was
    /// generated at (the opening year less the year of birth, which is what the generator sets
    /// its headroom by).
    fn generated_players(clubs: usize) -> Vec<(Player, i32)> {
        use rand::SeedableRng;
        let names_def = super::super::definitions::default_names_definition();
        let mut rng = rand::rngs::StdRng::seed_from_u64(869);
        (0..clubs * 23)
            .map(|n| {
                let player = generate_random_player_from_def(
                    "club-id",
                    n % 23,
                    "ENG",
                    2026,
                    None,
                    &names_def,
                    &mut rng,
                );
                let birth_year: i32 = player.date_of_birth[..4].parse().unwrap();
                (player, 2026 - birth_year)
            })
            .collect()
    }

    /// Given rolled ratings at, above and below the rolled mean,
    /// When they are shaped onto the ceiling bell,
    /// Then the mean maps to the bell's centre and a distance from it is kept at the spread.
    #[test]
    fn a_rolled_rating_is_shaped_onto_the_ceiling_bell() {
        assert_eq!(
            shaped_ceiling(ROLLED_RATING_MEAN, 25),
            CEILING_MEAN.round() as u8
        );
        let above = shaped_ceiling(ROLLED_RATING_MEAN + 10.0, 25);
        let below = shaped_ceiling(ROLLED_RATING_MEAN - 10.0, 25);
        let centre = CEILING_MEAN.round() as u8;
        assert_eq!(above - centre, (10.0 * CEILING_SPREAD).round() as u8);
        assert_eq!(centre - below, (10.0 * CEILING_SPREAD).round() as u8);
    }

    /// Given the same rolled rating far above the mean for a teenager and for a prime-age player,
    /// When it is shaped onto the ceiling bell,
    /// Then the teenager's ceiling is closer to the centre (his bell is narrower) and the centre
    /// itself is the same.
    #[test]
    fn a_teenagers_ceiling_bell_is_narrower_than_a_prime_players() {
        let prime = shaped_ceiling(ROLLED_RATING_MEAN + 20.0, 25);
        let teenager = shaped_ceiling(ROLLED_RATING_MEAN + 20.0, 16);
        assert!(teenager < prime, "{teenager} vs {prime}");
        assert_eq!(
            shaped_ceiling(ROLLED_RATING_MEAN, 16),
            shaped_ceiling(ROLLED_RATING_MEAN, 25)
        );
    }

    /// Given generated teenagers (18 or younger),
    /// When their rating is compared with their ceiling,
    /// Then the ceiling is the headroom for their age above the rating: they are an earlier self.
    #[test]
    fn a_generated_teenager_is_an_earlier_version_of_his_ceiling() {
        let teenagers: Vec<_> = generated_players(120)
            .into_iter()
            .filter(|(_, age)| *age <= 18)
            .collect();
        assert!(teenagers.len() > 200, "the fixture has too few teenagers");
        for (player, age) in teenagers {
            let room = player.potential - player.ovr;
            assert!(
                (13..=32).contains(&room),
                "a {age}-year-old rated {} has a ceiling of {}, {room} above",
                player.ovr,
                player.potential
            );
        }
    }

    /// Given generated players of 26 or older,
    /// When their rating is compared with their ceiling,
    /// Then they are at it: a prime-age player has nothing left to grow into.
    #[test]
    fn a_generated_veteran_is_at_his_ceiling() {
        let veterans: Vec<_> = generated_players(120)
            .into_iter()
            .filter(|(_, age)| *age >= 26)
            .collect();
        assert!(veterans.len() > 500, "the fixture has too few veterans");
        for (player, age) in veterans {
            assert!(
                player.potential - player.ovr <= 1,
                "a {age}-year-old rated {} has a ceiling of {}",
                player.ovr,
                player.potential
            );
        }
    }

    /// Given the same generated world,
    /// When the average ceiling of its teenagers is compared with the average rating of its veterans,
    /// Then they match: a teenager's ceiling is where a veteran's rating ends up, not that plus his
    /// headroom.
    #[test]
    fn a_teenagers_ceiling_is_where_a_veterans_rating_ends_up() {
        let world = generated_players(360);
        let mean = |values: Vec<u8>| {
            values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64
        };
        let teenage_ceiling = mean(
            world
                .iter()
                .filter(|(_, age)| *age <= 18)
                .map(|(p, _)| p.potential)
                .collect(),
        );
        let veteran_rating = mean(
            world
                .iter()
                .filter(|(_, age)| *age >= 26)
                .map(|(p, _)| p.ovr)
                .collect(),
        );

        assert!(
            (teenage_ceiling - veteran_rating).abs() < 1.5,
            "teenagers' ceilings average {teenage_ceiling:.1}, veterans' ratings {veteran_rating:.1}"
        );
    }

    /// Given a generated world of 360 clubs,
    /// When its senior players are averaged,
    /// Then the world opens near the mean the maintainer asked to keep (about 68).
    #[test]
    fn a_new_world_opens_near_the_intended_mean_rating() {
        let seniors: Vec<f64> = generated_players(360)
            .into_iter()
            .filter(|(_, age)| *age >= 21)
            .map(|(player, _)| f64::from(player.ovr))
            .collect();
        let mean = seniors.iter().sum::<f64>() / seniors.len() as f64;

        assert!((67.0..=68.5).contains(&mean), "seniors average {mean:.2}");
    }

    /// Given the same world,
    /// When the players the game tags as wonderkids are counted,
    /// Then they are rare: between 0.2% and 1.2% of everyone.
    #[test]
    fn a_new_worlds_wonderkids_are_rare() {
        let world = generated_players(360);
        let wonderkids = world
            .iter()
            .filter(|(player, _)| {
                player
                    .traits
                    .contains(&domain::player::PlayerTrait::Wonderkid)
            })
            .count();
        let share = wonderkids as f64 / world.len() as f64;

        assert!(
            (0.002..=0.012).contains(&share),
            "{wonderkids} of {} are wonderkids ({:.2}%)",
            world.len(),
            share * 100.0
        );
    }

    /// Given a generated world of 360 clubs,
    /// When the ceilings of its senior players are tiered,
    /// Then S is 1 to 3.5% and A is 4.5 to 9%, the maintainer's targets.
    #[test]
    fn the_ceilings_of_senior_players_follow_the_tier_targets() {
        use crate::player_tier::{PlayerTier, player_tier};
        let seniors: Vec<_> = generated_players(360)
            .into_iter()
            .filter(|(_, age)| *age >= 21)
            .collect();
        let share = |tier: PlayerTier| {
            seniors
                .iter()
                .filter(|(player, _)| player_tier(player.potential) == tier)
                .count() as f64
                / seniors.len() as f64
        };

        let (s, a) = (share(PlayerTier::S), share(PlayerTier::A));

        assert!(
            (0.01..=0.035).contains(&s),
            "S ceilings are {:.1}%",
            s * 100.0
        );
        assert!(
            (0.045..=0.09).contains(&a),
            "A ceilings are {:.1}%",
            a * 100.0
        );
    }

    /// Given staff authored with and without a `knownAs` name (#557)
    /// When they are generated
    /// Then the one with it is displayed by it, keeping the real name, and the other by first and last.
    #[test]
    fn authored_staff_are_displayed_by_what_they_are_called() {
        let names_def = super::super::definitions::default_names_definition();
        let generate = |value: serde_json::Value| {
            let def: super::super::package::StaffDef = serde_json::from_value(value).unwrap();
            generate_staff_from_authored_def(
                &def,
                Some("club-id"),
                2026,
                &names_def,
                &mut rand::rng(),
            )
        };

        let pep = generate(serde_json::json!({
            "id": "pep", "firstName": "Josep", "lastName": "Guardiola i Sala",
            "knownAs": "Pep Guardiola", "role": "AssistantManager",
        }));
        let plain = generate(serde_json::json!({
            "id": "alice", "firstName": "Alice", "lastName": "Coach", "role": "Coach",
        }));

        assert_eq!(pep.display_name, "Pep Guardiola");
        assert_eq!(pep.full_name(), "Josep Guardiola i Sala");
        assert_eq!(plain.display_name, "Alice Coach");
    }

    fn authored_names(value: serde_json::Value) -> (String, String) {
        let mut def = serde_json::json!({
            "id": "p1", "club": "club-id", "nationality": "BRA", "position": "Striker",
            "dateOfBirth": "1990-01-01", "overall": 70,
        });
        def.as_object_mut()
            .unwrap()
            .extend(value.as_object().unwrap().clone());
        let player = generate_from_json(def, 2026);
        (player.full_name, player.match_name)
    }

    /// Given players authored with a `name` beside their real first and last names (#556)
    /// When they are generated
    /// Then `full_name` is the real name and `match_name` is what they are called: the `name`
    ///      without the words it shares with the first name, or the whole `name` when it
    ///      shares none (a nickname).
    #[test]
    fn an_authored_name_is_what_the_player_is_called() {
        let cases = [
            // nickname: shares nothing with the first name
            (
                serde_json::json!({"name": "Kaká", "firstName": "Ricardo Izecson", "lastName": "dos Santos Leite"}),
                "Ricardo Izecson dos Santos Leite",
                "Kaká",
            ),
            // plain "First Last" that repeats the first name
            (
                serde_json::json!({"name": "Iker Casillas", "firstName": "Iker", "lastName": "Casillas Fernández"}),
                "Iker Casillas Fernández",
                "Casillas",
            ),
            // the first name is longer than the part of it the name repeats
            (
                serde_json::json!({"name": "Lionel Messi", "firstName": "Lionel Andrés", "lastName": "Messi Cuccittini"}),
                "Lionel Andrés Messi Cuccittini",
                "Messi",
            ),
            // compound surname with particles: nothing past the first name is dropped
            (
                serde_json::json!({"name": "Nicolás de la Cruz", "firstName": "Nicolás", "lastName": "de la Cruz Bascuñán"}),
                "Nicolás de la Cruz Bascuñán",
                "de la Cruz",
            ),
            (
                serde_json::json!({"name": "Alfredo Di Stefano", "firstName": "Alfredo", "lastName": "Di Stéfano Laulhé"}),
                "Alfredo Di Stéfano Laulhé",
                "Di Stefano",
            ),
            // the comparison ignores case
            (
                serde_json::json!({"name": "Nicolás de la Cruz", "firstName": "nicolás", "lastName": "de la Cruz Bascuñán"}),
                "nicolás de la Cruz Bascuñán",
                "de la Cruz",
            ),
            // a name that is only the first name stays whole, never empty
            (
                serde_json::json!({"name": "Ronaldo", "firstName": "Ronaldo", "lastName": "Nazário de Lima"}),
                "Ronaldo Nazário de Lima",
                "Ronaldo",
            ),
            // no `name`: today's fallback, the last name
            (
                serde_json::json!({"firstName": "John", "lastName": "Smith"}),
                "John Smith",
                "Smith",
            ),
            // only a `name`, no first or last: it is split, and the first word is the first name
            (
                serde_json::json!({"name": "Lionel Messi"}),
                "Lionel Messi",
                "Messi",
            ),
        ];
        for (authored, full_name, match_name) in cases {
            assert_eq!(
                authored_names(authored.clone()),
                (full_name.to_string(), match_name.to_string()),
                "{authored}"
            );
        }
    }

    /// Given a date of birth that validation would have refused, reaching generation anyway (#594).
    ///       "1990" is chosen because its first four characters read as a year: only the date check
    ///       keeps it from being stored as the date
    /// When the player is generated
    /// Then it is treated as if none were written (here, the authored age decides), not read as
    ///      a year or as "24 years before the opening", the invented value it used to become.
    #[test]
    fn an_unreadable_date_of_birth_is_not_replaced_by_an_invented_age() {
        let player = generate_from_json(
            serde_json::json!({
                "id": "p", "firstName": "A", "lastName": "B", "club": "club-id",
                "nationality": "ENG", "position": "Striker",
                "dateOfBirth": "1990", "age": 31, "overall": 60,
            }),
            2026,
        );

        assert_eq!(
            player.date_of_birth, "1995-01-01",
            "{:?}",
            player.date_of_birth
        );
    }

    /// Given staff authored with a date of birth validation would have refused ("1990" reads as a
    ///       year but is not a date), and an age
    /// When they are generated
    /// Then the age decides and the stored date is a real one, as for a player.
    #[test]
    fn an_unreadable_staff_date_of_birth_is_not_replaced_by_an_invented_age() {
        use rand::SeedableRng;
        let names_def = super::super::definitions::default_names_definition();
        let def: super::super::package::StaffDef = serde_json::from_value(serde_json::json!({
            "id": "coach-1", "firstName": "A", "lastName": "B", "role": "Coach",
            "dateOfBirth": "1990", "age": 31,
        }))
        .unwrap();

        let staff = generate_staff_from_authored_def(
            &def,
            Some("club-id"),
            2026,
            &names_def,
            &mut rand::rngs::StdRng::seed_from_u64(7),
        );

        assert_eq!(staff.date_of_birth, "1995-01-01");
    }

    /// The rating of 100 players authored at `overall`, one per seed.
    fn ovrs_of_authored(overall: u8, position: &str) -> Vec<u8> {
        use rand::SeedableRng;
        let names_def = super::super::definitions::default_names_definition();
        let def = player_def_from_json(serde_json::json!({
            "id": "p", "firstName": "A", "lastName": "B", "club": "club-id",
            "nationality": "ENG", "position": position,
            "dateOfBirth": "1995-01-01", "overall": overall,
        }));
        (0..100u64)
            .map(|seed| {
                let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
                generate_player_from_def(&def, "club-id", 2026, &names_def, &mut rng).ovr
            })
            .collect()
    }

    /// Given players authored at a low, middling and high `overall` (#495)
    /// When many are generated
    /// Then the mean rating of each lands within 3 of what was written, for any position.
    #[test]
    fn an_authored_overall_comes_out_near_itself() {
        for position in ["Striker", "CenterBack", "Goalkeeper"] {
            for overall in [10u8, 20, 30, 40, 50, 70, 90] {
                let ovrs = ovrs_of_authored(overall, position);

                // The balance: no lean to one side.
                let mean = ovrs.iter().map(|ovr| f64::from(*ovr)).sum::<f64>() / ovrs.len() as f64;
                assert!(
                    (mean - f64::from(overall)).abs() <= 3.0,
                    "{position} authored {overall} averaged {mean}"
                );
                // The spread: a mean can hide ratings too high and too low cancelling out, so
                // every player is bounded too. Measured over these seeds the worst is 2 (1 at 70
                // and 90); 3 leaves a point of room. ±1 does not hold: the attributes are whole
                // numbers and the position weights round.
                for ovr in &ovrs {
                    assert!(
                        (i32::from(*ovr) - i32::from(overall)).abs() <= 3,
                        "{position} authored {overall} produced a player rated {ovr}"
                    );
                }
            }
        }
    }

    /// Given a base so low the spread would reach below zero
    /// When attributes are jittered around it
    /// Then none is ever below 1: the floor is pinned, since 0 reads as "missing" elsewhere.
    #[test]
    fn jittered_attributes_never_fall_below_one() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let draws: Vec<u8> = (0..2000).map(|_| jitter(1, 8, 30, 97, &mut rng)).collect();
        assert_eq!(draws.iter().copied().min(), Some(1));
    }

    /// An authored ceiling is the author's to set.
    ///
    /// A package could set a player's current ability but never their ceiling —
    /// it was always rolled from `ovr` and age, so a modder could write a
    /// 17-year-old at 55 but not say that he becomes a 92.
    #[test]
    fn an_authored_potential_is_kept() {
        let player = generate_from_json(
            serde_json::json!({
                "id": "wonder-kid",
                "firstName": "Wonder",
                "lastName": "Kid",
                "club": "club-id",
                "nationality": "ENG",
                "position": "Striker",
                "dateOfBirth": "2009-01-01",
                "overall": 55,
                "potential": 92,
            }),
            2026,
        );

        assert_eq!(player.potential, 92, "the authored ceiling was overwritten");
    }

    /// The same, for the author who specifies attributes rather than an overall.
    #[test]
    fn an_authored_potential_is_kept_in_attributes_mode() {
        let player = generate_from_json(
            serde_json::json!({
                "id": "precise-kid",
                "firstName": "Precise",
                "lastName": "Kid",
                "club": "club-id",
                "nationality": "ENG",
                "position": "CentralMidfielder",
                "dateOfBirth": "2009-01-01",
                "attributes": {
                    "pace": 50, "stamina": 50, "strength": 50,
                    "passing": 50, "shooting": 50, "tackling": 50,
                    "dribbling": 50, "defending": 50,
                    "positioning": 50, "vision": 50, "decisions": 50,
                },
                "potential": 88,
            }),
            2026,
        );

        assert_eq!(player.potential, 88, "the authored ceiling was overwritten");
    }

    /// Omitting it keeps the roll. The back-compat guarantee for every package
    /// already in the wild.
    ///
    /// Asserted against the age bonus band rather than "non-zero and >= ovr",
    /// which a regression that simply set `potential = ovr` would also pass.
    #[test]
    fn an_omitted_potential_is_still_rolled() {
        let player = generate_from_json(
            serde_json::json!({
                "id": "ordinary-kid",
                "firstName": "Ordinary",
                "lastName": "Kid",
                "club": "club-id",
                "nationality": "ENG",
                "position": "Striker",
                "dateOfBirth": "2009-01-01",
                "overall": 55,
            }),
            2026,
        );

        // The inverse of the ceiling-first roll: an authored rating gets the headroom for his
        // age above it, from the same range a generated player's ceiling is lowered by.
        let headroom = crate::player_rating::growth_headroom_range(17);
        let floor = player.ovr.saturating_add(*headroom.start()).min(99);
        let ceiling = player.ovr.saturating_add(*headroom.end()).min(99);
        assert!(
            (floor..=ceiling).contains(&player.potential),
            "a 17-year-old's rolled ceiling should sit {floor}..={ceiling}, got {}",
            player.potential
        );
    }

    // -- authored contract, wage, value and status ---------------------------
    //
    // Built from JSON for the same reason as the tests above. The dates used here
    // are ones generation can never produce (it always ends a contract on 30 June),
    // so an unfixed tree cannot pass by coincidence.

    fn striker_json(extra: serde_json::Value) -> serde_json::Value {
        let mut base = serde_json::json!({
            "id": "authored-striker",
            "firstName": "Authored",
            "lastName": "Striker",
            "club": "club-id",
            "nationality": "ENG",
            "position": "Striker",
            "dateOfBirth": "1990-05-01",
            "overall": 70,
        });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        base
    }

    #[test]
    fn an_authored_contract_end_is_kept() {
        let player = generate_from_json(
            striker_json(serde_json::json!({ "contractEnd": "2031-03-15" })),
            2026,
        );

        assert_eq!(player.contract_end(), Some("2031-03-15"));
        assert_eq!(
            player.contract_start(),
            None,
            "no start was authored, and one is given when the career opens, not here"
        );
    }

    /// A length resolves against the year the career opens in, which is what lets a
    /// package written for one era be played in another without every contract
    /// having already run out.
    #[test]
    fn a_contract_length_counts_from_the_opening_year() {
        for (opening_year, expected_end) in [(1962, "1965-06-30"), (2026, "2029-06-30")] {
            let player = generate_from_json(
                striker_json(serde_json::json!({ "contractLength": 3 })),
                opening_year,
            );

            assert_eq!(
                player.contract_end(),
                Some(expected_end),
                "a 3-year length opened in {opening_year}"
            );
            assert_eq!(player.contract_start(), None);
        }
    }

    #[test]
    fn a_contract_length_counts_from_an_authored_start() {
        let player = generate_from_json(
            striker_json(serde_json::json!({
                "contractStart": "2024-01-15",
                "contractLength": 2,
            })),
            2026,
        );

        assert_eq!(player.contract_start(), Some("2024-01-15"));
        assert_eq!(player.contract_end(), Some("2026-01-15"));
    }

    /// A start alone is half an interval. Rather than invent an end date, the engine
    /// rolls a length the way it always has, counted from the author's start, so the
    /// interval is whole and the start is the author's.
    #[test]
    fn a_start_with_no_end_keeps_the_start_and_rolls_a_length_from_it() {
        let player = generate_from_json(
            striker_json(serde_json::json!({ "contractStart": "2024-01-15" })),
            2026,
        );

        assert_eq!(player.contract_start(), Some("2024-01-15"));
        let end = player.contract_end().expect("an end was rolled");
        assert!(end > "2024-01-15", "ended {end}, not after the start");
        assert!(
            end <= "2029-01-15",
            "{end} is more than five years from the start"
        );
        assert!(
            end.ends_with("-01-15"),
            "{end}: the length should count in whole years from the start"
        );
    }

    #[test]
    fn an_authored_wage_and_value_are_kept() {
        let player = generate_from_json(
            striker_json(serde_json::json!({ "wage": 12_345, "value": 9_000_000 })),
            2026,
        );

        assert_eq!(player.wage(), 12_345);
        assert_eq!(player.market_value, 9_000_000);
    }

    /// Omit the wage and it is sized from the value the player actually ends up
    /// with, not from a value the author replaced.
    #[test]
    fn an_omitted_wage_follows_an_authored_value() {
        let player = generate_from_json(
            striker_json(serde_json::json!({ "value": 4_000_000 })),
            2026,
        );

        assert_eq!(player.market_value, 4_000_000);
        assert_eq!(
            player.wage(),
            4_000_000 / 200,
            "wage should be sized from the authored value"
        );
    }

    /// Zero is a value, not an absence. A `0` that fell back to the roll would make
    /// it impossible to author a player on no wage or with no condition.
    #[test]
    fn an_explicit_zero_survives() {
        let player = generate_from_json(
            striker_json(serde_json::json!({ "wage": 0, "condition": 0, "morale": 0 })),
            2026,
        );

        assert_eq!(player.wage(), 0);
        assert_eq!(player.condition, 0);
        assert_eq!(player.morale, 0);
    }

    #[test]
    fn authored_status_and_identity_fields_are_kept() {
        let player = generate_from_json(
            striker_json(serde_json::json!({
                "condition": 64,
                "morale": 51,
                "weakFoot": 4,
                "alternatePositions": ["LeftWinger"],
                "careerHistory": [
                    { "season": 2019, "teamName": "Juventus",
                      "appearances": 30, "goals": 10, "assists": 5 },
                    { "season": 2020, "teamId": "club-x", "teamName": "Club X",
                      "appearances": 12, "goals": 1, "assists": 2 },
                ],
            })),
            2026,
        );

        assert_eq!(player.condition, 64);
        assert_eq!(player.morale, 51);
        assert_eq!(player.weak_foot, 4);
        assert_eq!(player.alternate_positions, vec![Position::LeftWinger]);
        assert_eq!(player.career.len(), 2);
        // A club that is not in the package has no id, and keeps its name.
        assert_eq!(player.career[0].team_id, "");
        assert_eq!(player.career[0].team_name, "Juventus");
        assert_eq!(player.career[0].goals, 10);
        assert_eq!(player.career[1].team_id, "club-x");
    }

    /// Everything here is optional, and leaving it out must change nothing: every
    /// package written before these fields existed has to generate as it always did.
    #[test]
    fn omitted_status_fields_are_still_generated() {
        let player = generate_from_json(striker_json(serde_json::json!({})), 2026);

        assert!(
            (75..100).contains(&player.condition),
            "condition {}",
            player.condition
        );
        assert!(
            (40..76).contains(&player.morale),
            "morale {}",
            player.morale
        );
        assert!(player.market_value > 0);
        assert!(player.wage() >= 500);
        assert!(player.contract_end().is_some());
        assert_eq!(player.contract_start(), None);
        assert!(
            player.career.is_empty(),
            "history is not invented for an authored player here"
        );
    }

    /// An authored weak foot and alternate positions have to survive the identity
    /// upgrade that runs when a career opens and when a save loads, or they would be
    /// accepted and then silently replaced. That holds for a specific position; for
    /// a general group (Midfielder, Forward, …) the upgrade re-infers them, which is
    /// why validation refuses the fields there rather than letting them be discarded.
    #[test]
    fn an_authored_weak_foot_and_alternates_survive_the_identity_upgrade() {
        let mut player = generate_from_json(
            striker_json(serde_json::json!({
                "weakFoot": 5,
                "alternatePositions": ["LeftWinger"],
            })),
            2026,
        );

        crate::player_identity::upgrade_player_identity(&mut player, None);

        assert_eq!(
            player.weak_foot, 5,
            "the identity upgrade replaced the authored weak foot"
        );
        assert_eq!(player.alternate_positions, vec![Position::LeftWinger]);
    }

    /// An authored ceiling is exact, not a suggestion.
    ///
    /// An `overall` is turned into a *jittered* attribute spread and the real ovr
    /// derived from that, so it lands near the target rather than on it. Since
    /// `refresh_player_derived` floors potential at ovr, a ceiling the author
    /// wrote could be quietly raised — measured at 26-32% for mid-range
    /// abilities, and every single time below the attribute clamp floor.
    #[test]
    fn an_authored_ceiling_is_never_raised_by_attribute_jitter() {
        for _ in 0..200 {
            let player = generate_from_json(
                serde_json::json!({
                    "id": "veteran",
                    "firstName": "Fin", "lastName": "Ished",
                    "club": "club-id", "nationality": "ENG", "position": "Striker",
                    "dateOfBirth": "1990-01-01",
                    "overall": 70,
                    "potential": 70,
                }),
                2026,
            );
            assert_eq!(
                player.potential, 70,
                "the authored ceiling was raised to match a jittered ovr of {}",
                player.ovr
            );
        }
    }

    /// The same, where it used to fail every time rather than sometimes.
    ///
    /// Attributes are clamped to a floor of 30 (`jitter(base, 8, 30, 97, ..)`),
    /// so a deliberately poor player could never be generated at the ability
    /// they were authored with, and their ceiling rose with it.
    #[test]
    fn a_deliberately_limited_player_keeps_their_low_ceiling() {
        let player = generate_from_json(
            serde_json::json!({
                "id": "journeyman",
                "firstName": "Jour", "lastName": "Neyman",
                "club": "club-id", "nationality": "ENG", "position": "Striker",
                "dateOfBirth": "1990-01-01",
                "overall": 10,
                "potential": 10,
            }),
            2026,
        );

        assert_eq!(player.potential, 10, "the authored ceiling was raised");
        assert!(
            player.ovr <= 10,
            "a player cannot be generated above their own ceiling, got ovr {}",
            player.ovr
        );
    }

    /// Authoring a ceiling for a youth prospect is the case the field exists for.
    #[test]
    fn a_youth_prospect_keeps_both_their_squad_role_and_their_ceiling() {
        let player = generate_from_json(
            serde_json::json!({
                "id": "academy-star",
                "firstName": "Academy", "lastName": "Star",
                "club": "club-id", "nationality": "ENG", "position": "AttackingMidfielder",
                "dateOfBirth": "2009-06-02",
                "overall": 58,
                "potential": 88,
                "youth": true,
            }),
            2026,
        );

        assert_eq!(player.potential, 88);
        assert_eq!(player.squad_role, domain::player::SquadRole::Youth);
    }

    /// Authoring a generational talent also authors the badge.
    ///
    /// `refresh_player_derived` awards Wonderkid from age, ceiling and headroom,
    /// so it follows from the ceiling rather than being set separately. The
    /// modding reference promises this; without a test the promise is unbacked.
    #[test]
    fn an_authored_ceiling_can_earn_the_wonderkid_trait() {
        let player = generate_from_json(
            serde_json::json!({
                "id": "generational",
                "firstName": "Gene", "lastName": "Rational",
                "club": "club-id", "nationality": "ENG", "position": "Striker",
                "dateOfBirth": "2009-01-01",
                "overall": 55,
                "potential": 92,
            }),
            2026,
        );

        assert!(
            player
                .traits
                .contains(&domain::player::PlayerTrait::Wonderkid),
            "17, ceiling 92, ovr {} — that is a wonderkid: {:?}",
            player.ovr,
            player.traits
        );
    }

    /// And only then. Otherwise every authored ceiling would earn the badge.
    #[test]
    fn an_authored_ceiling_alone_does_not_earn_the_wonderkid_trait() {
        // Old enough that the age gate refuses regardless of the ceiling.
        let veteran = generate_from_json(
            serde_json::json!({
                "id": "late-bloomer",
                "firstName": "Late", "lastName": "Bloomer",
                "club": "club-id", "nationality": "ENG", "position": "Striker",
                "dateOfBirth": "1996-01-01",
                "overall": 70,
                "potential": 95,
            }),
            2026,
        );
        assert!(
            !veteran
                .traits
                .contains(&domain::player::PlayerTrait::Wonderkid),
            "a 30-year-old is not a wonderkid whatever his ceiling: {:?}",
            veteran.traits
        );

        // Young, but the ceiling is nowhere near the elite threshold.
        let ordinary = generate_from_json(
            serde_json::json!({
                "id": "ordinary-prospect",
                "firstName": "Ordinary", "lastName": "Prospect",
                "club": "club-id", "nationality": "ENG", "position": "Striker",
                "dateOfBirth": "2009-01-01",
                "overall": 55,
                "potential": 70,
            }),
            2026,
        );
        assert!(
            !ordinary
                .traits
                .contains(&domain::player::PlayerTrait::Wonderkid),
            "a ceiling of 70 is not wonderkid territory: {:?}",
            ordinary.traits
        );
    }

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
    ///
    /// This is about the *catalog* resolver alone: `zz` names no real nation, so
    /// on its own it is unresolvable. It is emphatically not a statement that a
    /// two-letter id can never be a nationality — a package declaring `ZZ` makes
    /// it one, which `a_package_declared_country_supplies_its_own_clubs` covers.
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

    /// A package declares its own country, and its clubs must be staffed with
    /// people from it. This is the whole point of authoring a package, and the
    /// catalog cannot help: `ZZ` is in neither list, so drawing from the static
    /// distribution alone leaves a Zedlandian club with no Zedlandians at all.
    #[test]
    fn a_package_declared_country_supplies_its_own_clubs() {
        let mut rng = rand::rng();
        let pool = nationality_distribution_including(["ZZ"].into_iter());

        assert!(
            pool.iter().any(|code| code == "ZZ"),
            "a declared country must be drawable"
        );

        let drawn: Vec<String> = (0..400)
            .map(|_| pick_nationality_from_def("ZZ", &pool, &mut rng))
            .collect();
        let local = drawn.iter().filter(|code| *code == "ZZ").count();

        // The local share is 60%; allow generous slack for the draw. The bug
        // being guarded is zero, not a few points either way.
        assert!(
            local > drawn.len() / 3,
            "{local}/{} drawn from the club's own declared country",
            drawn.len()
        );
    }

    /// A declared id that is already a catalog code must not be appended a
    /// second time — it is in the pool at its proper weight already, and
    /// stacking would quietly promote whichever real nations a package happens
    /// to name.
    #[test]
    fn declaring_a_country_the_catalog_already_has_does_not_promote_it() {
        let baseline = nationality_distribution()
            .iter()
            .filter(|c| *c == "BR")
            .count();
        let pool = nationality_distribution_including(["BR", "br"].into_iter());
        let after = pool.iter().filter(|c| *c == "BR").count();

        assert_eq!(after, baseline, "Brazil must keep its catalog weight");
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
        assert!(
            count("FR") > count("PL"),
            "FR {} vs PL {}",
            count("FR"),
            count("PL")
        );
        assert!(
            count("PL") > count("AD"),
            "PL {} vs AD {}",
            count("PL"),
            count("AD")
        );
        assert!(
            count("BR") > count("BO"),
            "BR {} vs BO {}",
            count("BR"),
            count("BO")
        );
    }

    /// Rank is only meaningful inside a region, so the assertions above cannot
    /// see the failure that matters most: with no region factor, the top of
    /// every region weighs the same and Costa Rica draws as often as Brazil.
    #[test]
    fn a_regions_depth_counts_not_just_rank_within_it() {
        let pool = nationality_distribution();
        let count = |code: &str| pool.iter().filter(|entry| *entry == code).count();

        // Each pair is top-of-region against top-of-region, so rank alone ties
        // them and only the region factor can separate them.
        assert!(
            count("BR") > count("CR"),
            "BR {} vs CR {}",
            count("BR"),
            count("CR")
        );
        assert!(
            count("FR") > count("CR"),
            "FR {} vs CR {}",
            count("FR"),
            count("CR")
        );
        assert!(
            count("BR") > count("NZ"),
            "BR {} vs NZ {}",
            count("BR"),
            count("NZ")
        );

        // And a mid-table European outranks the best of a shallow region.
        assert!(
            count("IT") > count("NZ"),
            "IT {} vs NZ {}",
            count("IT"),
            count("NZ")
        );

        // South America must out-supply Central America overall, which was
        // inverted while rank was the only factor.
        let region_share = |region: &str| {
            pool.iter()
                .filter(|code| nations::region_for_code(code) == region)
                .count()
        };
        assert!(
            region_share("south-america") > region_share("central-america"),
            "south-america {} vs central-america {}",
            region_share("south-america"),
            region_share("central-america")
        );
    }

    /// The pool is indexed with the RNG, so its order has to be stable or the
    /// same seed stops producing the same world.
    ///
    /// Built from scratch rather than compared against the memoised copy:
    /// `nationality_distribution()` returns the same `&'static` reference every
    /// time, so asserting it equals itself passes even if the builder iterated
    /// a `HashMap` and produced a different order on every run.
    #[test]
    fn the_distribution_is_stable_across_builds() {
        let build = || {
            let mut pool = Vec::new();
            let mut seen_in_region: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for nation in nations::NATION_CATALOG {
                let rank = seen_in_region.entry(nation.region_id).or_insert(0);
                let weight = 12usize.saturating_sub(*rank).max(2) * region_weight(nation.region_id);
                *rank += 1;
                for _ in 0..weight {
                    pool.push(nation.code.to_string());
                }
            }
            for nation in nations::ADDITIONAL_NATIONS {
                pool.push(nation.code.to_string());
            }
            pool
        };

        assert_eq!(build(), build(), "two independent builds must agree");
        assert_eq!(
            &build(),
            nationality_distribution(),
            "and must agree with the memoised pool"
        );
    }
}
