//! The one rule the list searches share for matching typed text: case is ignored for every
//! letter (`Á`/`á`, `Ø`/`ø`, Cyrillic, Turkish), not just ASCII.

/// The text a search is for, folded to lowercase once per request. Folding goes letter by letter
/// (`char::to_lowercase`), the same way [`contains_folded`] reads a candidate, so both sides agree
/// even for letters whose whole-string lowercasing depends on position (a final `Σ`).
pub(crate) fn fold_needle(typed: &str) -> String {
    typed.trim().chars().flat_map(char::to_lowercase).collect()
}

/// Whether `haystack` holds `needle` (already folded by [`fold_needle`]) ignoring case. It reads
/// the haystack through lowercase iterators rather than building a lowercased copy: a search runs
/// this for every candidate on every keystroke.
pub(crate) fn contains_folded(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.char_indices().any(|(start, _)| {
        let mut folded = haystack[start..].chars().flat_map(char::to_lowercase);
        needle.chars().all(|wanted| folded.next() == Some(wanted))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given text in either case and letters beyond ASCII
    /// When it is searched for folded
    /// Then it is found whichever case either side was typed in.
    #[test]
    fn folding_matches_across_case_for_non_ascii_letters() {
        assert!(contains_folded("Julián Álvarez", &fold_needle("ÁLVAREZ")));
        assert!(contains_folded("Martin Ødegaard", &fold_needle("ødeg")));
        assert!(contains_folded("Иван Петров", &fold_needle("ПЕТР")));
        assert!(contains_folded("İstanbul", &fold_needle("i\u{307}stan")));
        assert!(!contains_folded("John Smith", &fold_needle("jones")));
    }

    /// Given an empty needle
    /// When it is searched for
    /// Then it is found everywhere, as `contains("")` says.
    #[test]
    fn an_empty_needle_is_in_everything() {
        assert!(contains_folded("anything", ""));
        assert!(contains_folded("", ""));
    }
}
