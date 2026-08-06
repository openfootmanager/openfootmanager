//! Catalog of football nations the game can field internationally, beyond
//! whatever nationalities a world's clubs happen to contain. Used to fill a
//! World Cup field by synthesising national squads for missing nations.
//!
//! Two lists, kept deliberately separate:
//! - [`NATION_CATALOG`] — the curated World Cup pool. Qualifying synthesises its
//!   field from exactly these nations, so its size drives qualifying scale.
//! - [`ADDITIONAL_NATIONS`] — the rest of the FIFA membership. Selectable in the
//!   editor and accepted by the importer, but *not* World Cup entrants.
//!
//! [`all_nations`] is the union and the single source of truth for which
//! nationalities are valid/selectable (see #270); the World Cup deliberately
//! reads only `NATION_CATALOG`.

pub struct NationDef {
    pub code: &'static str,
    pub name: &'static str,
    pub region_id: &'static str,
}

/// The World Cup pool: real football nations across every confederation,
/// strongest footballing traditions first within each region (order is used as
/// a soft seeding hint when squads are otherwise equal). Qualifying builds its
/// field from this list, so keep it curated — adding a nation here enters it
/// into the World Cup. Merely-selectable nations belong in [`ADDITIONAL_NATIONS`].
pub const NATION_CATALOG: &[NationDef] = &[
    // Europe
    NationDef { code: "FR", name: "France", region_id: "europe" },
    NationDef { code: "ENG", name: "England", region_id: "europe" },
    NationDef { code: "ES", name: "Spain", region_id: "europe" },
    NationDef { code: "DE", name: "Germany", region_id: "europe" },
    NationDef { code: "IT", name: "Italy", region_id: "europe" },
    NationDef { code: "PT", name: "Portugal", region_id: "europe" },
    NationDef { code: "NL", name: "Netherlands", region_id: "europe" },
    NationDef { code: "BE", name: "Belgium", region_id: "europe" },
    NationDef { code: "HR", name: "Croatia", region_id: "europe" },
    NationDef { code: "CH", name: "Switzerland", region_id: "europe" },
    NationDef { code: "DK", name: "Denmark", region_id: "europe" },
    NationDef { code: "AT", name: "Austria", region_id: "europe" },
    NationDef { code: "UA", name: "Ukraine", region_id: "europe" },
    NationDef { code: "TR", name: "Türkiye", region_id: "europe" },
    NationDef { code: "PL", name: "Poland", region_id: "europe" },
    NationDef { code: "RS", name: "Serbia", region_id: "europe" },
    NationDef { code: "SE", name: "Sweden", region_id: "europe" },
    NationDef { code: "NO", name: "Norway", region_id: "europe" },
    NationDef { code: "CZ", name: "Czechia", region_id: "europe" },
    NationDef { code: "GR", name: "Greece", region_id: "europe" },
    NationDef { code: "HU", name: "Hungary", region_id: "europe" },
    NationDef { code: "RO", name: "Romania", region_id: "europe" },
    NationDef { code: "SCO", name: "Scotland", region_id: "europe" },
    NationDef { code: "WAL", name: "Wales", region_id: "europe" },
    NationDef { code: "IE", name: "Ireland", region_id: "europe" },
    NationDef { code: "NIR", name: "Northern Ireland", region_id: "europe" },
    // South America
    NationDef { code: "BR", name: "Brazil", region_id: "south-america" },
    NationDef { code: "AR", name: "Argentina", region_id: "south-america" },
    NationDef { code: "UY", name: "Uruguay", region_id: "south-america" },
    NationDef { code: "CO", name: "Colombia", region_id: "south-america" },
    NationDef { code: "CL", name: "Chile", region_id: "south-america" },
    NationDef { code: "PE", name: "Peru", region_id: "south-america" },
    NationDef { code: "EC", name: "Ecuador", region_id: "south-america" },
    NationDef { code: "PY", name: "Paraguay", region_id: "south-america" },
    NationDef { code: "VE", name: "Venezuela", region_id: "south-america" },
    NationDef { code: "BO", name: "Bolivia", region_id: "south-america" },
    // North America
    NationDef { code: "MX", name: "Mexico", region_id: "north-america" },
    NationDef { code: "US", name: "United States", region_id: "north-america" },
    NationDef { code: "CA", name: "Canada", region_id: "north-america" },
    // Central America & Caribbean
    NationDef { code: "CR", name: "Costa Rica", region_id: "central-america" },
    NationDef { code: "PA", name: "Panama", region_id: "central-america" },
    NationDef { code: "HN", name: "Honduras", region_id: "central-america" },
    NationDef { code: "JM", name: "Jamaica", region_id: "central-america" },
    NationDef { code: "GT", name: "Guatemala", region_id: "central-america" },
    NationDef { code: "SV", name: "El Salvador", region_id: "central-america" },
    // Africa
    NationDef { code: "MA", name: "Morocco", region_id: "africa" },
    NationDef { code: "SN", name: "Senegal", region_id: "africa" },
    NationDef { code: "EG", name: "Egypt", region_id: "africa" },
    NationDef { code: "NG", name: "Nigeria", region_id: "africa" },
    NationDef { code: "CM", name: "Cameroon", region_id: "africa" },
    NationDef { code: "GH", name: "Ghana", region_id: "africa" },
    NationDef { code: "CI", name: "Ivory Coast", region_id: "africa" },
    NationDef { code: "DZ", name: "Algeria", region_id: "africa" },
    NationDef { code: "TN", name: "Tunisia", region_id: "africa" },
    NationDef { code: "ZA", name: "South Africa", region_id: "africa" },
    // Asia
    NationDef { code: "JP", name: "Japan", region_id: "asia" },
    NationDef { code: "KR", name: "South Korea", region_id: "asia" },
    NationDef { code: "IR", name: "Iran", region_id: "asia" },
    NationDef { code: "SA", name: "Saudi Arabia", region_id: "asia" },
    NationDef { code: "QA", name: "Qatar", region_id: "asia" },
    NationDef { code: "AE", name: "United Arab Emirates", region_id: "asia" },
    NationDef { code: "UZ", name: "Uzbekistan", region_id: "asia" },
    NationDef { code: "CN", name: "China", region_id: "asia" },
    NationDef { code: "IQ", name: "Iraq", region_id: "asia" },
    NationDef { code: "TH", name: "Thailand", region_id: "asia" },
    // Oceania
    NationDef { code: "AU", name: "Australia", region_id: "oceania" },
    NationDef { code: "NZ", name: "New Zealand", region_id: "oceania" },
];

