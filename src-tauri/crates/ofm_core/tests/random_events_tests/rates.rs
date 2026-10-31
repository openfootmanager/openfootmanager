//! How often the club's random events come, how much they move a player, and which message an
//! answer settles. Rates are counted over a thousand days of one world, so each is many rolls of
//! the day's dice, never one.

use super::*;

/// Every message id the random events send over `days` days from `make_game`, with injuries
/// healed each morning so nobody drops out of the eligible squad. `prepare` runs before each day.
fn message_ids_over(days: usize, mut prepare: impl FnMut(&mut Game)) -> Vec<String> {
    let mut game = make_game();
    for _ in 0..days {
        for player in &mut game.players {
            player.injury = None;
        }
        prepare(&mut game);
        check_random_events(&mut game);
        game.clock.advance_days(1);
    }
    game.messages
        .into_iter()
        .map(|message| message.id)
        .collect()
}

fn count_with(ids: &[String], prefix: &str) -> usize {
    ids.iter().filter(|id| id.starts_with(prefix)).count()
}

/// Given a thousand days of the club's life
/// When the random events roll each day
/// Then a media story comes on between 5 and 50 of them: about one day in fifty
#[test]
fn media_stories_come_about_one_day_in_fifty() {
    let count = count_with(&message_ids_over(1000, |_| {}), "media_");

    assert!((5..=50).contains(&count), "{count} in 1000 days");
}

/// Given a thousand days of the club's life
/// When the random events roll each day
/// Then a fan petition comes on between 5 and 50 of them: about one day in fifty
#[test]
fn fan_petitions_come_about_one_day_in_fifty() {
    let count = count_with(&message_ids_over(1000, |_| {}), "fan_petition_");

    assert!((5..=50).contains(&count), "{count} in 1000 days");
}

/// Given a thousand days of the club's life
/// When the random events roll each day
/// Then a rival's interest in a player is reported on between 5 and 50 of them: about one day in
///      fifty
#[test]
fn rival_interest_comes_about_one_day_in_fifty() {
    let count = count_with(&message_ids_over(1000, |_| {}), "rival_interest_");

    assert!((5..=50).contains(&count), "{count} in 1000 days");
}

/// Given a thousand days of the club's life
/// When the random events roll each day
/// Then a community event comes on between 2 and 30 of them: about one day in a hundred
#[test]
fn community_events_come_about_one_day_in_a_hundred() {
    let count = count_with(&message_ids_over(1000, |_| {}), "community_");

    assert!((2..=30).contains(&count), "{count} in 1000 days");
}

/// The user's next match, three days from the game's current date.
fn a_match_in_three_days(game: &mut Game) {
    let date = (game.clock.current_date.date_naive() + chrono::Duration::days(3))
        .format("%Y-%m-%d")
        .to_string();
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "next".to_string(),
            matchday: 1,
            date,
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            ..Default::default()
        }],
        standings: vec![StandingEntry::new("team1".to_string())],
        ..Default::default()
    });
}

/// Given a thousand days, each with the user's next match three days away
/// When the random events roll each day
/// Then an international call-up comes on between 5 and 50 of them: about one day in fifty
#[test]
fn international_callups_come_about_one_day_in_fifty_before_a_match() {
    let count = count_with(
        &message_ids_over(1000, a_match_in_three_days),
        "intl_callup_",
    );

    assert!((5..=50).contains(&count), "{count} in 1000 days");
}

/// Given the squad at morale 70 and a match three days away, over many seeds of one day
/// When a seed brings an international call-up (and no media story that might touch him too)
/// Then the called-up player's morale rises by 3 to 8
#[test]
fn an_international_callup_lifts_the_players_morale_by_three_to_eight() {
    let mut seen = 0;
    for seed in 0..2000 {
        let mut game = make_game();
        game.seed = seed;
        a_match_in_three_days(&mut game);

        check_random_events(&mut game);

        let Some(callup) = game
            .messages
            .iter()
            .find(|m| m.id.starts_with("intl_callup_"))
        else {
            continue;
        };
        if game.messages.iter().any(|m| m.id.starts_with("media_")) {
            continue;
        }
        let called = callup
            .id
            .strip_prefix("intl_callup_")
            .and_then(|rest| rest.rsplit_once('_'))
            .map(|(player, _)| player.to_string())
            .expect("a call-up names its player");
        let morale = game.players.iter().find(|p| p.id == called).unwrap().morale;
        assert!(
            (73..=78).contains(&morale),
            "seed {seed}: {called} at {morale}"
        );
        seen += 1;
    }
    assert!(seen > 0, "no seed brought a call-up");
}

/// An actionable message about nothing, sent before the one under test.
fn an_unrelated_message() -> InboxMessage {
    let mut message = fan_petition_msg();
    message.id = "unrelated_2025-06-15".to_string();
    message
}

fn resolved(game: &Game, id: &str) -> bool {
    game.messages
        .iter()
        .find(|m| m.id == id)
        .unwrap()
        .actions
        .iter()
        .all(|action| action.resolved)
}

