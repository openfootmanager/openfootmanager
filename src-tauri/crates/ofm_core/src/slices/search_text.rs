//! The one rule the list searches share for matching typed text. Case is ignored for every
//! letter (`Á`/`á`, `Ø`/`ø`, Cyrillic), and a few letters that a keyboard without them cannot type
//! count as their plain form: the Turkish `İ` and `ı` as `i`, the Greek final `ς` as `σ`, and `ß`
//! as `ss`. Accents are NOT ignored: `alvarez` does not find `Álvarez`.

/// The letters of `text` as a search reads them: lowercased, with the combining dot above (what
/// `İ` leaves behind once lowercased) dropped, and the folds above applied. The same iterator reads
/// the needle and every candidate, so the two sides always agree.
fn folded(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter(|letter| *letter != '\u{307}')
        .flat_map(|letter| {
            let (first, second) = match letter {
                'ς' => ('σ', None),
                'ı' => ('i', None),
                'ß' => ('s', Some('s')),
                other => (other, None),
            };
            std::iter::once(first).chain(second)
        })
}

/// The text a search is for, folded once per request.
pub(crate) fn fold_needle(typed: &str) -> String {
    folded(typed.trim()).collect()
}

/// Whether `haystack` holds `needle` (already folded by [`fold_needle`]). It reads the haystack
/// through the folding iterator rather than building a folded copy: a search runs this for every
/// candidate on every keystroke.
pub(crate) fn contains_folded(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.char_indices().any(|(start, _)| {
        let mut letters = folded(&haystack[start..]);
        needle.chars().all(|wanted| letters.next() == Some(wanted))
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

    /// Given Turkish names with a dotted capital İ or a dotless ı
    /// When they are searched for with the plain Latin letters a keyboard gives
    /// Then they are found: `ilkay` finds İlkay and `kilic` finds Kılıç.
    #[test]
    fn turkish_dotted_and_dotless_i_match_the_plain_i() {
        assert!(contains_folded("İlkay Gündoğan", &fold_needle("ilkay")));
        assert!(contains_folded("İlkay Gündoğan", &fold_needle("İLKAY")));
        assert!(contains_folded("Kılıç", &fold_needle("KILIÇ")));
    }

    /// Given a Greek name in capitals and in lowercase
    /// When either is searched for with the other
    /// Then they match: the final sigma ς and σ are one letter.
    #[test]
    fn greek_final_sigma_matches_the_ordinary_sigma() {
        assert!(contains_folded("Γιάννης", &fold_needle("ΓΙΆΝΝΗΣ")));
        assert!(contains_folded("ΓΙΆΝΝΗΣ", &fold_needle("γιάννης")));
    }

    /// Given a German name spelled with ß
    /// When it is searched for with ss, or the reverse
    /// Then it is found.
    #[test]
    fn german_sharp_s_matches_ss() {
        assert!(contains_folded("Strauß", &fold_needle("strauss")));
        assert!(contains_folded("Strauss", &fold_needle("STRAUß")));
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