/// The rest of the FIFA membership: real nations selectable in the world editor
/// / manager creation and accepted by the package importer (see #270), but not
/// part of the World Cup pool, so qualifying scale and balance are unchanged.
/// Codes are ISO 3166-1 alpha-2 (the UK home nations live in [`NATION_CATALOG`]
/// under football identities). Grouped by region for readability only.
pub const ADDITIONAL_NATIONS: &[NationDef] = &[
    // Europe (UEFA)
    NationDef { code: "AL", name: "Albania", region_id: "europe" },
    NationDef { code: "AD", name: "Andorra", region_id: "europe" },
    NationDef { code: "AM", name: "Armenia", region_id: "europe" },
    NationDef { code: "AZ", name: "Azerbaijan", region_id: "europe" },
    NationDef { code: "BY", name: "Belarus", region_id: "europe" },
    NationDef { code: "BA", name: "Bosnia and Herzegovina", region_id: "europe" },
    NationDef { code: "BG", name: "Bulgaria", region_id: "europe" },
    NationDef { code: "CY", name: "Cyprus", region_id: "europe" },
    NationDef { code: "EE", name: "Estonia", region_id: "europe" },
    NationDef { code: "FO", name: "Faroe Islands", region_id: "europe" },
    NationDef { code: "FI", name: "Finland", region_id: "europe" },
    NationDef { code: "GE", name: "Georgia", region_id: "europe" },
    NationDef { code: "GI", name: "Gibraltar", region_id: "europe" },
    NationDef { code: "IS", name: "Iceland", region_id: "europe" },
    NationDef { code: "IL", name: "Israel", region_id: "europe" },
    NationDef { code: "KZ", name: "Kazakhstan", region_id: "europe" },
    NationDef { code: "XK", name: "Kosovo", region_id: "europe" },
    NationDef { code: "LV", name: "Latvia", region_id: "europe" },
    NationDef { code: "LI", name: "Liechtenstein", region_id: "europe" },
    NationDef { code: "LT", name: "Lithuania", region_id: "europe" },
    NationDef { code: "LU", name: "Luxembourg", region_id: "europe" },
    NationDef { code: "MT", name: "Malta", region_id: "europe" },
    NationDef { code: "MD", name: "Moldova", region_id: "europe" },
    NationDef { code: "ME", name: "Montenegro", region_id: "europe" },
    NationDef { code: "MK", name: "North Macedonia", region_id: "europe" },
    NationDef { code: "SM", name: "San Marino", region_id: "europe" },
    NationDef { code: "SK", name: "Slovakia", region_id: "europe" },
    NationDef { code: "SI", name: "Slovenia", region_id: "europe" },
    // Central America & Caribbean (CONCACAF)
    NationDef { code: "BZ", name: "Belize", region_id: "central-america" },
    NationDef { code: "NI", name: "Nicaragua", region_id: "central-america" },
    NationDef { code: "CU", name: "Cuba", region_id: "central-america" },
    NationDef { code: "HT", name: "Haiti", region_id: "central-america" },
    NationDef { code: "TT", name: "Trinidad and Tobago", region_id: "central-america" },
    NationDef { code: "DO", name: "Dominican Republic", region_id: "central-america" },
    NationDef { code: "CW", name: "Curaçao", region_id: "central-america" },
    NationDef { code: "GY", name: "Guyana", region_id: "central-america" },
    NationDef { code: "SR", name: "Suriname", region_id: "central-america" },
    NationDef { code: "BB", name: "Barbados", region_id: "central-america" },
    NationDef { code: "AG", name: "Antigua and Barbuda", region_id: "central-america" },
    NationDef { code: "GD", name: "Grenada", region_id: "central-america" },
    NationDef { code: "KN", name: "Saint Kitts and Nevis", region_id: "central-america" },
    NationDef { code: "LC", name: "Saint Lucia", region_id: "central-america" },
    NationDef { code: "VC", name: "Saint Vincent and the Grenadines", region_id: "central-america" },
    NationDef { code: "DM", name: "Dominica", region_id: "central-america" },
    NationDef { code: "AW", name: "Aruba", region_id: "central-america" },
    NationDef { code: "BS", name: "Bahamas", region_id: "central-america" },
    NationDef { code: "BM", name: "Bermuda", region_id: "central-america" },
    NationDef { code: "KY", name: "Cayman Islands", region_id: "central-america" },
    NationDef { code: "PR", name: "Puerto Rico", region_id: "central-america" },
    NationDef { code: "MS", name: "Montserrat", region_id: "central-america" },
    NationDef { code: "VG", name: "British Virgin Islands", region_id: "central-america" },
    NationDef { code: "VI", name: "U.S. Virgin Islands", region_id: "central-america" },
    NationDef { code: "TC", name: "Turks and Caicos Islands", region_id: "central-america" },
    NationDef { code: "AI", name: "Anguilla", region_id: "central-america" },
    // Africa (CAF)
    NationDef { code: "AO", name: "Angola", region_id: "africa" },
    NationDef { code: "BJ", name: "Benin", region_id: "africa" },
    NationDef { code: "BW", name: "Botswana", region_id: "africa" },
    NationDef { code: "BF", name: "Burkina Faso", region_id: "africa" },
    NationDef { code: "BI", name: "Burundi", region_id: "africa" },
    NationDef { code: "CV", name: "Cape Verde", region_id: "africa" },
    NationDef { code: "CF", name: "Central African Republic", region_id: "africa" },
    NationDef { code: "TD", name: "Chad", region_id: "africa" },
    NationDef { code: "KM", name: "Comoros", region_id: "africa" },
    NationDef { code: "CG", name: "Congo", region_id: "africa" },
    NationDef { code: "CD", name: "DR Congo", region_id: "africa" },
    NationDef { code: "DJ", name: "Djibouti", region_id: "africa" },
    NationDef { code: "GQ", name: "Equatorial Guinea", region_id: "africa" },
    NationDef { code: "ER", name: "Eritrea", region_id: "africa" },
    NationDef { code: "SZ", name: "Eswatini", region_id: "africa" },
    NationDef { code: "ET", name: "Ethiopia", region_id: "africa" },
    NationDef { code: "GA", name: "Gabon", region_id: "africa" },
    NationDef { code: "GM", name: "Gambia", region_id: "africa" },
    NationDef { code: "GN", name: "Guinea", region_id: "africa" },
    NationDef { code: "GW", name: "Guinea-Bissau", region_id: "africa" },
    NationDef { code: "KE", name: "Kenya", region_id: "africa" },
    NationDef { code: "LS", name: "Lesotho", region_id: "africa" },
    NationDef { code: "LR", name: "Liberia", region_id: "africa" },
    NationDef { code: "LY", name: "Libya", region_id: "africa" },
    NationDef { code: "MG", name: "Madagascar", region_id: "africa" },
    NationDef { code: "MW", name: "Malawi", region_id: "africa" },
    NationDef { code: "ML", name: "Mali", region_id: "africa" },
    NationDef { code: "MR", name: "Mauritania", region_id: "africa" },
    NationDef { code: "MU", name: "Mauritius", region_id: "africa" },
    NationDef { code: "MZ", name: "Mozambique", region_id: "africa" },
    NationDef { code: "NA", name: "Namibia", region_id: "africa" },
    NationDef { code: "NE", name: "Niger", region_id: "africa" },
    NationDef { code: "RW", name: "Rwanda", region_id: "africa" },
    NationDef { code: "ST", name: "São Tomé and Príncipe", region_id: "africa" },
    NationDef { code: "SC", name: "Seychelles", region_id: "africa" },
    NationDef { code: "SL", name: "Sierra Leone", region_id: "africa" },
    NationDef { code: "SO", name: "Somalia", region_id: "africa" },
    NationDef { code: "SS", name: "South Sudan", region_id: "africa" },
    NationDef { code: "SD", name: "Sudan", region_id: "africa" },
    NationDef { code: "TZ", name: "Tanzania", region_id: "africa" },
    NationDef { code: "TG", name: "Togo", region_id: "africa" },
    NationDef { code: "UG", name: "Uganda", region_id: "africa" },
    NationDef { code: "ZM", name: "Zambia", region_id: "africa" },
    NationDef { code: "ZW", name: "Zimbabwe", region_id: "africa" },
    // Asia (AFC)
    NationDef { code: "AF", name: "Afghanistan", region_id: "asia" },
    NationDef { code: "BH", name: "Bahrain", region_id: "asia" },
    NationDef { code: "BD", name: "Bangladesh", region_id: "asia" },
    NationDef { code: "BT", name: "Bhutan", region_id: "asia" },
    NationDef { code: "BN", name: "Brunei", region_id: "asia" },
    NationDef { code: "KH", name: "Cambodia", region_id: "asia" },
    NationDef { code: "TW", name: "Chinese Taipei", region_id: "asia" },
    NationDef { code: "GU", name: "Guam", region_id: "asia" },
    NationDef { code: "HK", name: "Hong Kong", region_id: "asia" },
    NationDef { code: "IN", name: "India", region_id: "asia" },
    NationDef { code: "ID", name: "Indonesia", region_id: "asia" },
    NationDef { code: "JO", name: "Jordan", region_id: "asia" },
    NationDef { code: "KP", name: "North Korea", region_id: "asia" },
    NationDef { code: "KW", name: "Kuwait", region_id: "asia" },
    NationDef { code: "KG", name: "Kyrgyzstan", region_id: "asia" },
    NationDef { code: "LA", name: "Laos", region_id: "asia" },
    NationDef { code: "LB", name: "Lebanon", region_id: "asia" },
    NationDef { code: "MO", name: "Macau", region_id: "asia" },
    NationDef { code: "MY", name: "Malaysia", region_id: "asia" },
    NationDef { code: "MV", name: "Maldives", region_id: "asia" },
    NationDef { code: "MN", name: "Mongolia", region_id: "asia" },
    NationDef { code: "MM", name: "Myanmar", region_id: "asia" },
    NationDef { code: "NP", name: "Nepal", region_id: "asia" },
    NationDef { code: "OM", name: "Oman", region_id: "asia" },
    NationDef { code: "PK", name: "Pakistan", region_id: "asia" },
    NationDef { code: "PS", name: "Palestine", region_id: "asia" },
    NationDef { code: "PH", name: "Philippines", region_id: "asia" },
    NationDef { code: "SG", name: "Singapore", region_id: "asia" },
    NationDef { code: "LK", name: "Sri Lanka", region_id: "asia" },
    NationDef { code: "SY", name: "Syria", region_id: "asia" },
    NationDef { code: "TJ", name: "Tajikistan", region_id: "asia" },
    NationDef { code: "TL", name: "Timor-Leste", region_id: "asia" },
    NationDef { code: "TM", name: "Turkmenistan", region_id: "asia" },
    NationDef { code: "VN", name: "Vietnam", region_id: "asia" },
    NationDef { code: "YE", name: "Yemen", region_id: "asia" },
    // Oceania (OFC)
    NationDef { code: "FJ", name: "Fiji", region_id: "oceania" },
    NationDef { code: "NC", name: "New Caledonia", region_id: "oceania" },
    NationDef { code: "PG", name: "Papua New Guinea", region_id: "oceania" },
    NationDef { code: "SB", name: "Solomon Islands", region_id: "oceania" },
    NationDef { code: "PF", name: "Tahiti", region_id: "oceania" },
    NationDef { code: "VU", name: "Vanuatu", region_id: "oceania" },
    NationDef { code: "AS", name: "American Samoa", region_id: "oceania" },
    NationDef { code: "CK", name: "Cook Islands", region_id: "oceania" },
    NationDef { code: "WS", name: "Samoa", region_id: "oceania" },
    NationDef { code: "TO", name: "Tonga", region_id: "oceania" },
];

