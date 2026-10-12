//! A known bug the gate carries on purpose.

use std::panic::{AssertUnwindSafe, catch_unwind};

/// Runs `scenario`, which asserts the behaviour a known bug breaks.
///
/// While the bug reproduces the scenario fails with a message containing `expected_failure`, and
/// that is the expected outcome. Any other failure is a different regression and is not hidden.
/// Once the scenario passes the bug is fixed and the guard has to go.
///
/// Same contract as `mcp_e2e::expect_bug`, kept here because that crate pulls in the app's
/// database and an HTTP client, which this gate has no use for.
pub fn expect_bug(issue: u32, expected_failure: &str, scenario: impl FnOnce()) {
    match catch_unwind(AssertUnwindSafe(scenario)) {
        Err(payload) => {
            let message = panic_message(payload.as_ref());
            assert!(
                message.contains(expected_failure),
                "the scenario for #{issue} failed, but not the way the bug does \
                 (expected `{expected_failure}`): {message}"
            );
            eprintln!("known bug #{issue} still reproduces");
        }
        Ok(()) => panic!("remove the guard for #{issue}: the scenario passes, so the bug is fixed"),
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_string())
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::expect_bug;

    /// Given a scenario whose bug still reproduces, When it runs under the guard, Then it passes.
    #[test]
    fn a_reproducing_bug_passes_the_guard() {
        expect_bug(1, "the bug", || panic!("this is the bug"));
    }

    /// Given a scenario whose bug is fixed, When it runs under the guard,
    /// Then the guard fails and names the issue to remove it for.
    #[test]
    #[should_panic(expected = "remove the guard for #7")]
    fn a_fixed_bug_fails_the_guard() {
        expect_bug(7, "the bug", || {});
    }

    /// Given a scenario that fails some other way, When it runs under the guard,
    /// Then the guard fails instead of hiding a different regression.
    #[test]
    #[should_panic(expected = "not the way the bug does")]
    fn a_different_failure_is_not_hidden() {
        expect_bug(8, "the bug", || panic!("the server crashed"));
    }
}
