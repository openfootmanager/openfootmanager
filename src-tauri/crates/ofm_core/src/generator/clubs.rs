//! Procedural club generation.
//!
//! Rather than ship a fixed handful of named clubs, the generator fills every
//! catalogued nation with a full set of fictional clubs so each country fields
//! a real league (and, for the strongest footballing nations, two divisions).
//! Club names are built from per-nation city pools combined with culturally
//! flavoured naming patterns, so a 20-club division reads like a believable
//! domestic competition without hand-authoring every team.

use rand::{Rng, RngExt};
use std::collections::HashSet;

use super::definitions::{TeamColorsDef, TeamDef};

/// Naming conventions drive the club-name patterns used for a nation, giving
/// each footballing culture its own flavour (United/City vs CF/Real vs Calcio…).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingStyle {
    English,
    Scottish,
    Spanish,
    Italian,
    German,
    French,
    Portuguese,
    Dutch,
    Nordic,
    Balkan,
    LatinAmerican,
    Brazilian,
    Generic,
}

impl NamingStyle {
    /// Name patterns for the style. `{}` is replaced by the city name. Ordered
    /// so the first patterns (plain "{} FC") are the most common, with prefixes
    /// adding variety as a league grows.
    fn patterns(self) -> &'static [&'static str] {
        match self {
            NamingStyle::English => &[
                "{} FC", "{} United", "{} City", "{} Town", "{} Athletic", "{} Rovers",
                "{} Wanderers", "{} Albion", "{} County",
            ],
            NamingStyle::Scottish => &[
                "{} FC", "{} United", "{} City", "{} Rovers", "{} Athletic", "{} County",
                "{} Thistle", "Heart of {}",
            ],
            NamingStyle::Spanish => &[
                "{} CF", "Real {}", "Atlético {}", "Deportivo {}", "Racing {}", "{} FC",
                "Club {}", "Unión {}",
            ],
            NamingStyle::Italian => &[
                "{} Calcio", "AC {}", "{} FC", "US {}", "Inter {}", "Virtus {}", "Real {}",
                "Pro {}",
            ],
            NamingStyle::German => &[
                "FC {}", "{} 04", "SV {}", "VfB {}", "Borussia {}", "{} United", "TSV {}",
                "SC {}",
            ],
            NamingStyle::French => &[
                "{} FC", "Olympique {}", "AS {}", "Racing {}", "Stade {}", "RC {}", "FC {}",
                "US {}",
            ],
            NamingStyle::Portuguese => &[
                "{} FC", "Sporting {}", "Académico {}", "União {}", "CD {}", "{} SC", "Os {}",
                "Real {}",
            ],
            NamingStyle::Dutch => &[
                "{} FC", "FC {}", "SV {}", "VV {}", "{} United", "Sparta {}", "{} City",
                "Go Ahead {}",
            ],
            NamingStyle::Nordic => &[
                "{} IF", "IFK {}", "{} FF", "{} BK", "{} FC", "{} United", "{} SK", "{} AIK",
            ],
            NamingStyle::Balkan => &[
                "NK {}", "{} FC", "HNK {}", "Dinamo {}", "Hajduk {}", "FK {}", "{} United",
                "Slaven {}",
            ],
            NamingStyle::LatinAmerican => &[
                "Club {}", "{} FC", "Atlético {}", "Racing {}", "Deportivo {}", "Unión {}",
                "Independiente {}", "Nacional {}",
            ],
            NamingStyle::Brazilian => &[
                "{} Esporte Clube", "Associação Atlética {}", "Grêmio Esportivo {}",
                "Clube Atlético {}", "{} Futebol Clube", "União Esportiva {}",
            ],
            NamingStyle::Generic => &[
                "{} FC", "{} United", "{} City", "Club {}", "{} Athletic", "Sporting {}",
                "Real {}", "{} SC",
            ],
        }
    }
}

/// Per-nation generation spec: where clubs come from and how strong the league
/// is. `tiers` is 1 (a single division) or 2 (a major nation with a second
/// division below the top flight). `strength` (1–5) seeds the reputation band.
#[derive(Debug, Clone, Copy)]
pub struct NationGen {
    pub code: &'static str,
    pub cities: &'static [&'static str],
    pub style: NamingStyle,
    pub tiers: usize,
    pub strength: u8,
}

