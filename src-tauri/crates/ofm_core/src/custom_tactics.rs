//! The player's saved tactics: what is a valid one, and how one is added, replaced or removed.
//!
//! They belong to the career (`Game::custom_tactics`), not to a club, so a change of club leaves
//! them alone. The frontend used to keep them in the browser's storage, which lost them with the
//! webview data and orphaned them on a club change (#390).

use domain::custom_tactic::CustomTactic;
use domain::team::PlayStyle;

use crate::game::Game;
use crate::player_rating::formation_slots;

pub const NAME_REQUIRED_ERROR: &str = "be.error.customTactic.nameRequired";
pub const INVALID_FORMATION_ERROR: &str = "be.error.customTactic.invalidFormation";
pub const INVALID_PLAY_STYLE_ERROR: &str = "be.error.customTactic.invalidPlayStyle";
pub const NOT_FOUND_ERROR: &str = "be.error.customTactic.notFound";

/// The longest name kept; longer ones are a mistake rather than a name.
const MAX_NAME_CHARS: usize = 60;

const PLAY_STYLES: [(&str, PlayStyle); 6] = [
    ("Balanced", PlayStyle::Balanced),
    ("Attacking", PlayStyle::Attacking),
    ("Defensive", PlayStyle::Defensive),
    ("Possession", PlayStyle::Possession),
    ("Counter", PlayStyle::Counter),
    ("HighPress", PlayStyle::HighPress),
];

/// The tactic as it will be kept, or the translation key of what is wrong with it.
fn validated(tactic: &CustomTactic) -> Result<CustomTactic, String> {
    let name = tactic.name.trim();
    if tactic.id.trim().is_empty() || name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(NAME_REQUIRED_ERROR.to_string());
    }
    if formation_slots(&tactic.formation).len() != 11 {
        return Err(INVALID_FORMATION_ERROR.to_string());
    }
    if !PLAY_STYLES
        .iter()
        .any(|(style, _)| *style == tactic.play_style)
    {
        return Err(INVALID_PLAY_STYLE_ERROR.to_string());
    }
    Ok(CustomTactic {
        name: name.to_string(),
        description: tactic.description.trim().to_string(),
        ..tactic.clone()
    })
}

/// Adds the tactic, or replaces the one with the same id where it stands in the list.
pub fn save_custom_tactic(game: &mut Game, tactic: &CustomTactic) -> Result<(), String> {
    let tactic = validated(tactic)?;
    match game.custom_tactics.iter_mut().find(|t| t.id == tactic.id) {
        Some(existing) => *existing = tactic,
        None => game.custom_tactics.push(tactic),
    }
    Ok(())
}

/// Removes the tactic with `id`.
pub fn delete_custom_tactic(game: &mut Game, id: &str) -> Result<(), String> {
    let before = game.custom_tactics.len();
    game.custom_tactics.retain(|t| t.id != id);
    if game.custom_tactics.len() == before {
        return Err(NOT_FOUND_ERROR.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;

    fn game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 7, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn tactic(id: &str, name: &str) -> CustomTactic {
        CustomTactic {
            id: id.into(),
            name: name.into(),
            description: "  Press high  ".into(),
            formation: "4-4-2".into(),
            play_style: "HighPress".into(),
            source_preset_name: Some("Gegenpress".into()),
        }
    }

    /// Given an empty library
    /// When tactics are saved
    /// Then they are kept in the order made, tidied (names and descriptions trimmed).
    #[test]
    fn saved_tactics_are_kept_in_order_and_tidied() {
        let mut game = game();
        save_custom_tactic(&mut game, &tactic("a", "  Gegen  ")).unwrap();
        save_custom_tactic(&mut game, &tactic("b", "Park the bus")).unwrap();

        let names: Vec<&str> = game
            .custom_tactics
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert_eq!(names, ["Gegen", "Park the bus"]);
        assert_eq!(game.custom_tactics[0].description, "Press high");
        assert_eq!(
            game.custom_tactics[0].source_preset_name.as_deref(),
            Some("Gegenpress")
        );
    }

    /// Given a saved tactic
    /// When one with the same id is saved
    /// Then it replaces it where it stands, not at the end.
    #[test]
    fn saving_an_existing_id_replaces_it_in_place() {
        let mut game = game();
        save_custom_tactic(&mut game, &tactic("a", "First")).unwrap();
        save_custom_tactic(&mut game, &tactic("b", "Second")).unwrap();

        save_custom_tactic(&mut game, &tactic("a", "First, renamed")).unwrap();

        assert_eq!(game.custom_tactics.len(), 2);
        assert_eq!(game.custom_tactics[0].name, "First, renamed");
        assert_eq!(game.custom_tactics[1].name, "Second");
    }

    /// Given tactics that are not valid
    /// When they are saved
    /// Then each is refused with its own key and nothing is stored.
    #[test]
    fn an_invalid_tactic_is_refused_with_its_own_key() {
        let mut game = game();
        let mut blank = tactic("a", "   ");
        let mut no_id = tactic("", "Named");
        let mut long = tactic("a", &"x".repeat(MAX_NAME_CHARS + 1));
        let mut bad_formation = tactic("a", "Named");
        bad_formation.formation = "9-9-9".into();
        let mut bad_style = tactic("a", "Named");
        bad_style.play_style = "Anarchy".into();
        blank.description.clear();
        no_id.description.clear();
        long.description.clear();

        for (case, key) in [
            (blank, NAME_REQUIRED_ERROR),
            (no_id, NAME_REQUIRED_ERROR),
            (long, NAME_REQUIRED_ERROR),
            (bad_formation, INVALID_FORMATION_ERROR),
            (bad_style, INVALID_PLAY_STYLE_ERROR),
        ] {
            assert_eq!(save_custom_tactic(&mut game, &case), Err(key.to_string()));
        }
        assert!(game.custom_tactics.is_empty());
    }

    /// Given names of exactly the longest allowed length, and one character more
    /// When they are saved
    /// Then the first is kept and the second refused: the limit is inclusive.
    #[test]
    fn the_longest_allowed_name_is_kept_and_one_more_is_refused() {
        let mut game = game();
        let longest = "x".repeat(MAX_NAME_CHARS);
        save_custom_tactic(&mut game, &tactic("a", &longest)).unwrap();
        assert_eq!(game.custom_tactics[0].name, longest);

        let too_long = "x".repeat(MAX_NAME_CHARS + 1);
        assert_eq!(
            save_custom_tactic(&mut game, &tactic("b", &too_long)),
            Err(NAME_REQUIRED_ERROR.to_string())
        );
    }

    /// Given saved tactics
    /// When one is deleted, and then a missing one
    /// Then the first goes and the second is reported as not found.
    #[test]
    fn deleting_removes_one_and_reports_a_missing_one() {
        let mut game = game();
        save_custom_tactic(&mut game, &tactic("a", "First")).unwrap();
        save_custom_tactic(&mut game, &tactic("b", "Second")).unwrap();

        delete_custom_tactic(&mut game, "a").unwrap();

        assert_eq!(game.custom_tactics.len(), 1);
        assert_eq!(game.custom_tactics[0].id, "b");
        assert_eq!(
            delete_custom_tactic(&mut game, "missing"),
            Err(NOT_FOUND_ERROR.to_string())
        );
    }
}
