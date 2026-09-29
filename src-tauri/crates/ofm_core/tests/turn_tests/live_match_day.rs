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

#[test]
fn finishing_a_live_match_plays_a_national_team_fixture_due_today() {
    let mut game = game_with_a_second_competition();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let mut england = NationalTeam::new("nt-eng".into(), "England".into(), "ENG".into(), None);
    england.squad_player_ids = vec!["t1_fwd0".into(), "t1_mid0".into(), "t1_def0".into()];
    england.fixtures.push(Fixture {
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
    let mut brazil = NationalTeam::new("nt-bra".into(), "Brazil".into(), "BRA".into(), None);
    brazil.squad_player_ids = vec!["t2_fwd0".into(), "t2_mid0".into()];
    game.national_teams = vec![england, brazil];

    turn::finish_live_match_day(&mut game);

    let fixture = &game.national_teams[0].fixtures[0];
    assert_eq!(
        fixture.status,
        FixtureStatus::Completed,
        "an international due today must be played on a live-match day too"
    );
    assert!(fixture.result.is_some());
}

#[test]
fn finishing_a_live_match_plays_a_world_cup_fixture_due_today() {
    let mut game = game_with_a_second_competition();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let mut england = NationalTeam::new("nt-eng".into(), "England".into(), "ENG".into(), None);
    england.squad_player_ids = vec!["t1_fwd0".into(), "t1_mid0".into()];
    let mut brazil = NationalTeam::new("nt-bra".into(), "Brazil".into(), "BR".into(), None);
    brazil.squad_player_ids = vec!["t2_fwd0".into()];
    game.national_teams = vec![england, brazil];

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
        id: "wc-final".to_string(),
        competition_id: "wc".to_string(),
        matchday: 1,
        date: today,
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

    turn::finish_live_match_day(&mut game);

    let cup = game
        .competitions
        .iter()
        .find(|competition| competition.id == "wc")
        .expect("the tournament survives the day");
    assert_eq!(
        cup.fixtures[0].status,
        FixtureStatus::Completed,
        "a World Cup tie due today must be played on a live-match day too"
    );
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