/// Given the squad at morale 50 and a fan petition
/// When the manager listens to the fans
/// Then every squad player's morale rises by 1 to 3
#[test]
fn listening_to_fans_lifts_each_squad_player_by_one_to_three() {
    let mut game = make_game();
    for player in &mut game.players {
        player.morale = 50;
    }
    game.messages.push(fan_petition_msg());

    apply_event_response(
        &mut game,
        "fan_petition_2025-06-15",
        "respond",
        "listen_fans",
    )
    .expect("the answer is applied");

    for player in &game.players {
        assert!(
            (51..=53).contains(&player.morale),
            "{}: {}",
            player.id,
            player.morale
        );
    }
}

/// Given an unrelated actionable message, then a fan petition
/// When the manager listens to the fans
/// Then the petition is settled and the other message is not
#[test]
fn listening_to_fans_resolves_the_petition_and_no_other_message() {
    let mut game = make_game();
    game.messages.push(an_unrelated_message());
    game.messages.push(fan_petition_msg());

    apply_event_response(
        &mut game,
        "fan_petition_2025-06-15",
        "respond",
        "listen_fans",
    )
    .expect("the answer is applied");

    assert_eq!(
        (
            resolved(&game, "fan_petition_2025-06-15"),
            resolved(&game, "unrelated_2025-06-15")
        ),
        (true, false)
    );
}

/// Given p0 at morale 50 and a rival's interest in him
/// When the manager says he is not for sale
/// Then his morale rises by 3 to 8
#[test]
fn not_for_sale_lifts_the_player_by_three_to_eight() {
    let mut game = make_game();
    game.players[0].morale = 50;
    game.messages.push(rival_interest_msg("p0"));

    apply_event_response(
        &mut game,
        "rival_interest_2025-06-15",
        "respond",
        "not_for_sale",
    )
    .expect("the answer is applied");

    assert!(
        (53..=58).contains(&game.players[0].morale),
        "{}",
        game.players[0].morale
    );
}

/// Given an unrelated actionable message, then a rival's interest in p0
/// When the manager says he is not for sale
/// Then the rival's message is settled and the other message is not
#[test]
fn not_for_sale_resolves_the_rival_message_and_no_other() {
    let mut game = make_game();
    game.messages.push(an_unrelated_message());
    game.messages.push(rival_interest_msg("p0"));

    apply_event_response(
        &mut game,
        "rival_interest_2025-06-15",
        "respond",
        "not_for_sale",
    )
    .expect("the answer is applied");

    assert_eq!(
        (
            resolved(&game, "rival_interest_2025-06-15"),
            resolved(&game, "unrelated_2025-06-15")
        ),
        (true, false)
    );
}

/// Given a player of ordinary value on a long contract, and a 2,000,000 player in the final weeks
///       of his
/// When their weight as a rival's target is worked out
/// Then it is 2 (1 for the contract, 1 for value) and 19 (16 and 3)
#[test]
fn rival_interest_weight_adds_contract_and_value_weights() {
    let game = make_game();
    let today = game.clock.current_date.date_naive();
    let ordinary = make_player("ordinary", "Ordinary", "team1");
    let mut wanted = make_player("wanted", "Wanted", "team1");
    wanted.market_value = 2_000_000;
    wanted.stage_contract_end(Some("2025-07-05".to_string()));

    assert_eq!(
        (
            rival_interest_weight(&ordinary, today),
            rival_interest_weight(&wanted, today)
        ),
        (2, 19)
    );
}

/// Given fourteen days from Sunday 15 June 2025
/// When the random events roll each day
/// Then exactly two mood reports come, both on a Monday
#[test]
fn mood_report_arrives_only_on_mondays() {
    let ids = message_ids_over(14, |_| {});

    let reports: Vec<&String> = ids
        .iter()
        .filter(|id| id.starts_with("mood_report_"))
        .collect();
    assert_eq!(
        reports,
        vec!["mood_report_2025-06-16", "mood_report_2025-06-23"]
    );
}

/// The club names a day's messages carry, for a world whose first team is not the user's.
fn messages_with_another_club_listed_first() -> Vec<InboxMessage> {
    let mut all = Vec::new();
    for seed in 0..400 {
        let mut game = make_game();
        game.seed = seed;
        game.teams.insert(0, make_team("team0", "Other Town"));
        check_random_events(&mut game);
        all.extend(game.messages);
    }
    all
}

/// Given a world whose first club listed is not the user's, over many seeds of one day
/// When a fan petition or a community event is sent
/// Then it names the user's club, Test FC
#[test]
fn fan_petition_and_community_messages_name_the_users_club() {
    let messages = messages_with_another_club_listed_first();
    let named: Vec<&InboxMessage> = messages
        .iter()
        .filter(|m| m.id.starts_with("fan_petition_") || m.id.starts_with("community_"))
        .collect();

    assert!(
        !named.is_empty(),
        "no petition or community event in 400 seeds"
    );
    for message in named {
        assert!(
            message.i18n_params.values().any(|value| value == "Test FC"),
            "{}: {:?}",
            message.id,
            message.i18n_params
        );
    }
}
