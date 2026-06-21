use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{Staff, StaffAttributes, StaffRole};
use domain::team::PlayStyle;
use rand::{Rng, RngExt};
use uuid::Uuid;

use super::definitions::NamesDefinition;
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

/// Pick a nationality code weighted 60% toward team country.
pub(super) fn pick_nationality_from_def(
    team_country: &str,
    available_codes: &[String],
    rng: &mut impl Rng,
) -> String {
    // Map team country name → ISO code for the 60% local weight
    let local_code = country_to_iso(team_country);
    let selected_code = if available_codes.is_empty() || rng.random_range(0..100) < 60 {
        local_code.to_string()
    } else {
        available_codes[rng.random_range(0..available_codes.len())].clone()
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

    // Fallback: pick from any available pool. Use the lexicographically
    // smallest key so the choice is stable (HashMap order is randomized).
    if let Some(key) = names_def.pools.keys().min() {
        let pool = &names_def.pools[key];
        let first = pool.first_names[rng.random_range(0..pool.first_names.len())].clone();
        let last = pool.last_names[rng.random_range(0..pool.last_names.len())].clone();
        return (first, last);
    }
    ("Player".to_string(), "Unknown".to_string())
}

pub(super) fn country_to_iso(country: &str) -> &str {
    match country {
        "England" | "ENG" => "ENG",
        "Scotland" | "SCO" => "SCO",
        "Wales" | "WAL" => "WAL",
        "Northern Ireland" | "NIR" => "NIR",
        "Ireland" | "Republic of Ireland" | "IE" => "IE",
        "GB" => "GB",
        "Spain" | "ES" => "ES",
        "Germany" | "DE" => "DE",
        "France" | "FR" => "FR",
        "Italy" | "IT" => "IT",
        "Netherlands" | "NL" => "NL",
        "Portugal" | "PT" => "PT",
        "Brazil" | "BR" => "BR",
        "Argentina" | "AR" => "AR",
        "Belgium" | "BE" => "BE",
        "Croatia" | "HR" => "HR",
        "Sweden" | "SE" => "SE",
        other => {
            // If already a short code, return as-is.
            if other.len() == 2 || other.len() == 3 {
                other
            } else {
                "ENG"
            }
        }
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

pub(super) fn generate_random_player_from_def(
    team_id: &str,
    index: usize,
    nationality: &str,
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

    // Reserve slots across the back line, midfield, and attack always start youth-aged
    // so each club can open with real academy prospects instead of an empty youth squad.
    let age = if matches!(index, 8 | 15 | 21) {
        rng.random_range(17..22)
    } else {
        rng.random_range(17..36)
    };
    let birth_year = 2026 - age;
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

    // For initial market-value sizing, use a temporary simple attribute average.
    // The accurate position-weighted OVR is computed by refresh_player_derived() below.
    let current_year: u32 = 2026;

    let approx_ovr = (attributes.pace as u32
        + attributes.stamina as u32
        + attributes.strength as u32
        + attributes.passing as u32
        + attributes.shooting as u32
        + attributes.tackling as u32
        + attributes.dribbling as u32
        + attributes.defending as u32
        + attributes.positioning as u32
        + attributes.vision as u32
        + attributes.decisions as u32)
        / 11;

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
    let contract_end = format!("{}-06-30", 2026 + contract_years);

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
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Staff {
    let (first_name, last_name) = pick_name_from_def(nationality, names_def, rng);
    let age = rng.random_range(30..60);
    let birth_year = 2026 - age;
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
    names_def: &NamesDefinition,
    rng: &mut impl Rng,
) -> Staff {
    let (first_name, last_name) = pick_name_from_def(nationality, names_def, rng);
    let age = rng.random_range(28..55);
    let birth_year = 2026 - age;
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

fn resolve_birth_year(def: &super::package::PlayerDef, rng: &mut impl Rng) -> u32 {
    if let Some(dob) = &def.date_of_birth {
        dob.get(0..4)
            .and_then(|year| year.parse::<u32>().ok())
            .unwrap_or(2026 - 24)
    } else if let Some(age) = def.age {
        2026u32.saturating_sub(age)
    } else {
        2026 - rng.random_range(18..34)
    }
}

/// Convert a hand-authored [`PlayerDef`](super::package::PlayerDef) into a full
/// player for `team_id`. Ability comes from an explicit `attributes` block or is
/// generated around `overall`; identity falls back to the name pools when not
/// given.
pub(super) fn generate_player_from_def(
    def: &super::package::PlayerDef,
    team_id: &str,
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

    let current_year: u32 = 2026;
    let birth_year = resolve_birth_year(def, rng);
    let dob = def
        .date_of_birth
        .clone()
        .unwrap_or_else(|| format!("{birth_year:04}-01-01"));
    let age = current_year.saturating_sub(birth_year);

    let attributes = def
        .attributes
        .clone()
        .unwrap_or_else(|| attributes_for_overall(def.overall.unwrap_or(65), &def.position, rng));

    let approx_ovr = (attributes.pace as u32
        + attributes.stamina as u32
        + attributes.strength as u32
        + attributes.passing as u32
        + attributes.shooting as u32
        + attributes.tackling as u32
        + attributes.dribbling as u32
        + attributes.defending as u32
        + attributes.positioning as u32
        + attributes.vision as u32
        + attributes.decisions as u32)
        / 11;
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
    let contract_end = format!("{}-06-30", 2026 + contract_years);

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
    player.market_value = market_value;
    player.wage = wage;
    player.contract_end = Some(contract_end);
    player.condition = rng.random_range(75..100);
    player.morale = rng.random_range(40..76);

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