/// Every nation the game recognises: the World Cup pool plus the wider FIFA
/// membership. The single source of truth for selectable / importable
/// nationalities — the World Cup deliberately reads only [`NATION_CATALOG`].
pub fn all_nations() -> impl Iterator<Item = &'static NationDef> {
    NATION_CATALOG.iter().chain(ADDITIONAL_NATIONS.iter())
}

pub fn nation_by_code(code: &str) -> Option<&'static NationDef> {
    all_nations().find(|nation| nation.code == code)
}

/// Look a nation up by what a person would type: its display name, or its code.
/// Case-insensitive, so `ofm-cli add country "brazil"` finds Brazil.
///
/// Used when scaffolding a country, where the input is a hand-typed name and the
/// output has to be the nation the rest of the game already knows.
///
/// Folds case with `to_lowercase`, not `eq_ignore_ascii_case`: the catalog holds
/// `Türkiye`, `Curaçao` and `São Tomé and Príncipe`, whose accented letters are
/// not ASCII, so a byte-wise comparison would miss `TÜRKIYE` and `CURAÇAO`.
pub fn nation_by_name(name: &str) -> Option<&'static NationDef> {
    let needle = name.trim().to_lowercase();
    all_nations()
        .find(|nation| nation.name.to_lowercase() == needle || nation.code.to_lowercase() == needle)
}

