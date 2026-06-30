//! Pure region- and division-classification helpers used when building a
//! generated world's competition pyramid. All functions here are leaf helpers
//! (no dependencies on other `game` helpers) and are unit-tested in
//! `game::tests`.

use domain::league::{CompetitionScope, League};

pub(crate) fn infer_region_id(country_code: &str) -> String {
    ofm_core::nations::region_for_code(country_code).to_string()
}

pub(crate) fn infer_team_region_id(team: &domain::team::Team) -> String {
    if !team.football_nation.is_empty() {
        return infer_region_id(&team.football_nation);
    }
    infer_region_id(&team.country)
}

pub(crate) fn competition_required_region_ids(competition: &League) -> Vec<String> {
    let mut region_ids = competition.required_region_ids.clone();
    if matches!(
        competition.scope,
        CompetitionScope::Domestic | CompetitionScope::Regional
    ) {
        if let Some(region_id) = &competition.region_id {
            region_ids.push(region_id.clone());
        }
    }
    region_ids.sort();
    region_ids.dedup();
    region_ids
}

/// Split a country's clubs (passed strongest-first) into divisions of
/// `division_size`, strongest tier first. A trailing remainder smaller than
/// half a division is folded up so no tier is left tiny.
pub(crate) fn split_into_divisions(
    sorted_team_ids: &[String],
    division_size: usize,
) -> Vec<Vec<String>> {
    let division_size = division_size.max(2);
    if sorted_team_ids.len() <= division_size {
        return vec![sorted_team_ids.to_vec()];
    }
    let mut divisions: Vec<Vec<String>> = sorted_team_ids
        .chunks(division_size)
        .map(<[String]>::to_vec)
        .collect();
    if divisions.len() >= 2 && divisions.last().map(Vec::len).unwrap_or(0) < division_size / 2 {
        let tail = divisions.pop().expect("len >= 2");
        divisions.last_mut().expect("len >= 1").extend(tail);
    }
    divisions
}

pub(crate) fn division_tier_name(tier: usize, division_count: usize) -> &'static str {
    if division_count <= 1 {
        "League"
    } else if tier == 0 {
        "First Division"
    } else {
        "Second Division"
    }
}

pub(crate) fn division_tier_name_key(tier: usize, division_count: usize) -> &'static str {
    if division_count <= 1 {
        "tournaments.competitions.league"
    } else if tier == 0 {
        "tournaments.competitions.firstDivision"
    } else {
        "tournaments.competitions.secondDivision"
    }
}

/// Name a division within a country's pyramid.
pub(crate) fn division_name(country: &str, tier: usize, division_count: usize) -> String {
    format!("{country} {}", division_tier_name(tier, division_count))
}

/// Default league-start month for a region. South American leagues start in
/// March, Asian in February, Oceanian in October; everything else in August.
pub(crate) fn default_season_month_for_region(region_id: &str) -> u8 {
    match region_id {
        "south-america" => 3,
        "asia" => 2,
        "oceania" => 10,
        _ => 8,
    }
}

pub(crate) fn brazil_state_region(city: &str) -> Option<&'static str> {
    match city {
        "São Paulo" | "Rio" | "Belo Horizonte" | "Santos" | "Campinas" | "Bragantino"
        | "Juiz de Fora" | "Vitória" => Some("southeast"),
        "Porto Alegre" | "Curitiba" | "Florianópolis" => Some("south"),
        "Salvador" | "Recife" | "Fortaleza" | "Natal" | "Maceió" => Some("northeast"),
        "Goiânia" | "Belém" | "Manaus" | "Cuiabá" => Some("north-central-west"),
        _ => None,
    }
}
