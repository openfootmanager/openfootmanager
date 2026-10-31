//! What each MCP tool returns on success.
//!
//! One struct per tool. The server serializes it as `structuredContent` and renders the same
//! value, through `Display`, as the readable text, so the two cannot disagree. Test clients
//! deserialize the same structs. Fields are plain strings and numbers: this crate depends on
//! nothing in the workspace, so the wire contract cannot be changed by editing a game type.
//!
//! The exceptions are fields that pass a game or command type through as untyped JSON, whose
//! shape follows that type: `info::GameState::game`, `info::PlayerStats::stats`,
//! `info::TeamStats::stats` and `season::SeasonAdvanced::summary`.

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

    /// Given a page of archived mail with more behind it
    /// When it is rendered and sent
    /// Then the text names the cursor to continue from, and the structure round-trips.
    #[test]
    fn an_archive_page_names_where_to_continue() {
        let page = inbox::ArchivedMessages {
            messages: vec![inbox::ArchivedMessage {
                id: "m1".to_string(),
                subject: "Old news".to_string(),
                body: "Body".to_string(),
                category: "Welcome".to_string(),
                date: "2030-01-01".to_string(),
            }],
            next_before_date: Some("2030-01-01".to_string()),
            next_before_id: Some("m1".to_string()),
        };

        let text = page.to_string();

        assert!(text.contains("before_date=2030-01-01 before_id=m1"));
        // The text is what an agent reads when it ignores structuredContent: the body is in it.
        assert!(text.contains("Body"), "{text}");
        round_trips(page);
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
        assert!(
            text.contains("| A18 | 18 |"),
            "the 19th attribute fills a short last row and must not be dropped: {text}"
        );
        assert!(text.contains("| Contract End | - |"));
        round_trips(profile);
    }

    fn identity() -> info::PlayerProfile {
        info::PlayerProfile {
            id: "p1".to_string(),
            match_name: "A. Player".to_string(),
            full_name: "Alan Player".to_string(),
            position: "ST".to_string(),
            age: "24".to_string(),
            nationality: "England".to_string(),
            team: "Reds".to_string(),
            detail: info::PlayerDetail::Unknown,
        }
    }

    /// Given a player the club knows nothing about
    /// When the profile is rendered
    /// Then it shows who he is and says no more, with no rating or attribute table.
    #[test]
    fn an_unknown_players_profile_shows_identity_only() {
        let profile = identity();

        let text = profile.to_string();

        assert!(text.contains("| Full Name | Alan Player |"));
        assert!(!text.contains("OVR"));
        assert!(!text.contains("Attributes"));
        round_trips(profile);
    }

    /// Given a scouted player and a rival in the manager's division
    /// When their profiles are rendered
    /// Then the scouted one shows the report's date (flagged when out of date) and the rival his rating.
    #[test]
    fn scouted_and_known_profiles_show_what_the_club_knows() {
        let attributes = vec![
            info::Attribute {
                name: "Pace".to_string(),
                value: 55,
            },
            info::Attribute {
                name: "Shooting".to_string(),
                value: 56,
            },
            info::Attribute {
                name: "Passing".to_string(),
                value: 57,
            },
        ];
        let scouted = info::PlayerProfile {
            detail: info::PlayerDetail::Scouted {
                scouted_on: "2026-03-10".to_string(),
                current: false,
                attributes: attributes.clone(),
            },
            ..identity()
        };
        let known = info::PlayerProfile {
            detail: info::PlayerDetail::Known {
                ovr: 71,
                attributes,
            },
            ..identity()
        };

        assert!(scouted
            .to_string()
            .contains("| Scouted On | 2026-03-10 (out of date) |"));
        assert!(!scouted.to_string().contains("OVR"));
        assert!(known.to_string().contains("| OVR | 71 |"));
        assert!(!known.to_string().contains("Condition"));
        for text in [scouted.to_string(), known.to_string()] {
            assert!(text.contains("### Attributes"), "{text}");
            assert!(
                text.contains("| Pace | 55 | Shooting | 56 | Passing | 57 |"),
                "{text}"
            );
        }
        round_trips(scouted);
        round_trips(known);
    }

    /// Given no blockers, and then a live match with one real blocker
    /// When the blockers are rendered
    /// Then the first says time can advance, and the second lists the live match and the blocker
    ///      with its severity, screen and counts; both survive a round trip.
    #[test]
    fn blockers_render_the_live_match_and_the_games_own_list() {
        let none = time::Blockers {
            blockers: Vec::new(),
            live_match_in_progress: false,
        };
        let some = time::Blockers {
            blockers: vec![time::Blocker {
                id: "injured_xi".to_string(),
                severity: "warn".to_string(),
                tab: "Squad".to_string(),
                params: Some(serde_json::json!({"count": "2"})),
            }],
            live_match_in_progress: true,
        };

        assert!(none.to_string().contains("Time can be advanced safely."));
        let text = some.to_string();
        assert!(text.contains("Live match in progress"), "{text}");
        assert!(
            text.contains("- [warn] injured_xi (resolve on the Squad screen) {\"count\":\"2\"}"),
            "{text}"
        );
        round_trips(none);
        round_trips(some);
    }

    /// Given a skip that a blocker stopped after one day
    /// When it is rendered
    /// Then the days advanced and the blocker are shown, and it survives a round trip.
    #[test]
    fn a_blocked_skip_names_its_blockers() {
        let skip = time::SkipToMatchDay::Blocked {
            days_advanced: 1,
            blockers: vec![time::Blocker {
                id: "injured_xi".to_string(),
                severity: "warn".to_string(),
                tab: "Squad".to_string(),
                params: None,
            }],
        };

        let text = skip.to_string();

        assert!(text.contains("1 days advanced"), "{text}");
        assert!(
            text.contains("- [warn] injured_xi (resolve on the Squad screen)"),
            "{text}"
        );
        round_trips(skip);
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

    /// Given a match-roles update that sets only a captain
    /// When it is rendered
    /// Then the text names the captain and shows the other roles as none, instead of claiming all were set.
    #[test]
    fn match_roles_text_shows_the_roles_it_holds() {
        let roles = squad::MatchRolesUpdated {
            captain: Some("p1".to_string()),
            vice_captain: None,
            penalty_taker: None,
            free_kick_taker: None,
            corner_taker: None,
        };

        let text = roles.to_string();

        assert!(text.contains("Captain: p1"));
        assert!(text.contains("Penalties: none"));
        assert!(!text.contains("set as specified"));
    }

    /// Given a training groups update with three groups
    /// When it is rendered
    /// Then the text says how many groups were set.
    #[test]
    fn training_groups_text_carries_the_group_count() {
        let text = training::TrainingGroupsUpdated { group_count: 3 }.to_string();

        assert!(text.contains("**Groups**: 3"));
    }
}
