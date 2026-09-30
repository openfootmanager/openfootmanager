//! Test support for the enums this crate stores as text.
//!
//! Most repositories write an enum as a string (usually `format!("{:?}", value)`) and read it back
//! with a hand-written `match` that ends in a silent `_ =>` fallback. That fallback is deliberate —
//! an unknown string in an old save must still load — but it also means a missing arm is not a
//! compile error: the value saves correctly and loads back as the fallback. That is exactly how
//! `league_repo` came to load every cup and continental fixture as a league fixture.
//!
//! So each parser's test pins the *literal* strings, in both directions. Deriving them from
//! `Debug` would only prove the parser agrees with today's variant names; renaming a variant moves
//! the writer and the parser together and still breaks every existing save.

use std::fmt::Debug;

/// Assert that each `(stored, value)` pair reads and writes as that exact string.
///
/// Every call site builds its cases with `stored_text_cases!`, so the exhaustive match and
/// the values checked here come from the same list.
pub(crate) fn assert_stored_as<T: PartialEq + Debug>(
    cases: &[(&str, T)],
    write: impl Fn(&T) -> String,
    parse: impl Fn(&str) -> T,
) {
    for (stored, value) in cases {
        assert_eq!(&parse(stored), value, "{stored:?} does not load back");
        assert_eq!(
            &write(value),
            stored,
            "{value:?} is no longer saved as {stored:?}"
        );
    }
}

/// Keep the compile-time exhaustiveness check tied to the stored names tested at runtime.
macro_rules! stored_text_cases {
    ($sample:expr; [$(($stored:literal, $variant:path)),+ $(,)?]) => {{
        match $sample {
            $($variant)|+ => {}
        }
        [$(($stored, $variant)),+]
    }};
}

pub(crate) use stored_text_cases;
