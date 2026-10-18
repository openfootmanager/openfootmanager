//! What this binary was built from, answerable without starting the app.
//!
//! `--build-revision` prints the version and the source revision and exits before the crash hook,
//! the graphics configuration, or Tauri and the toolkit are touched, so it works with no display.
//! The MCP server reports the same string from `initialize`.

/// The flag that asks for the version line.
const FLAG: &str = "--build-revision";

/// The package version, with the sources the binary was built from as semver build metadata
/// (`0.3.0+<revision>`), so a client can tell a stale binary from a current one. Without a
/// revision (git could not say when it was built) it is the bare version.
pub fn build_version() -> String {
    version_with(env!("CARGO_PKG_VERSION"), env!("OFM_BUILD_REVISION"))
}

fn version_with(package_version: &str, revision: &str) -> String {
    match revision {
        "" => package_version.to_string(),
        revision => format!("{package_version}+{revision}"),
    }
}

/// Whether the process was asked for its build version instead of being started.
pub fn requested<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().any(|arg| arg.as_ref() == FLAG)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a package version and a source revision
    /// When the version line is built
    /// Then the revision is semver build metadata, and a missing one leaves the bare version.
    #[test]
    fn the_version_line_carries_the_revision_as_build_metadata() {
        assert_eq!(version_with("0.3.0", "abc123def456"), "0.3.0+abc123def456");
        assert_eq!(version_with("0.3.0", ""), "0.3.0");
    }

    /// Given the arguments of a launch
    /// When they are searched for the flag
    /// Then only the exact flag asks for the version; a normal launch and similar flags do not.
    #[test]
    fn only_the_exact_flag_asks_for_the_version_line() {
        assert!(requested(["--build-revision"]));
        assert!(requested(["--mcp-port", "3001", "--build-revision"]));
        assert!(!requested(["--mcp-port", "3001"]));
        assert!(!requested(["--build-revision-please"]));
        assert!(!requested(Vec::<String>::new()));
    }

    /// Given the binary under test
    /// When it names its own version
    /// Then the line starts with the package version, which the harness reads back.
    #[test]
    fn the_binary_names_its_own_version() {
        assert!(build_version().starts_with(env!("CARGO_PKG_VERSION")));
    }
}
