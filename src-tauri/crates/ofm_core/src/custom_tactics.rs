//! The player's saved tactics: what is a valid one, and how one is added, replaced or removed.
//!
//! They belong to the career (`Game::custom_tactics`), not to a club, so a change of club leaves
//! them alone. The frontend used to keep them in the browser's storage, which lost them with the
//! webview data and orphaned them on a club change (#390).

use std::collections::HashSet;

use domain::custom_tactic::{CustomTactic, SavedSlot, TacticContents};
use domain::team::{MatchRoles, PlayStyle, Team};

use crate::game::Game;
use crate::player_rating::{formation_slots, parse_formation};

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
pub fn one_tactic_per_id(tactics: &[CustomTactic]) -> Vec<CustomTactic> {
    let mut seen = std::collections::HashSet::new();
    let mut kept: Vec<CustomTactic> = tactics
        .iter()
        .rev()
        .filter(|tactic| seen.insert(tactic.id.trim()))
        .map(|tactic| CustomTactic {
            id: tactic.id.trim().to_string(),
            ..tactic.clone()
        })
        .collect();
    kept.reverse();
    kept
}

/// Adds the tactic, or replaces the one with the same id where it stands in the list.
pub fn save_custom_tactic(game: &mut Game, tactic: &CustomTactic) -> Result<(), String> {
    let tactic = validated(tactic)?;
    // The first tactic with this id is replaced where it stands; an old save may hold the same id
    // again, once padded, and those twins go.
    let mut replaced = false;
    let mut kept = Vec::with_capacity(game.custom_tactics.len() + 1);
    for existing in std::mem::take(&mut game.custom_tactics) {
        if existing.id.trim() != tactic.id {
            kept.push(existing);
        } else if !replaced {
            replaced = true;
            kept.push(tactic.clone());
        }
    }
    if !replaced {
        kept.push(tactic);
    }
    game.custom_tactics = kept;
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

/// The manager's tactic as it stands on the team, under `draft`'s id, name, description and
/// source preset: its formation, play style, every instruction, the eleven slot by slot with each
/// one's role, and the set-piece takers (#1189). What the draft says about the rest is ignored:
/// the team is the one source of what the tactic is.
fn current_tactic(game: &Game, draft: &CustomTactic) -> Result<CustomTactic, String> {
    let team = managed_team(game)?;
    let play_style = serde_json::to_value(&team.play_style)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .ok_or_else(|| INVALID_PLAY_STYLE_ERROR.to_string())?;
    let slots = team
        .starting_xi_ids
        .iter()
        .map(|player_id| SavedSlot {
            player_id: player_id.clone(),
            role: team.player_roles.get(player_id).cloned(),
        })
        .collect();
    Ok(CustomTactic {
        formation: team.formation.clone(),
        play_style,
        contents: Some(TacticContents {
            instructions: team.tactics_phase.clone(),
            slots,
            match_roles: team.match_roles.clone(),
        }),
        ..draft.clone()
    })
}

/// Saves the manager's tactic as it stands (see [`current_tactic`]) under `draft`'s id and name,
/// adding it or replacing the one with that id.
pub fn save_current_tactic(game: &mut Game, draft: &CustomTactic) -> Result<(), String> {
    let tactic = current_tactic(game, draft)?;
    save_custom_tactic(game, &tactic)
}

/// Puts the saved tactic `id` on the manager's team: its formation and play style always and,
/// for a tactic that holds them, its instructions, eleven, roles and takers.
///
/// A saved player who has left the club gives his slot back to the player the team has there
/// now (or to another of the team's current eleven), and a taker who has left leaves the team's
/// current taker in place: a tactic made last season still applies after a sale. Nothing is
/// written if the tactic is missing or its formation or style are not ones the game has.
pub fn apply_custom_tactic(game: &mut Game, id: &str) -> Result<(), String> {
    let id = id.trim();
    let tactic = game
        .custom_tactics
        .iter()
        .find(|tactic| tactic.id.trim() == id)
        .cloned()
        .ok_or_else(|| NOT_FOUND_ERROR.to_string())?;
    if parse_formation(&tactic.formation).is_none() {
        return Err(INVALID_FORMATION_ERROR.to_string());
    }
    let play_style: PlayStyle =
        serde_json::from_value(serde_json::Value::String(tactic.play_style.clone()))
            .map_err(|_| INVALID_PLAY_STYLE_ERROR.to_string())?;
    let team_id = managed_team(game)?.id.clone();
    let at_the_club: HashSet<String> = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id.as_str()))
        .map(|player| player.id.clone())
        .collect();
    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == team_id)
        .ok_or_else(|| MANAGED_TEAM_NOT_FOUND_ERROR.to_string())?;

    team.formation = tactic.formation.clone();
    team.play_style = play_style;
    let Some(contents) = tactic.contents else {
        return Ok(());
    };
    team.tactics_phase = contents.instructions;
    let lineup = restored_eleven(&contents.slots, &team.starting_xi_ids, &at_the_club);
    let positions = formation_slots(&team.formation);
    for (slot, saved) in contents.slots.iter().enumerate() {
        if lineup.get(slot) != Some(&saved.player_id) {
            continue;
        }
        // A role the slot no longer admits (an edited save, a changed role table) is left at the
        // position's standard one rather than carried into the match.
        let admitted = saved.role.as_ref().filter(|role| {
            positions
                .get(slot)
                .is_some_and(|position| position.admits_role(role))
        });
        match admitted {
            Some(role) => team
                .player_roles
                .insert(saved.player_id.clone(), role.clone()),
            None => team.player_roles.remove(&saved.player_id),
        };
    }
    team.starting_xi_ids = lineup;
    team.match_roles = restored_takers(&contents.match_roles, &team.match_roles, &at_the_club);
    Ok(())
}

