//! Story: choosing a club starts a career the same way, whichever door the player
//! came through.
//!
//! The app and an agent playing over MCP both choose a club, and both have to
//! arrive at the same world: the clock on the day that club's season opens, every
//! contract dated against that day, the simulation following the club, the manager
//! in charge. Each scenario below is one route through that, written
//! Given / When / Then, and is named after what it guarantees.

use domain::stats::StatsState;

use super::{CareerScope, begin_career, date_opening_contracts};
use crate::clock::GameClock;
use crate::game::Game;
use crate::world::test_fixtures::{nation_team, player_at, unemployed_manager};
use crate::world::{ensure_multi_competition_foundations, infer_region_id, start_date_for_year};

/// A world that exists but has no manager in any club: two English clubs and two
/// Brazilian ones, three players each under contract until 30 June 2033. It opens
/// on 1 July 2032, which is before Brazil's season and after England's.
fn world_awaiting_a_manager() -> Game {
    let clock = GameClock::new(start_date_for_year(2032).expect("a valid start year"));
    let teams = vec![
        nation_team("eng-a", "ENG", 500),
        nation_team("eng-b", "ENG", 400),
        nation_team("br-a", "BR", 500),
        nation_team("br-b", "BR", 400),
    ];
    let players = teams
        .iter()
        .flat_map(|team| {
            (0..3).map(move |index| {
                player_at(&format!("{}-p{index}", team.id), &team.id, "2033-06-30")
            })
        })
        .collect();
    Game::new(clock, unemployed_manager(), teams, players, vec![], vec![])
}

fn begin(game: &mut Game, club: &str) -> Result<StatsState, String> {
    begin_career(game, club, CareerScope::default(), StatsState::default())
}

/// Everyone under contract with a club has been given a start, and none of those
/// starts falls after the day the career opens on. (Foundations also mint
/// national-team fillers, who belong to no club and rightly have neither.)
fn assert_contracts_started_by_the_opening_date(game: &Game) {
    let opening = game.clock.current_date.date_naive().to_string();
    for player in game
        .players
        .iter()
        .filter(|player| player.team_id.is_some() && player.contract_end().is_some())
    {
        let start = player
            .contract_start()
            .unwrap_or_else(|| panic!("{} was given no contract start", player.id));
        assert!(
            start <= opening.as_str(),
            "{} starts {start}, after the career opens on {opening}",
            player.id
        );
    }
}

fn competitions_only_of(game: &Game, country_prefix: &str) -> Vec<String> {
    game.competitions
        .iter()
        .filter(|competition| {
            !competition.participant_ids.is_empty()
                && competition
                    .participant_ids
                    .iter()
                    .all(|club| club.starts_with(country_prefix))
        })
        .map(|competition| competition.id.clone())
        .collect()
}

// ── Scenario: the clock ─────────────────────────────────────────────────────

/// Given a club whose season starts after the game's clock,
/// When the career begins,
/// Then the clock is left where it was and every contract starts on or before it.
#[test]
fn an_english_career_opens_on_the_clock_it_was_given() {
    let mut game = world_awaiting_a_manager();
    let opened = game.clock.current_date;

    begin(&mut game, "eng-a").expect("the career begins");

    assert_eq!(game.clock.current_date, opened);
    assert_eq!(game.clock.start_date, opened);
    assert_contracts_started_by_the_opening_date(&game);
}

/// Given a Brazilian club, whose season began before the game's clock,
/// When the career begins,
/// Then the clock is set back to the day that club's season opens, and the
/// contracts are dated against that day rather than the one the world was built on.
#[test]
fn a_brazilian_career_opens_on_the_start_of_its_clubs_season() {
    let mut game = world_awaiting_a_manager();

    begin(&mut game, "br-a").expect("the career begins");

    assert_eq!(
        game.clock.current_date.date_naive().to_string(),
        "2031-12-15"
    );
    assert_eq!(game.clock.start_date, game.clock.current_date);
    assert_contracts_started_by_the_opening_date(&game);
}

/// Given a club whose season began before the clock, and a career that opens
/// mid-season rather than at the start,
/// When the career begins,
/// Then the clock is not pulled back to the start of the club's season.
#[test]
fn a_career_that_opens_mid_season_is_not_pulled_back_to_its_clubs_season_start() {
    let mut game = world_awaiting_a_manager();
    let opened = game.clock.start_date;
    game.clock.advance_days(120);
    let midseason = game.clock.current_date;

    begin(&mut game, "br-a").expect("the career begins");

    assert_eq!(game.clock.current_date, midseason, "the clock was moved");
    assert_eq!(
        game.clock.start_date, opened,
        "the game's start date was moved"
    );
}

/// #650's wiring test, moved with the code it guards and unchanged in what it asks.
///
/// A Brazilian career opens on 15 December of the year *before* the clock's, so
/// aligning the clock to that club moves it back by half a year. Every contract
/// has to be dated against the date the career actually opens on, and each club's
/// anchor has to have been read before the move: Brazil's is worked out from the
/// clock's year, so reading it afterwards lands a year too early.
#[test]
fn opening_a_brazilian_career_dates_no_contract_after_the_opening_date() {
    let mut game = world_awaiting_a_manager();
    ensure_multi_competition_foundations(&mut game);

    date_opening_contracts(&mut game, Some("br-a"));

    let opening = game.clock.current_date.date_naive();
    assert_eq!(
        opening.to_string(),
        "2031-12-15",
        "the clock should have been aligned to the Brazilian club's season"
    );
    let brazilian_start = game
        .players
        .iter()
        .find(|player| player.id == "br-a-p0")
        .and_then(|player| player.contract_start().map(str::to_string));
    assert_eq!(
        brazilian_start.as_deref(),
        Some("2031-12-15"),
        "the anchor was read after the clock moved, a year too early"
    );
    // The ledger says the same thing the start does: one initial contract, dated the
    // opening day, starting on his club's anchor.
    let brazilian = game.players.iter().find(|p| p.id == "br-a-p0").unwrap();
    let initial: Vec<_> = brazilian
        .movement_history
        .iter()
        .filter(|entry| entry.kind == domain::player::PlayerMovementKind::InitialContract)
        .collect();
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].date, "2031-12-15");
    assert_eq!(
        initial[0]
            .contract
            .as_ref()
            .and_then(|c| c.start.as_deref()),
        Some("2031-12-15")
    );
    assert_contracts_started_by_the_opening_date(&game);
}

