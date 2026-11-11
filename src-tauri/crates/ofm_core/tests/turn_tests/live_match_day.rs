//! The day the player watches their own match is still a day for everyone else.
//!
//! `finish_live_match_day` completes the day *instead of* `process_day`, not after it. The two
//! were separate copies of the same tail, and the live one was missing the steps that move the
//! rest of the world: other competitions due today, the dormant tier, national teams and the
//! World Cup. Because a fixture is only ever due on an exact date match, anything skipped here
//! was skipped for good.

use chrono::Datelike;
use domain::league::{
    CompetitionScope, CompetitionType, Fixture, FixtureCompetition, FixtureStatus,
    KnockoutRoundState, League,
};
use domain::national_team::NationalTeam;
use ofm_core::game::Game;
use ofm_core::turn;

use super::fixtures::{make_game_with_match, make_squad, make_team};

/// The user's club plays in `league1`; `league2` is another country's division with its own
/// fixture on the same date. The user's own fixture is already settled, as it is by the time
/// the live path finishes.
fn game_with_a_second_competition() -> Game {
    let mut game = make_game_with_match();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let mut user_league = game.league.clone().expect("the helper builds a league");
    user_league.participant_ids = vec!["team1".to_string(), "team2".to_string()];
    for fixture in &mut user_league.fixtures {
        fixture.status = FixtureStatus::Completed;
    }

    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    game.players.extend(make_squad("team3", "t3"));
    game.players.extend(make_squad("team4", "t4"));

    let mut other = League::new(
        "league2".to_string(),
        "Other League".to_string(),
        1,
        &["team3".to_string(), "team4".to_string()],
    );
    other.fixtures.push(Fixture {
        postponement: None,
        id: "fix2".to_string(),
        competition_id: "league2".to_string(),
        matchday: 1,
        date: today,
        home_team_id: "team3".to_string(),
        away_team_id: "team4".to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        result: None,
    });

    game.competitions = vec![user_league, other];
    game.league = Some(game.competitions[0].clone());
    game
}

#[test]
fn finishing_a_live_match_plays_the_other_competitions_due_today() {
    let mut game = game_with_a_second_competition();

    turn::finish_live_match_day(&mut game);

    let other = game
        .competitions
        .iter()
        .find(|competition| competition.id == "league2")
        .expect("the second competition survives the day");
    assert_eq!(
        other.fixtures[0].status,
        FixtureStatus::Completed,
        "a fixture due today in another competition must still be played when the user \
         watched their own match; the date has passed and it will never be due again"
    );
}

fn add_idle_call_up_clubs(game: &mut Game) {
    for (id, name, prefix) in [("team5", "Idle Home", "t5"), ("team6", "Idle Away", "t6")] {
        game.teams.push(make_team(id, name));
        game.players.extend(make_squad(id, prefix));
    }
}

fn national_squads(home_prefix: &str, away_prefix: &str) -> Vec<NationalTeam> {
    let mut england = NationalTeam::new("nt-eng".into(), "England".into(), "ENG".into(), None);
    england.squad_player_ids = vec![format!("{home_prefix}_fwd0"), format!("{home_prefix}_mid0")];
    let mut brazil = NationalTeam::new("nt-bra".into(), "Brazil".into(), "BRA".into(), None);
    brazil.squad_player_ids = vec![format!("{away_prefix}_fwd0"), format!("{away_prefix}_mid0")];
    vec![england, brazil]
}

/// Given a completed watched match and call-ups at idle clubs, When its live day ends,
/// Then the due national friendly still plays through the shared daily tail.
#[test]
fn finishing_a_live_match_plays_a_national_team_fixture_due_today() {
    let mut game = game_with_a_second_competition();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    add_idle_call_up_clubs(&mut game);
    game.national_teams = national_squads("t5", "t6");
    let england = &mut game.national_teams[0];
    england.fixtures.push(Fixture {
        postponement: None,
        id: "ntf-1".into(),
        competition_id: "international-friendlies".into(),
        matchday: 1,
        date: today,
        home_team_id: "nt-eng".into(),
        away_team_id: "nt-bra".into(),
        competition: FixtureCompetition::InternationalNation,
        status: FixtureStatus::Scheduled,
        result: None,
    });

    turn::finish_live_match_day(&mut game);

    let fixture = &game.national_teams[0].fixtures[0];
    assert_eq!(
        fixture.status,
        FixtureStatus::Completed,
        "an international due today must be played on a live-match day too"
    );
    assert!(fixture.result.is_some());
}

fn add_world_cup_final(game: &mut Game, today: &str) {
    let mut cup = League::new(
        "wc".to_string(),
        "World Cup".to_string(),
        u32::try_from(game.clock.current_date.year()).unwrap_or(2026),
        &["nt-eng".to_string(), "nt-bra".to_string()],
    );
    cup.kind = CompetitionType::InternationalNation;
    cup.scope = CompetitionScope::International;
    cup.rules.format = domain::league::CompetitionFormat::Knockout;
    cup.standings.clear();
    cup.fixtures.push(Fixture {
        postponement: None,
        id: "wc-final".to_string(),
        competition_id: "wc".to_string(),
        matchday: 1,
        date: today.to_string(),
        home_team_id: "nt-eng".to_string(),
        away_team_id: "nt-bra".to_string(),
        competition: FixtureCompetition::InternationalNation,
        status: FixtureStatus::Scheduled,
        result: None,
    });
    cup.knockout_rounds.push(KnockoutRoundState {
        id: "wc-round-1".to_string(),
        name: "Final".to_string(),
        fixture_ids: vec!["wc-final".to_string()],
        ..Default::default()
    });
    game.competitions.push(cup);
}

