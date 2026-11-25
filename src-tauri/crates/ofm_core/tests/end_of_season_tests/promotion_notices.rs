use super::*;

/// Given a generated third-tier closing division
/// When the user is promoted into it
/// Then the notice carries its key, country, edition, tier and phase.
#[test]
fn promotion_message_carries_division_display_metadata() {
    let mut game = make_two_division_game("team3");
    for competition in &mut game.competitions {
        if competition.name == "ENG First Division" {
            competition.name = "ENG Division 3".to_string();
            competition.name_key = Some("tournaments.competitions.tierDivision".to_string());
            competition.calendar = Some(CalendarMetadata {
                season: domain::competition_calendar::SeasonCalendar {
                    division: Some(domain::competition_calendar::DivisionIdentity {
                        family_id: "eng-league".to_string(),
                        tier: 3,
                        phase: domain::competition_calendar::SeasonPhase::Closing,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            });
        }
    }

    process_end_of_season(&mut game);

    let renewed = ofm_core::end_of_season::club_league_table(&game, "team3")
        .expect("the user still has a division")
        .season;
    let msg = game
        .messages
        .iter()
        .find(|m| m.id == format!("promotion_{renewed}"))
        .expect("promoted user must be told");
    assert_eq!(
        msg.i18n_params.get("divisionKey"),
        Some(&"tournaments.competitions.tierDivision".to_string())
    );
    assert_eq!(
        msg.i18n_params.get("divisionCountry"),
        Some(&"ENG".to_string())
    );
    assert_eq!(
        msg.i18n_params.get("divisionSeason"),
        Some(&renewed.to_string())
    );
    assert_eq!(msg.i18n_params.get("divisionTier"), Some(&"3".to_string()));
    assert_eq!(
        msg.i18n_params.get("divisionPhase"),
        Some(&"closing".to_string())
    );
    assert_eq!(
        msg.i18n_params.get("division"),
        Some(&"ENG Division 3".to_string()),
        "the stored name stays as the fallback"
    );
}

/// Given a division with no name key (a hand-made one)
/// When the user is promoted into it
/// Then the notice carries only the stored name, as before.
#[test]
fn promotion_message_without_a_name_key_carries_no_key_params() {
    let mut game = make_two_division_game("team3");

    process_end_of_season(&mut game);

    let renewed = ofm_core::end_of_season::club_league_table(&game, "team3")
        .expect("the user still has a division")
        .season;
    let msg = game
        .messages
        .iter()
        .find(|m| m.id == format!("promotion_{renewed}"))
        .expect("promoted user must be told");
    assert!(!msg.i18n_params.contains_key("divisionKey"));
    assert!(!msg.i18n_params.contains_key("divisionCountry"));
}