/// Configuration for a procedurally generated world.
#[derive(Debug, Clone)]
pub struct WorldGenConfig {
    /// Clubs in each division (20 by default).
    pub clubs_per_division: usize,
    pub nations: Vec<NationGen>,
}

impl WorldGenConfig {
    /// The full shipped world: every catalogued nation with curated content.
    pub fn standard() -> Self {
        Self {
            clubs_per_division: 20,
            nations: STANDARD_NATIONS.to_vec(),
        }
    }

    /// A tiny world for fast tests: two small single-division nations.
    pub fn compact() -> Self {
        Self {
            clubs_per_division: 4,
            nations: vec![STANDARD_NATIONS[0], STANDARD_NATIONS[1]],
        }
    }

    /// Total clubs this config will generate.
    pub fn total_clubs(&self) -> usize {
        self.nations
            .iter()
            .map(|nation| self.clubs_per_division * nation.tiers)
            .sum()
    }
}

const COLOR_PALETTE: &[(&str, &str)] = &[
    ("#dc2626", "#ffffff"),
    ("#1d4ed8", "#ffffff"),
    ("#16a34a", "#ffffff"),
    ("#000000", "#ffffff"),
    ("#eab308", "#1e3a5f"),
    ("#7c3aed", "#fbbf24"),
    ("#0ea5e9", "#1e3a5f"),
    ("#b91c1c", "#fbbf24"),
    ("#9f1239", "#1d4ed8"),
    ("#1e3a5f", "#dc2626"),
    ("#15803d", "#000000"),
    ("#ea580c", "#1e3a5f"),
];

const PLAY_STYLES: &[&str] = &[
    "Possession",
    "Attacking",
    "HighPress",
    "Counter",
    "Balanced",
    "Defensive",
];

/// Build a short 3-letter club code from a name, skipping common prefixes/
/// suffixes so "Real Madrid CF" → "MAD" rather than "RMC".
fn short_code(name: &str) -> String {
    const SKIP: &[&str] = &[
        "FC", "AC", "AS", "SV", "CF", "CD", "SC", "US", "RC", "NK", "FK", "BK", "IF", "FF", "SK",
        "TSV", "VfB", "HNK", "IFK", "AIK", "VV", "Pro", "Os", "Esporte", "Clube",
        "Associação", "Atlética", "Grêmio", "Esportivo", "Atlético", "Futebol", "União",
        "Esportiva",
    ];
    // Initial of each significant word, restricted to ASCII so accented names
    // (São, Málaga, Évora) still yield clean three-letter codes.
    let initials: String = name
        .split_whitespace()
        .filter(|word| !SKIP.contains(word))
        .filter_map(|word| ascii_letters(word).chars().next())
        .collect::<String>()
        .to_ascii_uppercase();
    if initials.len() >= 3 {
        return initials.chars().take(3).collect();
    }

    // Fall back to the first ASCII letters of the whole name, padding the rare
    // ultra-short case so a code is always exactly three letters.
    let mut code: String = ascii_letters(name)
        .chars()
        .take(3)
        .collect::<String>()
        .to_ascii_uppercase();
    while code.len() < 3 {
        code.push('X');
    }
    code
}

fn ascii_letters(value: &str) -> String {
    value
        .chars()
        .filter_map(|c| match c {
            'A'..='Z' | 'a'..='z' => Some(c),
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => Some('a'),
            'Ç' | 'ç' => Some('c'),
            'È' | 'É' | 'Ê' | 'Ë' | 'è' | 'é' | 'ê' | 'ë' => Some('e'),
            'Ì' | 'Í' | 'Î' | 'Ï' | 'ì' | 'í' | 'î' | 'ï' => Some('i'),
            'Ñ' | 'ñ' => Some('n'),
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'ò' | 'ó' | 'ô' | 'õ' | 'ö' => Some('o'),
            'Ù' | 'Ú' | 'Û' | 'Ü' | 'ù' | 'ú' | 'û' | 'ü' => Some('u'),
            _ => None,
        })
        .collect()
}

