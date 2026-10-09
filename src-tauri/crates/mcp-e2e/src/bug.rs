use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs `scenario`, which asserts the behaviour a known bug breaks.
///
/// While the bug reproduces the scenario fails with a message containing `expected_failure`, and
/// that is the expected outcome. A different failure is a different regression and is not hidden.
/// Once the scenario passes, the bug is fixed and the guard has to go.
pub fn expect_bug(issue: u32, expected_failure: &str, scenario: impl FnOnce()) {
    match catch_unwind(AssertUnwindSafe(scenario)) {
        Err(payload) => {
            let message = panic_message(payload.as_ref());
            if !message.contains(expected_failure) {
                panic!(
                    "the scenario for #{issue} failed, but not the way the bug does \
                     (expected `{expected_failure}`): {message}"
                );
            }
            eprintln!("known bug #{issue} still reproduces");
        }
        Ok(()) => panic!("remove the guard for #{issue}: the scenario passes, so the bug is fixed"),
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// `expect_bug!(#123, "what the bug says", { ... })`: see [`expect_bug`].
#[macro_export]
macro_rules! expect_bug {
    (# $issue:literal, $expected_failure:literal, $scenario:block) => {
        $crate::expect_bug($issue, $expected_failure, || $scenario)
    };
}

#[cfg(test)]
mod tests {
    use super::expect_bug;

    /// Given a scenario whose bug still reproduces (it fails the expected way)
    /// When it runs under the guard
    /// Then the guard passes.
    #[test]
    fn a_reproducing_bug_passes_the_guard() {
        expect_bug(1, "the bug", || panic!("this is the bug"));
    }

    /// Given a scenario whose bug no longer reproduces
    /// When it runs under the guard
    /// Then the guard fails, naming the issue to remove it for.
    #[test]
    #[should_panic(expected = "remove the guard for #7")]
    fn a_fixed_bug_fails_the_guard() {
        expect_bug(7, "the bug", || {});
    }

    /// Given a scenario that fails, but not the way the bug does
    /// When it runs under the guard
    /// Then the guard fails instead of hiding a different regression.
    #[test]
    #[should_panic(expected = "not the way the bug does")]
    fn a_different_failure_is_not_hidden() {
        expect_bug(8, "name_key", || panic!("the server crashed"));
    }

    #[test]
    fn the_macro_takes_an_issue_and_the_failure_it_expects() {
        expect_bug!(#9, "left", { assert_eq!(1, 2) });
    }
}
