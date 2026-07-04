//! Catalog of football nations the game can field internationally, beyond
//! whatever nationalities a world's clubs happen to contain. Used to fill a
//! World Cup field by synthesising national squads for missing nations.

pub struct NationDef {
    pub code: &'static str,
    pub name: &'static str,
    pub region_id: &'static str,
}

/// Real football nations across every confederation, strongest footballing
/// traditions first within each region (order is used as a soft seeding hint
/// when squads are otherwise equal).
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

pub fn nation_by_code(code: &str) -> Option<&'static NationDef> {
    NATION_CATALOG.iter().find(|nation| nation.code == code)
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
    fn catalog_codes_are_unique() {
        let mut codes: Vec<&str> = NATION_CATALOG.iter().map(|n| n.code).collect();
        codes.sort();
        let before = codes.len();
        codes.dedup();
        assert_eq!(before, codes.len());
    }

    #[test]
    fn display_name_falls_back_to_the_code() {
        assert_eq!(nation_display_name("BR"), "Brazil");
        assert_eq!(nation_display_name("XX"), "XX");
    }
}