fn unique_short_code(base_code: &str, used_codes: &HashSet<String>) -> String {
    if !used_codes.contains(base_code) {
        return base_code.to_string();
    }

    let base = base_code.as_bytes();
    for suffix in b'A'..=b'Z' {
        let candidate = format!(
            "{}{}{}",
            char::from(base[0]),
            char::from(base[1]),
            char::from(suffix)
        );
        if !used_codes.contains(&candidate) {
            return candidate;
        }
    }

    for middle in b'A'..=b'Z' {
        for suffix in b'A'..=b'Z' {
            let candidate = format!(
                "{}{}{}",
                char::from(base[0]),
                char::from(middle),
                char::from(suffix)
            );
            if !used_codes.contains(&candidate) {
                return candidate;
            }
        }
    }

    for first in b'A'..=b'Z' {
        for middle in b'A'..=b'Z' {
            for suffix in b'A'..=b'Z' {
                let candidate = format!(
                    "{}{}{}",
                    char::from(first),
                    char::from(middle),
                    char::from(suffix)
                );
                if !used_codes.contains(&candidate) {
                    return candidate;
                }
            }
        }
    }

    panic!("three-letter club code space exhausted");
}

/// Generate `count` distinct (club name, city) pairs for a nation, spreading
/// across cities first so a league looks geographically diverse before reusing
/// a city with a different pattern.
fn club_names(nation: &NationGen, count: usize) -> Vec<(String, String)> {
    let patterns = nation.style.patterns();
    let mut out = Vec::with_capacity(count);
    let mut seen = HashSet::new();

    'outer: for index in 0..count.saturating_mul(2) {
        let city = nation.cities[index % nation.cities.len()];
        let pattern = patterns[(index + index / nation.cities.len()) % patterns.len()];
        {
            let name = pattern.replace("{}", city);
            if seen.insert(name.clone()) {
                out.push((name, (*city).to_string()));
                if out.len() == count {
                    break 'outer;
                }
            }
        }
    }

    // Safety net for an over-large count: append numbered variants.
    let mut suffix = 2;
    while out.len() < count {
        for city in nation.cities {
            let name = format!("{} FC {}", city, suffix);
            if seen.insert(name.clone()) {
                out.push((name, (*city).to_string()));
                if out.len() == count {
                    break;
                }
            }
        }
        suffix += 1;
    }

    out
}

/// Reputation band centre for a club ranked `index` of `total` in its nation,
/// strongest first. Stronger nations sit higher; within a nation reputation
/// declines down the pyramid so divisions seed by quality.
fn reputation_center(strength: u8, index: usize, total: usize) -> u32 {
    let top = 300 + (strength as u32) * 110; // strength 5 → 850, strength 1 → 410
    let floor = top.saturating_sub(420).max(120);
    if total <= 1 {
        return top;
    }
    let span = top - floor;
    top - span * (index as u32) / (total as u32 - 1)
}

/// Build the team definitions for a whole world from a config.
pub fn generate_club_defs(config: &WorldGenConfig, rng: &mut impl Rng) -> Vec<TeamDef> {
    let mut defs = Vec::with_capacity(config.total_clubs());

    for nation in &config.nations {
        let mut used_codes = HashSet::new();
        let total = config.clubs_per_division * nation.tiers;
        let names = club_names(nation, total);
        for (index, (name, city)) in names.into_iter().enumerate() {
            let center = reputation_center(nation.strength, index, total);
            let rep_lo = center.saturating_sub(25).max(80);
            let rep_hi = (center + 25).min(950).max(rep_lo + 1);
            let fin_lo = (center as i64) * 4_000;
            let fin_hi = (center as i64) * 9_000;
            let (primary, secondary) = COLOR_PALETTE[rng.random_range(0..COLOR_PALETTE.len())];
            let play_style = PLAY_STYLES[rng.random_range(0..PLAY_STYLES.len())];

            let base_code = short_code(&name);
            let unique_code = unique_short_code(&base_code, &used_codes);
            used_codes.insert(unique_code.clone());
            defs.push(TeamDef {
                id: String::new(),
                short_name: unique_code,
                name,
                city: city.clone(),
                country: nation.code.to_string(),
                colors: TeamColorsDef {
                    primary: primary.to_string(),
                    secondary: secondary.to_string(),
                },
                play_style: play_style.to_string(),
                stadium_name: format!("{city} Arena"),
                reputation_range: Some([rep_lo, rep_hi]),
                finance_range: Some([fin_lo, fin_hi]),
                logo: None,
                kit_pattern: None,
            });
        }
    }

    defs
}

