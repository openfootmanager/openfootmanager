pub fn normalize_football_nation_code(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "eng" | "england" | "english" => "ENG".to_string(),
        "sco" | "scotland" | "scottish" => "SCO".to_string(),
        "wal" | "wales" | "welsh" => "WAL".to_string(),
        "nir" | "northern ireland" | "northern irish" => "NIR".to_string(),
        "ie" | "ireland" | "irish" | "republic of ireland" => "IE".to_string(),
        "gb" | "british" | "uk" | "united kingdom" | "great britain" => "GB".to_string(),
        _ => {
            let upper = trimmed.to_ascii_uppercase();
            if upper.len() <= 3 {
                upper
            } else {
                trimmed.to_string()
            }
        }
    }
}

pub fn derive_birth_country_code(value: &str) -> Option<String> {
    let normalized = normalize_football_nation_code(value);
    if normalized.is_empty() || normalized == "GB" {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_home_nations_and_legacy_aliases() {
        assert_eq!(normalize_football_nation_code("English"), "ENG");
        assert_eq!(normalize_football_nation_code("Scotland"), "SCO");
        assert_eq!(normalize_football_nation_code("Welsh"), "WAL");
        assert_eq!(normalize_football_nation_code("Northern Irish"), "NIR");
        assert_eq!(normalize_football_nation_code("Irish"), "IE");
        assert_eq!(normalize_football_nation_code("British"), "GB");
    }

    #[test]
    fn preserves_legacy_british_ambiguity_for_birth_country() {
        assert_eq!(derive_birth_country_code("British"), None);
        assert_eq!(derive_birth_country_code("GB"), None);
        assert_eq!(
            derive_birth_country_code("English"),
            Some("ENG".to_string())
        );
    }
}

/// What a person is called: the known-as name when there is one, else first and last name.
/// The one rule for staff and managers, so the two cannot disagree about it.
pub fn display_name(known_as: Option<&str>, first_name: &str, last_name: &str) -> String {
    match known_as {
        Some(name) => name.to_string(),
        None => format!("{first_name} {last_name}").trim().to_string(),
    }
}

/// A known-as name as it is kept: trimmed, and none when blank.
pub fn tidy_known_as(known_as: Option<String>) -> Option<String> {
    known_as
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod display_name_tests {
    use super::*;

    /// Given a person with and without a known-as name, and a blank one (#557)
    /// When their display name is worked out
    /// Then the known-as name wins, else first and last name, and a blank known-as is none.
    #[test]
    fn a_known_as_name_wins_and_a_blank_one_is_none() {
        assert_eq!(
            display_name(Some("Pep Guardiola"), "Josep", "Guardiola i Sala"),
            "Pep Guardiola"
        );
        assert_eq!(
            display_name(None, "Josep", "Guardiola i Sala"),
            "Josep Guardiola i Sala"
        );
        assert_eq!(display_name(None, "Madonna", ""), "Madonna");
        assert_eq!(tidy_known_as(Some("  Pep  ".into())), Some("Pep".into()));
        assert_eq!(tidy_known_as(Some("   ".into())), None);
        assert_eq!(tidy_known_as(None), None);
    }
}