/// Renames the saved tactic `id`, keeping everything else it holds.
pub fn rename_custom_tactic(game: &mut Game, id: &str, name: &str) -> Result<(), String> {
    let id = id.trim();
    let mut tactic = game
        .custom_tactics
        .iter()
        .find(|tactic| tactic.id.trim() == id)
        .cloned()
        .ok_or_else(|| NOT_FOUND_ERROR.to_string())?;
    tactic.name = name.to_string();
    save_custom_tactic(game, &tactic)
}

const NO_TEAM_ERROR: &str = "be.error.noTeamAssigned";
const MANAGED_TEAM_NOT_FOUND_ERROR: &str = "be.error.managedTeamNotFound";

fn managed_team(game: &Game) -> Result<&Team, String> {
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or_else(|| NO_TEAM_ERROR.to_string())?;
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or_else(|| MANAGED_TEAM_NOT_FOUND_ERROR.to_string())
}

/// The saved eleven, slot by slot, with each player who has left the club replaced by the one the
/// team has in that slot now, or else by another of its current eleven not already placed.
fn restored_eleven(
    saved: &[SavedSlot],
    current: &[String],
    at_the_club: &HashSet<String>,
) -> Vec<String> {
    let staying: HashSet<&str> = saved
        .iter()
        .map(|slot| slot.player_id.as_str())
        .filter(|id| at_the_club.contains(*id))
        .collect();
    // A current starter is free to fill a gap only if the saved eleven doesn't place him itself.
    let free = |id: &&String, placed: &[String]| {
        at_the_club.contains(id.as_str()) && !staying.contains(id.as_str()) && !placed.contains(id)
    };
    let mut placed: Vec<String> = Vec::with_capacity(saved.len());
    for (slot, saved_slot) in saved.iter().enumerate() {
        let player = if staying.contains(saved_slot.player_id.as_str()) {
            Some(saved_slot.player_id.clone())
        } else {
            current
                .get(slot)
                .filter(|id| free(id, &placed))
                .or_else(|| current.iter().find(|id| free(id, &placed)))
                .cloned()
        };
        if let Some(player) = player {
            placed.push(player);
        }
    }
    placed
}

