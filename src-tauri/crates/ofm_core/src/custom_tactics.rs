//! The player's saved tactics: what is a valid one, and how one is added, replaced or removed.
//!
//! They belong to the career (`Game::custom_tactics`), not to a club, so a change of club leaves
//! them alone. The frontend used to keep them in the browser's storage, which lost them with the
//! webview data and orphaned them on a club change (#390).

use domain::custom_tactic::CustomTactic;
use domain::team::PlayStyle;

use crate::game::Game;
use crate::player_rating::parse_formation;

pub const NAME_REQUIRED_ERROR: &str = "be.error.customTactic.nameRequired";
pub const INVALID_FORMATION_ERROR: &str = "be.error.customTactic.invalidFormation";
pub const INVALID_PLAY_STYLE_ERROR: &str = "be.error.customTactic.invalidPlayStyle";
pub const NOT_FOUND_ERROR: &str = "be.error.customTactic.notFound";

/// The longest name kept; longer ones are a mistake rather than a name.
const MAX_NAME_CHARS: usize = 60;

/// The tactic as it will be kept, or the translation key of what is wrong with it.
fn validated(tactic: &CustomTactic) -> Result<CustomTactic, String> {
    let name = tactic.name.trim();
    if tactic.id.trim().is_empty() || name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(NAME_REQUIRED_ERROR.to_string());
    }
    if parse_formation(&tactic.formation).is_none() {
        return Err(INVALID_FORMATION_ERROR.to_string());
    }
    // `PlayStyle` is the one list of styles: what it can deserialize from is what is allowed.
    if serde_json::from_value::<PlayStyle>(serde_json::Value::String(tactic.play_style.clone()))
        .is_err()
    {
        return Err(INVALID_PLAY_STYLE_ERROR.to_string());
    }
    Ok(CustomTactic {
        id: tactic.id.trim().to_string(),
        name: name.to_string(),
        description: tactic.description.trim().to_string(),
        ..tactic.clone()
    })
}

/// The list with one tactic per trimmed id: the last copy, at the place it last stood, under its
/// trimmed id. The one rule for ids kept before they were trimmed, which the save and the load of
/// the table share so a padded twin can neither be written nor read back as a second tactic.
pub fn one_tactic_per_id(tactics: Vec<CustomTactic>) -> Vec<CustomTactic> {
    let mut kept: Vec<CustomTactic> = Vec::with_capacity(tactics.len());
    for mut tactic in tactics.into_iter().rev() {
        tactic.id = tactic.id.trim().to_string();
        if !kept.iter().any(|later| later.id == tactic.id) {
            kept.push(tactic);
        }
    }
    kept.reverse();
    kept
}

/// Adds the tactic, or replaces the one with the same id where it stands in the list.
pub fn save_custom_tactic(game: &mut Game, tactic: &CustomTactic) -> Result<(), String> {
    let tactic = validated(tactic)?;
    let id = tactic.id.clone();
    let Some(first) = game.custom_tactics.iter().position(|t| t.id.trim() == id) else {
        game.custom_tactics.push(tactic);
        return Ok(());
    };
    game.custom_tactics[first] = tactic;
    // An old save may hold the same id twice, once padded; keep one.
    let mut at = 0;
    game.custom_tactics.retain(|t| {
        at += 1;
        at - 1 <= first || t.id.trim() != id
    });
    Ok(())
}

