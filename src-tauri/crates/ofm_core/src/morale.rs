//! How a player's morale moves, in one place.

/// Bad news stops here. A knock takes a player no lower than this, and a player already below
/// it, which only the transfer market's knocks can do, is not lifted by bad news either.
pub const MORALE_FLOOR: u8 = 10;

const MORALE_CEILING: i16 = 100;

/// The morale a player ends on after a change of `delta`.
///
/// A change never moves morale against its sign, and never by more than its size. Good news
/// lifts by `delta` and stops at 100. Bad news lowers by `-delta` and stops at
/// [`MORALE_FLOOR`], so a player who is already below the floor stays where he is.
///
/// Every morale event with a floor used to clamp to `(10, 100)` or `(5, 100)` on its own. A
/// clamp lifts whatever is below its floor, so a player at 3 who lost a match, or was told he
/// could leave, was raised to 10, and the same player gaining 2 jumped to 10 as well.
pub fn apply_morale_change(morale: u8, delta: i16) -> u8 {
    let current = i16::from(morale);
    let moved = current.saturating_add(delta);
    let bounded = if delta < 0 {
        moved.max(current.min(i16::from(MORALE_FLOOR)))
    } else {
        moved
    };
    bounded.clamp(0, MORALE_CEILING) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a player at 50
    /// When bad news worth 5 lands
    /// Then he drops by 5, to 45
    #[test]
    fn bad_news_lowers_morale_by_its_size() {
        assert_eq!(apply_morale_change(50, -5), 45);
    }

    /// Given a player at 12
    /// When bad news worth 5 lands
    /// Then he stops at the floor of 10
    #[test]
    fn bad_news_stops_at_the_floor() {
        assert_eq!(apply_morale_change(12, -5), 10);
    }

    /// Given a player at 3, already below the floor
    /// When bad news worth 1 or worth 20 lands
    /// Then he stays at 3: bad news never lifts anyone
    #[test]
    fn bad_news_never_lifts_a_player_already_below_the_floor() {
        assert_eq!(
            (apply_morale_change(3, -1), apply_morale_change(3, -20)),
            (3, 3)
        );
    }

    /// Given a player at 3, below the floor
    /// When good news worth 2 lands
    /// Then he rises by 2, to 5, not to the floor
    #[test]
    fn good_news_lifts_morale_by_its_size_even_below_the_floor() {
        assert_eq!(apply_morale_change(3, 2), 5);
    }

    /// Given a player at 98
    /// When good news worth 5 lands
    /// Then he stops at 100
    #[test]
    fn good_news_stops_at_one_hundred() {
        assert_eq!(apply_morale_change(98, 5), 100);
    }

    /// Given a player at 3
    /// When a change of nothing is applied
    /// Then he stays at 3
    #[test]
    fn no_news_leaves_morale_where_it_is() {
        assert_eq!(apply_morale_change(3, 0), 3);
    }

    /// The hand-written morale changes outside this file that are meant to be there: knocks with a
    /// floor of 0, which the transfer market and a free agent's signing deal in. Each is its
    /// file and the line's text, so moving one does not break the scan, but changing it does.
    const FLOOR_ZERO_CLAMPS: [(&str, &str); 3] = [
        (
            "contracts/free_agent.rs",
            "player.morale = (i16::from(player.morale) + 6).clamp(0, 100) as u8;",
        ),
        (
            "transfers/execution.rs",
            "player.morale = (i16::from(player.morale) - 4).clamp(0, 100) as u8;",
        ),
        (
            "transfers/market.rs",
            "player.morale = (i16::from(player.morale) - morale_drop).clamp(0, 100) as u8;",
        ),
    ];

    /// Whether a line changes a player's morale by hand: a compound assignment, or an assignment
    /// that reads the same player's morale on its right, other than through the one rule.
    /// Setting a value outright (a fixture, a new player) is not a change and is not listed.
    fn changes_morale(line: &str) -> bool {
        if line.contains("apply_morale_change(") {
            return false;
        }
        line.contains(".morale += ")
            || line.contains(".morale -= ")
            || line
                .split_once(".morale = ")
                .is_some_and(|(target, value)| {
                    let receiver = target
                        .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or(target);
                    value.contains(&format!("{receiver}.morale"))
                })
    }

    fn morale_changes_under(
        dir: &std::path::Path,
        root: &std::path::Path,
        found: &mut Vec<(String, String)>,
    ) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .expect("the source directory can be read")
            .map(|entry| entry.expect("a directory entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                morale_changes_under(&path, root, found);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("under the source root")
                .to_string_lossy()
                .replace('\\', "/");
            if path.extension().is_none_or(|ext| ext != "rs") || relative == "morale.rs" {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a source file can be read");
            for line in source.lines().map(str::trim) {
                if changes_morale(line) {
                    found.push((relative.clone(), line.to_string()));
                }
            }
        }
    }

    /// Given every source file in ofm_core outside this one
    /// When each line that changes a player's morale by hand, clamped or not, is listed
    /// Then the list is exactly the three floor-0 knocks: every other morale change goes through
    ///      apply_morale_change, so its floor is the one rule
    #[test]
    fn every_morale_change_with_a_floor_goes_through_apply_morale_change() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        morale_changes_under(&root, &root, &mut found);

        let expected: Vec<(String, String)> = FLOOR_ZERO_CLAMPS
            .iter()
            .map(|(file, line)| (file.to_string(), line.to_string()))
            .collect();
        assert_eq!(found, expected);
    }
}