/// Confederation/region id for a nation code, defaulting to Europe for nations
/// outside the catalog. Single source of truth for region inference across the
/// generator, competitions, and the UI.
pub fn region_for_code(code: &str) -> &'static str {
    nation_by_code(code)
        .map(|nation| nation.region_id)
        .unwrap_or("europe")
}

/// The FIFA confederation a region belongs to. The catalog splits the Americas
/// into three size-based regions; World Cup qualifying and berth quotas reason
/// in terms of the six real confederations, so North and Central America fold
/// into CONCACAF. Unknown regions default to UEFA, matching `region_for_code`.
pub fn confederation_of_region(region: &str) -> &'static str {
    match region {
        "south-america" => "conmebol",
        "north-america" | "central-america" => "concacaf",
        "africa" => "caf",
        "asia" => "afc",
        "oceania" => "ofc",
        _ => "uefa",
    }
}

/// Whether `id` names one of the built-in confederations/regions (so a world
/// package may reference it without redefining it).
pub fn is_builtin_region(id: &str) -> bool {
    NATION_CATALOG.iter().any(|nation| nation.region_id == id)
}

/// Human-readable nation name, falling back to the code for nations outside
/// the catalog (e.g. nationalities only present in a custom world file).
pub fn nation_display_name(code: &str) -> String {
    nation_by_code(code)
        .map(|nation| nation.name.to_string())
        .unwrap_or_else(|| code.to_string())
}