// ── Scenario: the manager ───────────────────────────────────────────────────

/// Given a club with its own manager,
/// When the player chooses it,
/// Then the player is in charge of that club and nobody else's.
#[test]
fn the_player_takes_charge_of_the_chosen_club_and_of_no_other() {
    let mut game = world_awaiting_a_manager();

    begin(&mut game, "eng-a").expect("the career begins");

    let user = game.manager.id.clone();
    assert_eq!(game.manager.team_id.as_deref(), Some("eng-a"));
    let club = |id: &str| game.teams.iter().find(|team| team.id == id).expect(id);
    assert_eq!(club("eng-a").manager_id.as_deref(), Some(user.as_str()));
    for rival in ["eng-b", "br-a", "br-b"] {
        assert_ne!(
            club(rival).manager_id.as_deref(),
            Some(user.as_str()),
            "{rival} must keep its own manager"
        );
    }
    assert!(
        game.messages
            .iter()
            .any(|message| message.id == "job_welcome_eng-a_2032-07-01"),
        "the board greets the new manager in the inbox"
    );
}

// ── Scenario: the simulation follows the club ───────────────────────────────

/// Given clubs in two countries,
/// When the player chooses an English one and asks for nothing in particular,
/// Then England is simulated in full and Brazil's own domestic competitions are not.
///
/// Brazil's *region* is on all the same: the continental cup both countries
/// qualify for requires it, and `resolve_simulation_scope` switches on whatever a
/// competition in scope requires. What stays dormant is what only Brazil plays in.
#[test]
fn the_simulation_scope_follows_the_chosen_club() {
    let mut game = world_awaiting_a_manager();

    begin(&mut game, "eng-a").expect("the career begins");

    assert!(game.active_region_ids.contains(&infer_region_id("ENG")));
    let brazilian = competitions_only_of(&game, "br-");
    assert!(
        !brazilian.is_empty(),
        "the world has Brazilian competitions"
    );
    for id in brazilian {
        assert!(
            !game.active_competition_ids.contains(&id),
            "{id} should not be simulated in full"
        );
    }
    let english = competitions_only_of(&game, "eng-");
    assert!(!english.is_empty(), "the world has English competitions");
    for id in english {
        assert!(
            game.active_competition_ids.contains(&id),
            "{id} should be simulated in full"
        );
    }
}

/// Given the player asks for a region to be simulated that nothing else switches on,
/// When they choose an English club,
/// Then that region is simulated as well as the club's own.
#[test]
fn a_region_the_player_asked_for_is_kept_alongside_the_clubs_own() {
    // A region with no club in this world. Any region that has clubs is already on:
    // the continental cup they all qualify for requires it, so asking for one
    // could not show whether the request was honoured.
    let asked_for = infer_region_id("NG");

    // The world has to be able to tell the two outcomes apart: without the
    // request, the region is off.
    let mut without = world_awaiting_a_manager();
    begin(&mut without, "eng-a").expect("the career begins");
    assert!(
        !without.active_region_ids.contains(&asked_for),
        "{asked_for} is on anyway, so this scenario would prove nothing"
    );

    let mut game = world_awaiting_a_manager();
    let scope = CareerScope {
        regions: Some(vec![asked_for.clone()]),
        competitions: None,
    };
    begin_career(&mut game, "eng-a", scope, StatsState::default()).expect("the career begins");

    assert!(game.active_region_ids.contains(&asked_for));
    assert!(game.active_region_ids.contains(&infer_region_id("ENG")));
}

// ── Scenario: the players ───────────────────────────────────────────────────

/// Given players who start with the generic positions a generated world gives them,
/// When the career begins,
/// Then every outfield position is granular straight away, rather than only after
/// the first save and reload.
#[test]
fn every_position_is_granular_as_soon_as_the_career_starts() {
    let mut game = world_awaiting_a_manager();
    assert!(
        game.players
            .iter()
            .all(|player| player.natural_position.is_legacy_bucket()),
        "the players start with generic positions"
    );

    begin(&mut game, "eng-a").expect("the career begins");

    for player in game
        .players
        .iter()
        .filter(|player| player.team_id.is_some())
    {
        assert!(
            !player.natural_position.is_legacy_bucket(),
            "{} still has a generic position",
            player.id
        );
    }
}

// ── Scenario: a club that is not there ──────────────────────────────────────

/// Given a club that does not exist,
/// When the player chooses it,
/// Then the career is refused with the translation key for it, and nothing in the
/// game has changed — not the clock, not the competitions, not the contracts.
#[test]
fn choosing_a_club_that_does_not_exist_is_refused_and_changes_nothing() {
    let mut game = world_awaiting_a_manager();
    let before = serde_json::to_string(&game).expect("the game serialises");

    let refusal = begin(&mut game, "nowhere-fc").expect_err("there is no such club");

    assert_eq!(refusal, "be.error.teamNotFound");
    assert_eq!(
        serde_json::to_string(&game).expect("the game serialises"),
        before,
        "a refused career must leave the game exactly as it was"
    );
}
