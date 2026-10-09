//! What each MCP tool returns on success.
//!
//! One struct per tool. The server serializes it as `structuredContent` and renders the same
//! value, through `Display`, as the readable text, so the two cannot disagree. Test clients
//! deserialize the same structs. Fields are plain strings and numbers: this crate depends on
//! nothing in the workspace, so the wire contract cannot be changed by editing a game type.

pub mod club;
pub mod contracts;
pub mod game;
pub mod help;
pub mod inbox;
pub mod info;
pub mod live_match;
pub mod scouting;
pub mod season;
pub mod squad;
pub mod time;
pub mod training;
pub mod transfers;

use std::fmt::Display;

use serde::Serialize;

/// Marks a type as the success result of one tool. The server accepts nothing else, so a tool
/// cannot go back to returning prose.
pub trait ToolResult: Serialize + Display {}

/// Implements [`ToolResult`] for each listed result struct.
macro_rules! tool_results {
    ($($result:ty),+ $(,)?) => {
        $(impl $crate::ToolResult for $result {})+
    };
}
pub(crate) use tool_results;

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use serde_json::json;

    use super::*;

    fn round_trips<T: ToolResult + DeserializeOwned + PartialEq + std::fmt::Debug>(value: T) {
        let wire = serde_json::to_value(&value).unwrap();
        assert!(
            wire.is_object(),
            "structuredContent must be an object: {wire}"
        );
        let back: T = serde_json::from_value(wire).unwrap();
        assert_eq!(back, value);
    }

    /// Given a result that is a tagged enum
    /// When it is serialized
    /// Then it is an object that names its variant, and a client reads it back.
    #[test]
    fn tagged_results_are_objects_a_client_can_read_back() {
        round_trips(time::SkipToMatchDay::Skipped {
            days_advanced: 3,
            target_date: "2026-08-15".to_string(),
        });
        round_trips(time::SkipToMatchDay::NoUpcomingMatch {});
        round_trips(season::SeasonStatus::InProgress {
            remaining_fixtures: 4,
        });
        round_trips(info::GameStatus::ManagerFired {});

        assert_eq!(
            serde_json::to_value(season::SeasonStatus::Complete {}).unwrap(),
            json!({"status": "complete"})
        );
    }

    /// Given results with no fields
    /// When they are serialized
    /// Then they are still objects, because structuredContent must be one.
    #[test]
    fn a_result_with_no_fields_is_an_empty_object() {
        assert_eq!(
            serde_json::to_value(inbox::AllMessagesMarkedRead {}).unwrap(),
            json!({})
        );
        round_trips(inbox::OldMessagesCleared {});
    }

    /// Given a day with a user match, a standings line, a dismissal and an auto-save
    /// When the result is rendered
    /// Then the text carries each of them in the order the agent has always read.
    #[test]
    fn a_day_advanced_reads_in_order() {
        let day = time::DayAdvanced {
            date: "15 August 2026".to_string(),
            results: vec![time::PlayedMatch {
                home_team: "Reds".to_string(),
                home_goals: 2,
                away_goals: 1,
                away_team: "Blues".to_string(),
            }],
            your_match: Some(time::YourMatch {
                outcome: time::Outcome::Won,
                your_goals: 2,
                their_goals: 1,
                opponent: "Blues".to_string(),
                at_home: true,
            }),
            standings: Some(time::StandingsUpdate {
                position: 1,
                points: 3,
                goal_difference: 1,
            }),
            fired: true,
            auto_saved: true,
        };

        let text = day.to_string();

        let positions: Vec<usize> = [
            "## Day Advanced — 15 August 2026",
            "| Reds | 2 - 1 | Blues |",
            "Your team won 2-1 vs Blues (H).",
            "League position: 1 | Points: 3 | GD: +1",
            "You have been fired!",
            "Auto-saved.",
        ]
        .iter()
        .map(|part| {
            text.find(part)
                .unwrap_or_else(|| panic!("missing {part}: {text}"))
        })
        .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
        round_trips(day);
    }

    /// Given an inbox of 25 messages
    /// When it is rendered
    /// Then the text lists 20 and counts the rest, while the structure keeps all 25.
    #[test]
    fn the_inbox_text_lists_twenty_but_the_structure_keeps_all() {
        let inbox = inbox::InboxMessages {
            messages: (0..25)
                .map(|i| inbox::InboxMessage {
                    id: format!("m{i}"),
                    subject: "Hello".to_string(),
                    category: "Welcome".to_string(),
                    read: false,
                    date: "2026-08-01".to_string(),
                })
                .collect(),
        };

        let text = inbox.to_string();

        assert!(text.contains("## Inbox (25 messages)"));
        assert!(text.contains("| m19 |") && !text.contains("| m20 |"));
        assert!(text.contains("... and 5 more."));
        assert_eq!(
            serde_json::to_value(&inbox).unwrap()["messages"]
                .as_array()
                .unwrap()
                .len(),
            25
        );
    }

    /// Given a player profile for a squad member with 19 attributes
    /// When it is rendered
    /// Then attributes are laid out three to a row, and the structure holds all 19.
    #[test]
    fn a_profile_lays_attributes_three_to_a_row() {
        let profile = info::PlayerProfile {
            id: "p1".to_string(),
            match_name: "A. Player".to_string(),
            full_name: "Alan Player".to_string(),
            position: "ST".to_string(),
            age: "24".to_string(),
            nationality: "England".to_string(),
            team: "Reds".to_string(),
            detail: info::PlayerDetail::Own {
                ovr: 70,
                condition: 90,
                morale: 80,
                fitness: 85,
                wage: 1000,
                contract_end: None,
                injury: None,
                attributes: (0..19)
                    .map(|i| info::Attribute {
                        name: format!("A{i}"),
                        value: i,
                    })
                    .collect(),
            },
        };

        let text = profile.to_string();

        assert!(text.contains("| A0 | 0 | A1 | 1 | A2 | 2 |"));
        assert!(text.contains("| A15 | 15 | A16 | 16 | A17 | 17 |"));
        assert!(text.contains("| Contract End | - |"));
        round_trips(profile);
    }

    /// Given a winner in a player award and in the manager award
    /// When the awards are rendered
    /// Then each table keeps its own header rule, as the text always had.
    #[test]
    fn award_tables_keep_their_header_rules() {
        let winner = season::AwardWinner {
            name: "N".to_string(),
            team: "T".to_string(),
            value: 12.0,
        };
        let awards = season::SeasonAwards {
            golden_boot: vec![winner.clone()],
            assist_king: vec![],
            player_of_year: vec![],
            clean_sheet_king: vec![],
            most_appearances: vec![],
            young_player: vec![],
            manager_of_season: vec![winner],
        };

        let text = awards.to_string();

        assert!(text.contains(
            "### 🏆 Golden Boot\n\n| # | Player | Team | Value |\n|---|--------|------|-------|\n| 1 | N | T | 12.0 |\n\n"
        ));
        assert!(text.contains(
            "### 👔 Manager of the Season\n\n| # | Manager | Team | Value |\n|---|---------|------|-------|\n| 1 | N | T | 12.0 |\n"
        ));
        round_trips(awards);
    }

    /// Given a scout just dispatched, so an assignment exists but no report yet
    /// When the reports are rendered
    /// Then the text still shows the pending assignment, as the structure does.
    #[test]
    fn a_pending_assignment_shows_before_any_report_exists() {
        let reports = scouting::ScoutReports {
            reports: vec![],
            active_assignments: vec![scouting::ScoutingAssignment {
                id: "a1".to_string(),
                scout_name: "S. Cout".to_string(),
                player_name: "A. Player".to_string(),
                days_remaining: 5,
            }],
        };

        let text = reports.to_string();

        assert!(text.contains("No scout reports available."));
        assert!(text.contains("### Active Assignments (1 pending)"));
        assert!(text.contains("| a1 | S. Cout | A. Player | 5 |"));
    }
}