/// Countries that use a split-season (Apertura + Clausura) format rather than
/// a single annual competition.
pub fn is_split_season_country(code: &str) -> bool {
    matches!(code, "AR" | "CO")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_large_enough_for_a_48_team_world_cup() {
        assert!(NATION_CATALOG.len() >= 48);
    }

    #[test]
    fn confederation_folds_the_americas_into_concacaf() {
        assert_eq!(confederation_of_region("north-america"), "concacaf");
        assert_eq!(confederation_of_region("central-america"), "concacaf");
        assert_eq!(confederation_of_region("south-america"), "conmebol");
        assert_eq!(confederation_of_region("europe"), "uefa");
        assert_eq!(confederation_of_region("africa"), "caf");
        assert_eq!(confederation_of_region("asia"), "afc");
        assert_eq!(confederation_of_region("oceania"), "ofc");
        // Unknown regions default to UEFA, matching region_for_code.
        assert_eq!(confederation_of_region("made-up"), "uefa");
    }

    #[test]
    fn nation_codes_are_unique_across_both_lists() {
        // A code appearing in both lists would double-count a nation and let a
        // World Cup entrant masquerade as a merely-selectable one.
        let mut codes: Vec<&str> = all_nations().map(|n| n.code).collect();
        codes.sort();
        let before = codes.len();
        codes.dedup();
        assert_eq!(before, codes.len(), "nation codes must be globally unique");
    }

    #[test]
    fn display_name_falls_back_to_the_code() {
        assert_eq!(nation_display_name("BR"), "Brazil");
        assert_eq!(nation_display_name("XX"), "XX");
    }

    #[test]
    fn selectable_covers_previously_missing_fifa_nations() {
        // Regression for #270: real FIFA nations selectable in the world editor
        // that used to be rejected on import because they were absent entirely.
        assert!(nation_by_code("AM").is_some(), "Armenia should be selectable");
        assert!(
            nation_by_code("GW").is_some(),
            "Guinea-Bissau should be selectable"
        );
        // Antarctica is not a football nation and must not leak in.
        assert!(nation_by_code("AQ").is_none(), "Antarctica must be excluded");
        // The bare UK code is never used — the home nations stand in for it.
        assert!(nation_by_code("GB").is_none(), "GB must not be used");
    }

    #[test]
    fn additional_nations_are_not_world_cup_entrants() {
        // The decoupling contract (see #270): widening who is selectable must
        // not widen the World Cup pool, or qualifying scale/balance shifts.
        for nation in ADDITIONAL_NATIONS {
            assert!(
                !NATION_CATALOG.iter().any(|c| c.code == nation.code),
                "{} is in both lists; ADDITIONAL_NATIONS must stay out of the WC pool",
                nation.code
            );
        }
    }

    #[test]
    fn every_nation_has_a_known_region() {
        for nation in all_nations() {
            assert!(
                is_builtin_region(nation.region_id),
                "{} has region {} which no nation defines",
                nation.code,
                nation.region_id
            );
        }
    }
}