/// The saved takers, each one who has left the club replaced by the team's current taker.
fn restored_takers(
    saved: &MatchRoles,
    current: &MatchRoles,
    at_the_club: &HashSet<String>,
) -> MatchRoles {
    let keep = |saved: &Option<String>, current: &Option<String>| match saved {
        Some(id) if at_the_club.contains(id) => saved.clone(),
        _ => current.clone(),
    };
    MatchRoles {
        captain: keep(&saved.captain, &current.captain),
        vice_captain: keep(&saved.vice_captain, &current.vice_captain),
        penalty_taker: keep(&saved.penalty_taker, &current.penalty_taker),
        free_kick_taker: keep(&saved.free_kick_taker, &current.free_kick_taker),
        corner_taker: keep(&saved.corner_taker, &current.corner_taker),
    }
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
            contents: None,
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

/// #1189: a saved tactic is the whole tactic.
#[cfg(test)]
mod whole_tactic_tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::world::test_fixtures::{manager_for, nation_team, player_at};
    use chrono::{TimeZone, Utc};
    use domain::team::{
        BreakSpeed, BuildUpStyle, DefensiveLine, PlayerRole, PressingIntensity,
        TacticsPhaseSettings,
    };

    const CLUB: &str = "alpha";

    fn eleven(prefix: &str) -> Vec<String> {
        (0..11).map(|n| format!("{prefix}{n}")).collect()
    }

    /// A club managed by the player with sixteen players: "s0".."s10" and "b0".."b4".
    fn game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 7, 1, 12, 0, 0).unwrap());
        let players = eleven("s")
            .into_iter()
            .chain((0..5).map(|n| format!("b{n}")))
            .map(|id| player_at(&id, CLUB, "2034-06-30"))
            .collect();
        Game::new(
            clock,
            manager_for(CLUB),
            vec![nation_team(CLUB, "England", 50)],
            players,
            vec![],
            vec![],
        )
    }

    fn team(game: &mut Game) -> &mut Team {
        &mut game.teams[0]
    }

    fn tuned() -> TacticsPhaseSettings {
        TacticsPhaseSettings {
            build_up_style: BuildUpStyle::Short,
            defensive_line: DefensiveLine::High,
            pressing_intensity: PressingIntensity::Aggressive,
            break_speed: BreakSpeed::Fast,
            ..TacticsPhaseSettings::default()
        }
    }

    /// The club set up as a manager would leave it: 4-4-2 on the counter, tuned dials, the "s"
    /// eleven with a sweeper keeper and a stopper, and every taker named.
    fn set_up(game: &mut Game) {
        let team = team(game);
        team.formation = "4-4-2".into();
        team.play_style = PlayStyle::Counter;
        team.tactics_phase = tuned();
        team.starting_xi_ids = eleven("s");
        team.player_roles
            .insert("s0".into(), PlayerRole::SweeperKeeper);
        team.player_roles.insert("s2".into(), PlayerRole::Stopper);
        team.match_roles = MatchRoles {
            captain: Some("s5".into()),
            vice_captain: Some("s2".into()),
            penalty_taker: Some("s9".into()),
            free_kick_taker: Some("s6".into()),
            corner_taker: Some("s8".into()),
        };
    }

    /// The club changed in every part the tactic holds: another shape, style, dials, eleven,
    /// no roles, other takers.
    fn change_everything(game: &mut Game) {
        let team = team(game);
        team.formation = "4-3-3".into();
        team.play_style = PlayStyle::Balanced;
        team.tactics_phase = TacticsPhaseSettings::default();
        team.starting_xi_ids = [
            "b0", "b1", "b2", "b3", "b4", "s5", "s6", "s7", "s8", "s9", "s10",
        ]
        .map(String::from)
        .to_vec();
        team.player_roles.clear();
        team.match_roles = MatchRoles {
            captain: Some("b0".into()),
            vice_captain: Some("b1".into()),
            penalty_taker: Some("b2".into()),
            free_kick_taker: Some("b3".into()),
            corner_taker: Some("b4".into()),
        };
    }

    fn draft(id: &str, name: &str) -> CustomTactic {
        CustomTactic {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            // What the screen sends for the shape is ignored: the team is the source.
            formation: "5-3-2".into(),
            play_style: "Attacking".into(),
            source_preset_name: None,
            contents: None,
        }
    }

    /// Given a club set up with its shape, style, tuned dials, an eleven with two roles and
    /// every set-piece taker
    /// When the manager saves the current tactic as "Away"
    /// Then the saved tactic holds all of it, slot by slot, with the team's shape and style
    ///      rather than the draft's.
    #[test]
    fn saving_the_current_tactic_keeps_everything_the_team_has() {
        let mut game = game();
        set_up(&mut game);

        save_current_tactic(&mut game, &draft("away", "Away")).unwrap();

        let saved = &game.custom_tactics[0];
        assert_eq!(saved.name, "Away");
        assert_eq!(
            (saved.formation.as_str(), saved.play_style.as_str()),
            ("4-4-2", "Counter")
        );
        let contents = saved
            .contents
            .as_ref()
            .expect("the tactic holds its contents");
        assert_eq!(contents.instructions, tuned());
        assert_eq!(
            contents
                .slots
                .iter()
                .map(|slot| slot.player_id.as_str())
                .collect::<Vec<_>>(),
            eleven("s").iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(contents.slots[0].role, Some(PlayerRole::SweeperKeeper));
        assert_eq!(contents.slots[2].role, Some(PlayerRole::Stopper));
        assert_eq!(contents.slots[1].role, None);
        assert_eq!(contents.match_roles.corner_taker.as_deref(), Some("s8"));
    }

    /// Given a saved tactic, and a club since changed in every part the tactic holds
    /// When the tactic is applied
    /// Then the club is back as it was saved: shape, style, dials, eleven, roles and takers.
    #[test]
    fn applying_a_saved_tactic_restores_everything_it_holds() {
        let mut game = game();
        set_up(&mut game);
        save_current_tactic(&mut game, &draft("away", "Away")).unwrap();
        let as_saved = game.teams[0].clone();
        change_everything(&mut game);

        apply_custom_tactic(&mut game, "away").unwrap();

        let team = &game.teams[0];
        assert_eq!(team.formation, "4-4-2");
        assert_eq!(team.play_style, PlayStyle::Counter);
        assert_eq!(team.tactics_phase, tuned());
        assert_eq!(team.starting_xi_ids, as_saved.starting_xi_ids);
        assert_eq!(team.player_roles, as_saved.player_roles);
        assert_eq!(team.match_roles, as_saved.match_roles);
    }

    /// Given a tactic saved before tactics held their contents (formation and style only)
    /// When it is applied to a club with tuned dials, its own eleven and takers
    /// Then only the formation and style change: the rest is the club's, as before #1189.
    #[test]
    fn a_tactic_saved_before_it_held_contents_applies_only_its_shape_and_style() {
        let mut game = game();
        set_up(&mut game);
        let before = game.teams[0].clone();
        let mut old = draft("old", "Old");
        old.formation = "4-3-3".into();
        old.play_style = "HighPress".into();
        game.custom_tactics.push(old);

        apply_custom_tactic(&mut game, "old").unwrap();

        let team = &game.teams[0];
        assert_eq!(team.formation, "4-3-3");
        assert_eq!(team.play_style, PlayStyle::HighPress);
        assert_eq!(team.tactics_phase, before.tactics_phase);
        assert_eq!(team.starting_xi_ids, before.starting_xi_ids);
        assert_eq!(team.player_roles, before.player_roles);
        assert_eq!(team.match_roles, before.match_roles);
    }

    /// Given a saved tactic whose centre-back "s2" (a stopper, the vice-captain) has since left,
    /// and a club now fielding "b2" in his slot
    /// When the tactic is applied
    /// Then "b2" keeps that slot, "s2"'s role isn't given to anyone, the rest of the eleven is
    ///      as saved, and the vice-captaincy stays with the club's current choice.
    #[test]
    fn a_player_who_has_left_gives_his_slot_and_his_armband_back() {
        let mut game = game();
        set_up(&mut game);
        save_current_tactic(&mut game, &draft("away", "Away")).unwrap();
        change_everything(&mut game);
        let s2 = game.players.iter_mut().find(|p| p.id == "s2").unwrap();
        s2.team_id = Some("elsewhere".into());

        apply_custom_tactic(&mut game, "away").unwrap();

        let team = &game.teams[0];
        let mut expected = eleven("s");
        expected[2] = "b2".into();
        assert_eq!(team.starting_xi_ids, expected);
        assert_eq!(team.player_roles.get("s2"), None);
        assert_eq!(team.player_roles.get("b2"), None);
        assert_eq!(team.match_roles.vice_captain.as_deref(), Some("b1"));
        assert_eq!(team.match_roles.captain.as_deref(), Some("s5"));
    }

    /// Given a saved tactic giving its centre-back slot a keeper's role (an edited save)
    /// When it is applied
    /// Then that player plays the slot's standard role; the keeper's own role is kept.
    #[test]
    fn a_role_the_slot_does_not_admit_is_left_at_the_standard_one() {
        let mut game = game();
        set_up(&mut game);
        save_current_tactic(&mut game, &draft("away", "Away")).unwrap();
        if let Some(contents) = game.custom_tactics[0].contents.as_mut() {
            contents.slots[2].role = Some(PlayerRole::SweeperKeeper);
        }
        change_everything(&mut game);

        apply_custom_tactic(&mut game, "away").unwrap();

        let roles = &game.teams[0].player_roles;
        assert_eq!(roles.get("s2"), None);
        assert_eq!(roles.get("s0"), Some(&PlayerRole::SweeperKeeper));
    }

    /// Given a saved tactic
    /// When it is renamed to "  Home  ", to a blank name, and a missing one is renamed
    /// Then it keeps everything under the trimmed new name; the blank and the missing are refused
    ///      with their keys and change nothing.
    #[test]
    fn renaming_keeps_the_tactic_and_refuses_a_blank_or_missing_one() {
        let mut game = game();
        set_up(&mut game);
        save_current_tactic(&mut game, &draft("away", "Away")).unwrap();
        let contents = game.custom_tactics[0].contents.clone();

        rename_custom_tactic(&mut game, "away", "  Home  ").unwrap();

        assert_eq!(game.custom_tactics[0].name, "Home");
        assert_eq!(game.custom_tactics[0].contents, contents);
        assert_eq!(
            rename_custom_tactic(&mut game, "away", "   "),
            Err(NAME_REQUIRED_ERROR.to_string())
        );
        assert_eq!(
            rename_custom_tactic(&mut game, "missing", "Name"),
            Err(NOT_FOUND_ERROR.to_string())
        );
        assert_eq!(game.custom_tactics.len(), 1);
        assert_eq!(game.custom_tactics[0].name, "Home");
    }

    /// Given a club and no tactic "missing"
    /// When "missing" is applied
    /// Then it is refused as not found and the club is unchanged.
    #[test]
    fn applying_a_missing_tactic_changes_nothing() {
        let mut game = game();
        set_up(&mut game);
        let before = game.teams[0].clone();

        assert_eq!(
            apply_custom_tactic(&mut game, "missing"),
            Err(NOT_FOUND_ERROR.to_string())
        );
        assert_eq!(game.teams[0].formation, before.formation);
        assert_eq!(game.teams[0].starting_xi_ids, before.starting_xi_ids);
    }

    /// Given a manager without a club
    /// When the current tactic is saved
    /// Then it is refused with the no-team key and nothing is saved.
    #[test]
    fn a_manager_without_a_club_has_no_tactic_to_save() {
        let mut game = game();
        game.manager.team_id = None;

        assert_eq!(
            save_current_tactic(&mut game, &draft("away", "Away")),
            Err(NO_TEAM_ERROR.to_string())
        );
        assert!(game.custom_tactics.is_empty());
    }
}