/// Removes the tactic with `id`.
pub fn delete_custom_tactic(game: &mut Game, id: &str) -> Result<(), String> {
    let id = id.trim();
    let before = game.custom_tactics.len();
    // Ids kept before they were trimmed may still be padded in an old save.
    game.custom_tactics.retain(|t| t.id.trim() != id);
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

    /// Given strings that name no formation the game can lay out
    /// When they are saved as a tactic
    /// Then each is refused as an invalid formation, and a real one is kept.
    #[test]
    fn a_formation_the_game_cannot_lay_out_is_refused() {
        let mut game = game();
        for formation in [
            "",
            "abc",
            "5-5",
            "4-4-2-extra",
            "10-0-0",
            "4-4-1",
            "4-4-3",
            "4--6",
        ] {
            let mut bad = tactic("a", "Named");
            bad.formation = formation.into();
            assert_eq!(
                save_custom_tactic(&mut game, &bad),
                Err(INVALID_FORMATION_ERROR.to_string()),
                "{formation:?}"
            );
        }
        for formation in ["4-4-2", "3-5-2", "4-1-4-1", "5-3-2"] {
            let mut good = tactic(formation, "Named");
            good.formation = formation.into();
            save_custom_tactic(&mut game, &good).unwrap();
        }
        assert_eq!(game.custom_tactics.len(), 4);
    }

    /// Given every play style the game has
    /// When a tactic is saved with its name
    /// Then it is accepted: the list of styles is `PlayStyle`'s own.
    #[test]
    fn every_play_style_is_accepted() {
        let mut game = game();
        for style in [
            PlayStyle::Balanced,
            PlayStyle::Attacking,
            PlayStyle::Defensive,
            PlayStyle::Possession,
            PlayStyle::Counter,
            PlayStyle::HighPress,
        ] {
            let name = serde_json::to_value(&style).unwrap();
            let mut case = tactic(name.as_str().unwrap(), "Named");
            case.play_style = name.as_str().unwrap().to_string();
            save_custom_tactic(&mut game, &case).unwrap();
        }
        assert_eq!(game.custom_tactics.len(), 6);
    }

    /// Given a saved tactic "a"
    /// When one with the id "a " (a trailing space) is saved, and then "a" is deleted
    /// Then the second replaces the first (one tactic, stored under the trimmed id) and the delete finds it.
    #[test]
    fn an_id_is_kept_trimmed_so_padding_cannot_make_a_second_tactic() {
        let mut game = game();
        save_custom_tactic(&mut game, &tactic("a", "First")).unwrap();

        save_custom_tactic(&mut game, &tactic("a ", "Second")).unwrap();

        assert_eq!(game.custom_tactics.len(), 1);
        assert_eq!(game.custom_tactics[0].id, "a");
        assert_eq!(game.custom_tactics[0].name, "Second");
        delete_custom_tactic(&mut game, "a ").unwrap();
        assert!(game.custom_tactics.is_empty());
    }

    /// Given a tactic already stored with a padded id "a " (kept before ids were trimmed)
    /// When "a" is saved, and then "a" is deleted
    /// Then the save replaces it (one tactic, id "a") and the delete finds it.
    #[test]
    fn a_legacy_padded_id_matches_its_trimmed_form() {
        let mut game = game();
        game.custom_tactics.push(tactic("a ", "Legacy"));

        save_custom_tactic(&mut game, &tactic("a", "Current")).unwrap();
        assert_eq!(game.custom_tactics.len(), 1);
        assert_eq!(game.custom_tactics[0].id, "a");
        assert_eq!(game.custom_tactics[0].name, "Current");

        game.custom_tactics[0].id = "a ".to_string();
        delete_custom_tactic(&mut game, "a").unwrap();
        assert!(game.custom_tactics.is_empty());
    }

    /// Given a legacy save holding both "a" and "a " as separate tactics, and another tactic
    /// When "a" is saved
    /// Then the first replaces in place, the padded twin is dropped, and the other tactic stays.
    #[test]
    fn saving_collapses_legacy_twins_that_trim_to_the_same_id() {
        let mut game = game();
        game.custom_tactics = vec![
            tactic("a", "First"),
            tactic("b", "Other"),
            tactic("a ", "Twin"),
        ];

        save_custom_tactic(&mut game, &tactic("a", "Current")).unwrap();

        let names: Vec<&str> = game
            .custom_tactics
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert_eq!(names, ["Current", "Other"]);
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
