//! The structural vocabulary a generated world is built from.
//!
//! Which region a country belongs to, which clubs represent a nation, how a
//! country's clubs divide into tiers, and what each tier is called. These are
//! facts about the football world, not about the command layer that used to
//! host them. They live here so the pyramid builder that consumes them can too:
//! it is the only thing in the game that creates a competition structure, and
//! while it sat behind a private module in the Tauri crate, no test could reach
//! it — which is why promotion, relegation and season rollover went unguarded.

use chrono::{Datelike, Duration, Utc};
use domain::league::{CompetitionScope, League};
use domain::national_team::NationalTeam;

use crate::clock::GameClock;
use crate::game::Game;

/// Earliest year a career may start. Historical world packages recreate eras
/// decades before the modern game — a 1962 Santos world is the motivating case —
/// so the floor only needs to keep the clock inside a sane calendar range.
/// Must match `MIN_CAREER_START_YEAR` in `src/pages/MainMenu.tsx`.
pub const MIN_START_YEAR: i32 = 1900;

pub fn preseason_season_start(clock: &GameClock) -> chrono::DateTime<Utc> {
    clock.start_date + Duration::days(30)
}

pub fn preseason_league_year(clock: &GameClock) -> u32 {
    let year = clock.start_date.year() + i32::from(clock.start_date.month() == 12);
    // Only reachable for a negative year, which the start-year floor rules out.
    u32::try_from(year).unwrap_or(MIN_START_YEAR as u32)
}

pub fn infer_region_id(country_code: &str) -> String {
    crate::nations::region_for_code(country_code).to_string()
}

pub fn infer_team_region_id(team: &domain::team::Team) -> String {
    if !team.football_nation.is_empty() {
        return infer_region_id(&team.football_nation);
    }
    infer_region_id(&team.country)
}

pub fn competition_required_region_ids(competition: &League) -> Vec<String> {
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

pub fn build_national_teams(game: &Game) -> Vec<NationalTeam> {
    use std::collections::BTreeMap;

    let mut players_by_nation: BTreeMap<String, Vec<&domain::player::Player>> = BTreeMap::new();
    for player in &game.players {
        let nation = if player.football_nation.is_empty() {
            player.nationality.clone()
        } else {
            player.football_nation.clone()
        };
        players_by_nation.entry(nation).or_default().push(player);
    }

    players_by_nation
        .into_iter()
        .map(|(nation, mut players)| {
            players.sort_by_key(|player| std::cmp::Reverse(player.ovr));
            let nation_label = crate::nations::nation_display_name(&nation);
            let mut national_team = NationalTeam::new(
                format!("nt-{}", nation.to_lowercase()),
                format!("{} National Team", nation_label),
                nation.clone(),
                Some(game.region_for_country(&nation)),
            );
            national_team.squad_player_ids = players
                .into_iter()
                .take(23)
                .map(|player| player.id.clone())
                .collect();
            national_team
        })
        .collect()
}

/// Pick continental-cup entrants: the strongest clubs by reputation from each
/// region, capped so the bracket stays manageable. Entrants are returned
/// strongest-first so the top seeds receive any knockout byes.
pub fn select_continental_entrants(
    teams: &[domain::team::Team],
    per_region: usize,
    max_entrants: usize,
) -> Vec<String> {
    use std::collections::BTreeMap;

    let reputation_then_id = |left: &&domain::team::Team, right: &&domain::team::Team| {
        right
            .reputation
            .cmp(&left.reputation)
            .then_with(|| left.id.cmp(&right.id))
    };

    let mut teams_by_region: BTreeMap<String, Vec<&domain::team::Team>> = BTreeMap::new();
    for team in teams {
        teams_by_region
            .entry(infer_team_region_id(team))
            .or_default()
            .push(team);
    }

    let mut entrants: Vec<&domain::team::Team> = Vec::new();
    for regional_teams in teams_by_region.values_mut() {
        regional_teams.sort_by(reputation_then_id);
        entrants.extend(regional_teams.iter().take(per_region).copied());
    }

    entrants.sort_by(reputation_then_id);
    entrants
        .into_iter()
        .take(max_entrants)
        .map(|team| team.id.clone())
        .collect()
}

/// Target number of clubs in a division. Countries are chunked into divisions
/// of this size: a 40-club major becomes two 20-club tiers, a 20-club nation a
/// single league. Smaller imported worlds run a single league per country.
pub const TOP_DIVISION_SIZE: usize = 20;

/// Stable id of the generated world's continental club competition.
pub const CONTINENTAL_CHAMPIONS_CUP_ID: &str = "continental-champions-cup";
/// Top finishers of each first division that earn a continental berth — matches
/// the inferred `CONTINENTAL_LEAGUE_SLOTS` so built-in qualification is unchanged.
pub const CONTINENTAL_QUALIFYING_POSITIONS: u32 = 4;

/// Split a country's clubs (passed strongest-first) into divisions of
/// `division_size`, strongest tier first. A trailing remainder smaller than
/// half a division is folded up so no tier is left tiny.
pub fn split_into_divisions(sorted_team_ids: &[String], division_size: usize) -> Vec<Vec<String>> {
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

pub fn division_tier_name(tier: usize, division_count: usize) -> &'static str {
    if division_count <= 1 {
        "League"
    } else if tier == 0 {
        "First Division"
    } else {
        "Second Division"
    }
}

pub fn division_tier_name_key(tier: usize, division_count: usize) -> &'static str {
    if division_count <= 1 {
        "tournaments.competitions.league"
    } else if tier == 0 {
        "tournaments.competitions.firstDivision"
    } else {
        "tournaments.competitions.secondDivision"
    }
}

