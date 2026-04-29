/// Support for custom nations
fn code_from_label(value: &str) -> String {
    let words: Vec<&str> = value
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|segment| !segment.is_empty())
        .collect();
    if words.len() > 1 {
        let initials: String = words
            .iter()
            .filter_map(|word| word.chars().next())
            .map(|ch| ch.to_ascii_uppercase())
            .take(3)
            .collect();
        if !initials.is_empty() {
            return initials;
        }
    }

    let letters: Vec<char> = value
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_uppercase())
        .collect();
    if letters.is_empty() {
        return String::new();
    }
    letters.into_iter().take(3).collect()
}

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
                code_from_label(trimmed)
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

    #[test]
    fn derives_codes_from_created_nation_labels() {
        assert_eq!(normalize_football_nation_code("France"), "FRA");
        assert_eq!(normalize_football_nation_code("United States"), "US");
        assert_eq!(normalize_football_nation_code("Atlantis"), "ATL");
    }
}
