//! Telling the user that a load set some of the save's data aside because it could not be read
//! (`db::json_column`): one plain message per newly set-aside batch, pointing to the bug report.

use std::collections::HashMap;

use domain::message::{InboxMessage, MessageCategory, MessagePriority};

use crate::game::Game;
use crate::inbox;

/// The translation keys of the frontend labels the message names, so it says where the bug report
/// is in the player's language and a renamed label renames the message with it.
const SETTINGS_LABEL: &str = "dashboard.settings";
const REPORT_BUG_LABEL: &str = "settings.reportBug";

/// Sends the user one message that the save holds `kept_cells` cells it could not read, unless that
/// count has been announced before. The count is the key, and the message id: the record of kept
/// cells only grows, so a load that sets aside more raises it, while a reload of a save that kept
/// the message finds it in the sent-ledger. None kept, nothing sent.
pub fn announce_set_aside_data(game: &mut Game, kept_cells: usize) {
    if kept_cells == 0 {
        return;
    }
    let id = format!("saved_data_set_aside_{kept_cells}");
    let date = game.clock.current_date.to_rfc3339();
    inbox::emit_once(game, &id, || set_aside_message(&id, &date));
}

fn set_aside_message(id: &str, date: &str) -> InboxMessage {
    InboxMessage::new(
        id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::System)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_i18n(
        "be.msg.savedDataSetAside.subject",
        "be.msg.savedDataSetAside.body",
        HashMap::from([
            ("settings".to_string(), SETTINGS_LABEL.to_string()),
            ("reportBug".to_string(), REPORT_BUG_LABEL.to_string()),
        ]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;

    fn game() -> Game {
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        let clock = GameClock::new(Utc.with_ymd_and_hms(2033, 3, 1, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn notices(game: &Game) -> Vec<&InboxMessage> {
        game.messages
            .iter()
            .filter(|message| {
                message.subject_key.as_deref() == Some("be.msg.savedDataSetAside.subject")
            })
            .collect()
    }

    /// Given a load that kept no cell
    /// When the notice is considered
    /// Then nothing is sent
    #[test]
    fn nothing_kept_tells_the_user_nothing() {
        let mut game = game();

        announce_set_aside_data(&mut game, 0);

        assert!(game.messages.is_empty());
    }

    /// Given a load that kept one cell, then the same count again, then a load that kept a second
    /// When the notice is considered each time
    /// Then one message is sent for the first, none for the repeat, and one more for the second
    #[test]
    fn each_new_count_is_told_once() {
        let mut game = game();

        announce_set_aside_data(&mut game, 1);
        announce_set_aside_data(&mut game, 1);
        assert_eq!(notices(&game).len(), 1);

        announce_set_aside_data(&mut game, 2);
        assert_eq!(notices(&game).len(), 2);
    }

    /// Given the notice
    /// When it is sent
    /// Then it is a system message whose body names the bug report by the frontend's own labels
    #[test]
    fn the_notice_names_the_bug_report_by_its_labels() {
        let mut game = game();

        announce_set_aside_data(&mut game, 1);

        let notice = notices(&game)[0];
        assert_eq!(notice.category, MessageCategory::System);
        assert_eq!(
            notice.body_key.as_deref(),
            Some("be.msg.savedDataSetAside.body")
        );
        assert_eq!(notice.i18n_params["settings"], SETTINGS_LABEL);
        assert_eq!(notice.i18n_params["reportBug"], REPORT_BUG_LABEL);
    }

    /// Given the English locale file the frontend renders the message with
    /// When the keys the notice sends are looked up in it
    /// Then each exists: a renamed label or message key fails here, not as a raw key on screen
    #[test]
    fn every_key_the_notice_sends_is_in_the_locale_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src/i18n/locales/en.json");
        let english: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let lookup = |key: &str| {
            key.split('.')
                .try_fold(&english, |node, part| node.get(part))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        };

        for key in [
            SETTINGS_LABEL,
            REPORT_BUG_LABEL,
            "be.msg.savedDataSetAside.subject",
            "be.msg.savedDataSetAside.body",
            "be.sender.assistantManager",
            "be.role.assistantManager",
        ] {
            assert!(lookup(key).is_some(), "{key} is not in en.json");
        }
        let body = lookup("be.msg.savedDataSetAside.body").unwrap();
        assert!(
            body.contains("{{settings}}") && body.contains("{{reportBug}}"),
            "{body}"
        );
    }
}