/// Given a completed watched match and call-ups at idle clubs, When its live day ends,
/// Then a due World Cup tie still plays and records its result.
#[test]
fn finishing_a_live_match_plays_a_world_cup_fixture_due_today() {
    let mut game = game_with_a_second_competition();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    add_idle_call_up_clubs(&mut game);
    game.national_teams = national_squads("t5", "t6");
    add_world_cup_final(&mut game, &today);
    turn::finish_live_match_day(&mut game);
    let cup = game
        .competitions
        .iter()
        .find(|competition| competition.id == "wc")
        .unwrap();
    assert_eq!(
        cup.fixtures[0].status,
        FixtureStatus::Completed,
        "a World Cup tie due today must be played on a live-match day too"
    );
    assert!(cup.fixtures[0].result.is_some());
}

/// Given call-ups whose clubs already played the watched match, When the live day ends,
/// Then their World Cup tie waits for rest, retains its identity and plays exactly once later.
#[test]
fn a_watched_club_match_defers_its_players_world_cup_tie() {
    let mut game = game_with_a_second_competition();
    let day = game.clock.current_date.date_naive();
    game.national_teams = national_squads("t1", "t2");
    add_world_cup_final(&mut game, &day.to_string());
    turn::finish_live_match_day(&mut game);
    let fixture = &game
        .competitions
        .iter()
        .find(|c| c.id == "wc")
        .unwrap()
        .fixtures[0];
    assert_eq!(fixture.status, FixtureStatus::Scheduled);
    assert!(fixture.result.is_none());
    let moved = chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").unwrap();
    assert!((moved - day).num_days() >= 3);
    assert_eq!(
        fixture.postponement.as_ref().unwrap().original_date,
        day.to_string()
    );
    assert_eq!(game.competitions[0].fixtures[0].date, day.to_string());
    assert_eq!(
        game.competitions[0].fixtures[0].status,
        FixtureStatus::Completed
    );
    game.clock.current_date = chrono::DateTime::from_naive_utc_and_offset(
        moved.and_hms_opt(12, 0, 0).unwrap(),
        chrono::Utc,
    );
    turn::finish_live_match_day(&mut game);
    let cup = game.competitions.iter().find(|c| c.id == "wc").unwrap();
    assert_eq!(cup.fixtures.len(), 1);
    assert_eq!(cup.fixtures[0].id, "wc-final");
    assert_eq!(cup.fixtures[0].date, moved.to_string());
    assert_eq!(cup.fixtures[0].status, FixtureStatus::Completed);
    assert!(cup.fixtures[0].result.is_some());
    let players = serde_json::to_value(&game.players).unwrap();
    assert_eq!(
        ofm_core::national_tournament::process_fixtures_due(&mut game, &moved.to_string()),
        0
    );
    assert_eq!(serde_json::to_value(&game.players).unwrap(), players);
}

#[test]
fn finishing_a_live_match_resolves_a_dormant_competition_due_today() {
    // The dormant tier was the one claimed omission with no test behind it: with no
    // `active_competition_ids` set, every competition counts as active, so
    // `dormant_competition_indices_due_today` selects nothing and the dormant loop could be
    // deleted with every other test still green. Scoping the world is what makes it bite.
    let mut game = game_with_a_second_competition();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    game.teams.push(make_team("dormant_home", "Dormant United"));
    game.teams.push(make_team("dormant_away", "Dormant Rovers"));

    let mut dormant = League::new(
        "dormant_league".to_string(),
        "Somewhere Else".to_string(),
        1,
        &["dormant_home".to_string(), "dormant_away".to_string()],
    );
    dormant.fixtures.push(Fixture {
        postponement: None,
        id: "dormant_fix".to_string(),
        competition_id: "dormant_league".to_string(),
        matchday: 1,
        date: today,
        home_team_id: "dormant_home".to_string(),
        away_team_id: "dormant_away".to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        result: None,
    });
    game.competitions.push(dormant);

    // Only the user's own competition is simulated in full; the rest is the dormant tier.
    game.active_competition_ids = vec!["league1".to_string()];

    let mut captured = 0usize;
    turn::finish_live_match_day_with_capture(&mut game, &mut |_| captured += 1);

    let dormant = game
        .competitions
        .iter()
        .find(|competition| competition.id == "dormant_league")
        .expect("the dormant competition survives the day");
    assert_eq!(
        dormant.fixtures[0].status,
        FixtureStatus::Completed,
        "a dormant competition's fixture due today must still be resolved by scoreline"
    );
    assert!(
        dormant.fixtures[0].result.is_some(),
        "the dormant fixture gets a scoreline"
    );
    assert_eq!(
        captured, 0,
        "a scoreline-only resolution runs no engine, so it captures no stats state"
    );
}

#[test]
fn the_sweeps_matches_hand_back_their_stats() {
    // `finish_live_match_day_with_capture` exists so the stats produced by the matches the sweep
    // plays are not dropped on the floor — the plain `finish_live_match_day` passes a no-op and
    // would lose them silently. Nothing asserted that the closure ever fires for a real match,
    // so the capture-preserving half of the API was untested.
    let mut game = game_with_a_second_competition();

    let mut captures = Vec::new();
    turn::finish_live_match_day_with_capture(&mut game, &mut |capture| captures.push(capture));

    assert!(
        !captures.is_empty(),
        "the other competition's fixture was played by the engine, so its stats must be handed \
         back to the caller rather than discarded"
    );
}