/// Name a division within a country's pyramid.
pub fn division_name(country: &str, tier: usize, division_count: usize) -> String {
    format!("{country} {}", division_tier_name(tier, division_count))
}

/// Default league-start month for a region. South American leagues start in
/// March, Asian in February, Oceanian in October; everything else in August.
pub fn default_season_month_for_region(region_id: &str) -> u8 {
    match region_id {
        "south-america" => 3,
        "asia" => 2,
        "oceania" => 10,
        _ => 8,
    }
}

pub fn brazil_state_region(city: &str) -> Option<&'static str> {
    match city {
        "São Paulo" | "Rio" | "Belo Horizonte" | "Santos" | "Campinas" | "Bragantino"
        | "Juiz de Fora" | "Vitória" => Some("southeast"),
        "Porto Alegre" | "Curitiba" | "Florianópolis" => Some("south"),
        "Salvador" | "Recife" | "Fortaleza" | "Natal" | "Maceió" => Some("northeast"),
        "Goiânia" | "Belém" | "Manaus" | "Cuiabá" => Some("north-central-west"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_continental_entrants_takes_top_clubs_per_region_by_reputation() {
        let make = |id: &str, nation: &str, reputation: u32| {
            let mut team = domain::team::Team::new(
                id.to_string(),
                id.to_string(),
                id.to_string(),
                "Country".to_string(),
                "City".to_string(),
                "Stadium".to_string(),
                10_000,
            );
            team.football_nation = nation.to_string();
            team.reputation = reputation;
            team
        };
        let teams = vec![
            make("eng-a", "GB", 900),
            make("eng-b", "GB", 800),
            make("eng-c", "GB", 700), // third in Europe -> excluded by per_region
            make("bra-a", "BR", 850),
            make("bra-b", "BR", 600),
        ];

        let entrants = select_continental_entrants(&teams, 2, 16);

        // Top two per region, ordered strongest-first across regions.
        assert_eq!(
            entrants,
            vec![
                "eng-a".to_string(),
                "bra-a".to_string(),
                "eng-b".to_string(),
                "bra-b".to_string(),
            ]
        );
    }

    #[test]
    fn split_into_divisions_chunks_a_major_into_two_tiers() {
        let clubs: Vec<String> = (0..40).map(|i| format!("club-{i:02}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 2);
        assert_eq!(divisions[0].len(), 20);
        assert_eq!(divisions[1].len(), 20);
        // Strongest tier first; the second tier starts where the first ends.
        assert_eq!(divisions[0][0], "club-00");
        assert_eq!(divisions[1][0], "club-20");
    }

    #[test]
    fn split_into_divisions_keeps_a_single_league_at_division_size() {
        let clubs: Vec<String> = (0..20).map(|i| format!("club-{i:02}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 1);
        assert_eq!(divisions[0].len(), 20);
    }

    #[test]
    fn split_into_divisions_keeps_a_single_tier_for_small_countries() {
        let clubs: Vec<String> = (0..7).map(|i| format!("club-{i}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 1);
        assert_eq!(divisions[0].len(), 7);
    }

    #[test]
    fn split_into_divisions_folds_a_tiny_remainder_up() {
        // 25 clubs → 20 + 5; the 5-club tail folds up rather than forming a
        // tiny second division.
        let clubs: Vec<String> = (0..25).map(|i| format!("club-{i:02}")).collect();

        let divisions = split_into_divisions(&clubs, 20);

        assert_eq!(divisions.len(), 1);
        assert_eq!(divisions[0].len(), 25);
    }

    #[test]
    fn select_continental_entrants_caps_the_field() {
        let teams: Vec<domain::team::Team> = (0..10)
            .map(|index| {
                let mut team = domain::team::Team::new(
                    format!("eng-{index}"),
                    format!("Club {index}"),
                    format!("C{index}"),
                    "Country".to_string(),
                    "City".to_string(),
                    "Stadium".to_string(),
                    10_000,
                );
                team.football_nation = "GB".to_string();
                team.reputation = 1000 - index as u32;
                team
            })
            .collect();

        let entrants = select_continental_entrants(&teams, 8, 4);

        assert_eq!(entrants.len(), 4);
        assert_eq!(entrants[0], "eng-0", "strongest club is seeded first");
    }

    #[test]
    fn brazil_state_region_covers_all_standard_br_cities() {
        // All cities from STANDARD_NATIONS BR entry must map to a region so that
        // state-series competitions are generated for every club location.
        let br_cities = [
            "São Paulo",
            "Rio",
            "Belo Horizonte",
            "Porto Alegre",
            "Salvador",
            "Recife",
            "Curitiba",
            "Fortaleza",
            "Goiânia",
            "Santos",
            "Campinas",
            "Belém",
            "Manaus",
            "Vitória",
            "Natal",
            "Florianópolis",
            "Cuiabá",
            "Maceió",
            "Bragantino",
            "Juiz de Fora",
        ];
        for city in br_cities {
            assert!(
                brazil_state_region(city).is_some(),
                "brazil_state_region returned None for BR city: {city}"
            );
        }
        assert_eq!(
            brazil_state_region("Vitória"),
            Some("southeast"),
            "Vitória (ES) belongs in the southeast region, not northeast"
        );
    }
}