// ---------------------------------------------------------------------------
// Standard nation content
// ---------------------------------------------------------------------------

/// Nations populated with curated city pools. Majors (tiers = 2) get a second
/// division. Further confederations are added in a follow-up slice.
pub const STANDARD_NATIONS: &[NationGen] = &[
    NationGen {
        code: "ENG",
        style: NamingStyle::English,
        tiers: 2,
        strength: 5,
        cities: &[
            "London", "Manchester", "Liverpool", "Birmingham", "Leeds", "Newcastle", "Sheffield",
            "Bristol", "Nottingham", "Leicester", "Southampton", "Brighton", "Sunderland",
            "Norwich", "Portsmouth", "Hull", "Coventry", "Blackburn", "Wolverhampton", "Derby",
        ],
    },
    NationGen {
        code: "ES",
        style: NamingStyle::Spanish,
        tiers: 2,
        strength: 5,
        cities: &[
            "Madrid", "Barcelona", "Valencia", "Seville", "Bilbao", "Málaga", "Zaragoza", "Vigo",
            "Gijón", "Granada", "Murcia", "Valladolid", "Pamplona", "Cádiz", "Córdoba", "Almería",
            "Getafe", "Elche", "Mallorca", "Las Palmas",
        ],
    },
    NationGen {
        code: "DE",
        style: NamingStyle::German,
        tiers: 2,
        strength: 5,
        cities: &[
            "Munich", "Dortmund", "Berlin", "Hamburg", "Cologne", "Frankfurt", "Stuttgart",
            "Leipzig", "Bremen", "Hannover", "Nuremberg", "Gladbach", "Leverkusen", "Wolfsburg",
            "Freiburg", "Mainz", "Augsburg", "Bochum", "Hoffenheim", "Kiel",
        ],
    },
    NationGen {
        code: "IT",
        style: NamingStyle::Italian,
        tiers: 2,
        strength: 5,
        cities: &[
            "Milan", "Rome", "Turin", "Naples", "Florence", "Genoa", "Bologna", "Verona",
            "Bergamo", "Udine", "Cagliari", "Palermo", "Bari", "Parma", "Sassuolo", "Empoli",
            "Lecce", "Venice", "Como", "Monza",
        ],
    },
    NationGen {
        code: "FR",
        style: NamingStyle::French,
        tiers: 2,
        strength: 4,
        cities: &[
            "Paris", "Marseille", "Lyon", "Lille", "Monaco", "Nice", "Bordeaux", "Nantes",
            "Rennes", "Lens", "Strasbourg", "Saint-Étienne", "Montpellier", "Toulouse", "Reims",
            "Brest", "Angers", "Metz", "Nîmes", "Auxerre",
        ],
    },
    NationGen {
        code: "BR",
        style: NamingStyle::Brazilian,
        tiers: 2,
        strength: 4,
        cities: &[
            "São Paulo", "Rio", "Belo Horizonte", "Porto Alegre", "Salvador", "Recife", "Curitiba",
            "Fortaleza", "Goiânia", "Santos", "Campinas", "Belém", "Manaus", "Vitória", "Natal",
            "Florianópolis", "Cuiabá", "Maceió", "Bragantino", "Juiz de Fora",
        ],
    },
    NationGen {
        code: "PT",
        style: NamingStyle::Portuguese,
        tiers: 1,
        strength: 4,
        cities: &[
            "Lisbon", "Porto", "Braga", "Guimarães", "Coimbra", "Faro", "Funchal", "Setúbal",
            "Aveiro", "Leiria", "Viseu", "Portimão", "Évora", "Famalicão", "Chaves", "Vizela",
        ],
    },
    NationGen {
        code: "NL",
        style: NamingStyle::Dutch,
        tiers: 1,
        strength: 4,
        cities: &[
            "Amsterdam", "Rotterdam", "Eindhoven", "Utrecht", "Alkmaar", "Enschede", "Groningen",
            "Tilburg", "Heerenveen", "Nijmegen", "Arnhem", "Breda", "Sittard", "Waalwijk",
            "Almelo", "Zwolle",
        ],
    },
    NationGen {
        code: "BE",
        style: NamingStyle::Generic,
        tiers: 1,
        strength: 3,
        cities: &[
            "Brussels", "Bruges", "Antwerp", "Ghent", "Liège", "Charleroi", "Genk", "Leuven",
            "Mechelen", "Kortrijk", "Ostend", "Sint-Truiden", "Eupen", "Waregem", "Mouscron",
            "Lokeren",
        ],
    },
    NationGen {
        code: "SCO",
        style: NamingStyle::Scottish,
        tiers: 1,
        strength: 3,
        cities: &[
            "Glasgow", "Edinburgh", "Aberdeen", "Dundee", "Perth", "Inverness", "Kilmarnock",
            "Motherwell", "Paisley", "Falkirk", "Hamilton", "Livingston", "Dingwall", "Stirling",
            "Greenock", "Dunfermline",
        ],
    },
    NationGen {
        code: "AR",
        style: NamingStyle::LatinAmerican,
        tiers: 1,
        strength: 4,
        cities: &[
            "Buenos Aires", "Rosario", "Córdoba", "La Plata", "Mendoza", "Avellaneda", "Santa Fe",
            "Mar del Plata", "Tucumán", "Salta", "San Juan", "Bahía Blanca", "Quilmes", "Lanús",
            "Banfield", "Tigre",
        ],
    },
    NationGen {
        code: "HR",
        style: NamingStyle::Balkan,
        tiers: 1,
        strength: 3,
        cities: &[
            "Zagreb", "Split", "Rijeka", "Osijek", "Zadar", "Pula", "Varaždin", "Šibenik",
            "Karlovac", "Dubrovnik", "Vinkovci", "Slavonski Brod", "Velika Gorica", "Koprivnica",
            "Samobor", "Gorica",
        ],
    },
    NationGen {
        code: "SE",
        style: NamingStyle::Nordic,
        tiers: 1,
        strength: 3,
        cities: &[
            "Stockholm", "Gothenburg", "Malmö", "Uppsala", "Norrköping", "Helsingborg", "Örebro",
            "Linköping", "Västerås", "Sundsvall", "Kalmar", "Halmstad", "Gävle", "Borås",
            "Trelleborg", "Falkenberg",
        ],
    },
    NationGen {
        code: "IE",
        style: NamingStyle::English,
        tiers: 1,
        strength: 2,
        cities: &[
            "Dublin", "Cork", "Limerick", "Galway", "Waterford", "Sligo", "Drogheda", "Dundalk",
            "Bray", "Athlone", "Wexford", "Longford", "Tallaght", "Finglas", "Cobh", "Derry",
        ],
    },
    NationGen {
        code: "WAL",
        style: NamingStyle::English,
        tiers: 1,
        strength: 2,
        cities: &[
            "Cardiff", "Swansea", "Wrexham", "Newport", "Bangor", "Barry", "Merthyr", "Llanelli",
            "Aberystwyth", "Caernarfon", "Haverfordwest", "Bala", "Penybont", "Flint", "Rhyl",
            "Newtown",
        ],
    },
    NationGen {
        code: "NIR",
        style: NamingStyle::English,
        tiers: 1,
        strength: 2,
        cities: &[
            "Belfast", "Derry", "Lisburn", "Ballymena", "Coleraine", "Portadown", "Newry",
            "Larne", "Bangor", "Glenavon", "Carrick", "Dungannon", "Cliftonville", "Crusaders",
            "Warrenpoint", "Loughgall",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nations;

    #[test]
    fn standard_config_gives_every_nation_a_full_pyramid() {
        let config = WorldGenConfig::standard();
        for nation in &config.nations {
            let expected = config.clubs_per_division * nation.tiers;
            let mut rng = rand::rng();
            let defs = generate_club_defs(
                &WorldGenConfig {
                    clubs_per_division: config.clubs_per_division,
                    nations: vec![*nation],
                },
                &mut rng,
            );
            assert_eq!(
                defs.len(),
                expected,
                "{} should generate {} clubs",
                nation.code,
                expected
            );
        }
    }

    #[test]
    fn club_names_are_unique_within_a_nation() {
        let mut rng = rand::rng();
        let defs = generate_club_defs(&WorldGenConfig::standard(), &mut rng);
        for nation in WorldGenConfig::standard().nations {
            let names: Vec<&str> = defs
                .iter()
                .filter(|def| def.country == nation.code)
                .map(|def| def.name.as_str())
                .collect();
            let unique: HashSet<&str> = names.iter().copied().collect();
            assert_eq!(
                names.len(),
                unique.len(),
                "duplicate club name generated for {}",
                nation.code
            );
        }
    }

    #[test]
    fn majors_get_two_divisions_worth_of_clubs() {
        let config = WorldGenConfig::standard();
        let eng = config.nations.iter().find(|n| n.code == "ENG").unwrap();
        assert_eq!(eng.tiers, 2);
        assert!(config.clubs_per_division >= 18, "leagues should be realistic");
    }

    #[test]
    fn every_standard_nation_is_in_the_catalog_with_a_region() {
        for nation in STANDARD_NATIONS {
            let region = nations::region_for_code(nation.code);
            assert!(
                !region.is_empty(),
                "{} has no region mapping",
                nation.code
            );
        }
    }

    #[test]
    fn reputation_descends_down_the_pyramid() {
        let top = reputation_center(5, 0, 40);
        let mid = reputation_center(5, 20, 40);
        let bottom = reputation_center(5, 39, 40);
        assert!(top > mid && mid > bottom, "{top} {mid} {bottom}");
    }

    #[test]
    fn short_codes_are_three_uppercase_letters() {
        let mut rng = rand::rng();
        let defs = generate_club_defs(&WorldGenConfig::compact(), &mut rng);
        for def in &defs {
            assert_eq!(def.short_name.chars().count(), 3, "{}", def.name);
            assert!(
                def.short_name.chars().all(|c| c.is_ascii_uppercase()),
                "{} → {}",
                def.name,
                def.short_name
            );
        }
    }

    #[test]
    fn short_code_deduplication_extends_beyond_one_suffix_letter() {
        let mut used = HashSet::new();
        for suffix in b'A'..=b'Z' {
            used.insert(format!("AA{}", char::from(suffix)));
        }

        let code = unique_short_code("AAA", &used);

        assert_eq!(code, "ABA");
        assert!(!used.contains(&code));
    }

    #[test]
    fn brazilian_pyramid_uses_varied_local_names_and_unique_codes() {
        let brazil = *STANDARD_NATIONS.iter().find(|nation| nation.code == "BR").unwrap();
        let mut rng = rand::rng();
        let defs = generate_club_defs(&WorldGenConfig {
            clubs_per_division: 20,
            nations: vec![brazil],
        }, &mut rng);
        assert_eq!(defs.len(), 40);
        assert!(defs.iter().all(|club| !club.name.starts_with("Club ") && !club.name.ends_with(" FC")));
        let forms: HashSet<&str> = defs.iter().filter_map(|club| {
            ["Esporte Clube", "Associação Atlética", "Grêmio Esportivo", "Clube Atlético", "Futebol Clube", "União Esportiva"]
                .into_iter().find(|form| club.name.contains(form))
        }).collect();
        assert!(forms.len() >= 4, "expected several Brazilian naming forms");
        let codes: HashSet<&str> = defs.iter().map(|club| club.short_name.as_str()).collect();
        assert_eq!(codes.len(), 40);
    }
}
