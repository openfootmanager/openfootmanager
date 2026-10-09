use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs `scenario`, which asserts the behaviour a known bug breaks.
///
/// While the bug reproduces the scenario fails, and that failure is the expected outcome. Once it
/// passes, the bug is fixed and the guard has to go.
pub fn expect_bug(issue: u32, scenario: impl FnOnce()) {
    match catch_unwind(AssertUnwindSafe(scenario)) {
        Err(_) => eprintln!("known bug #{issue} still reproduces"),
        Ok(()) => panic!("remove the guard for #{issue}: the scenario passes, so the bug is fixed"),
    }
}

/// `expect_bug!(#123, { ... })`: see [`expect_bug`].
#[macro_export]
macro_rules! expect_bug {
    (# $issue:literal, $scenario:block) => {
        $crate::expect_bug($issue, || $scenario)
    };
}

#[cfg(test)]
mod tests {
    use super::expect_bug;

    /// Given a scenario whose bug still reproduces (it asserts and fails)
    /// When it runs under the guard
    /// Then the guard passes.
    #[test]
    fn a_reproducing_bug_passes_the_guard() {
        expect_bug(1, || panic!("the bug"));
    }

    /// Given a scenario whose bug no longer reproduces
    /// When it runs under the guard
    /// Then the guard fails, naming the issue to remove it for.
    #[test]
    #[should_panic(expected = "remove the guard for #7")]
    fn a_fixed_bug_fails_the_guard() {
        expect_bug(7, || {});
    }

    #[test]
    fn the_macro_takes_an_issue_number() {
        expect_bug!(#9, { assert_eq!(1, 2) });
    }
}
