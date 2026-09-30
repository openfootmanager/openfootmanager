//! Contracts as they stand on the day a career opens.
//!
//! World generation rolls a contract's *end* and leaves the start unwritten: it
//! cannot know the start, because a career's opening date is only settled when the
//! player picks a club and the clock moves to that club's season. So the start is
//! given here, once, at that point, by one rule.

use super::helpers::parse_contract_date;
use crate::game::Game;
use crate::world::team_season_anchor;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;

/// The day a contract nobody wrote a start for is taken to have begun.
///
/// A club's own season anchor, because that is when its agreements are struck, but
/// never later than `opening`: a contract cannot have started after the career did,
/// and a northern club's pre-season lands after the 1 July clock. With no anchor
/// the opening date stands in.
///
/// `None` when that date would not come before `contract_end`. An inverted or
/// zero-length interval would be a fabricated fact, and an honest unknown is better.
pub fn unauthored_contract_start(
    club_anchor: Option<NaiveDate>,
    opening: NaiveDate,
    contract_end: NaiveDate,
) -> Option<NaiveDate> {
    let start = club_anchor.map_or(opening, |anchor| anchor.min(opening));
    (start < contract_end).then_some(start)
}

/// Every club's season anchor, read from the game as it stands.
///
/// **Read this before the clock is moved.** Brazil's anchor is worked out from the
/// clock's year, so reading it after the hemisphere fix has pulled the clock back
/// lands a year too early. Pass the result to [`record_opening_contracts`]
/// afterwards.
pub fn club_season_anchors(game: &Game) -> HashMap<String, DateTime<Utc>> {
    game.teams
        .iter()
        .filter_map(|team| {
            team_season_anchor(game, &team.id).map(|anchor| (team.id.clone(), anchor))
        })
        .collect()
}

/// Record the contract every signed player opens the career on.
///
/// Run once, after the opening clock is final. Each contracted player gets one
/// `InitialContract` entry dated the opening day, carrying the contract generation
/// (or his package) gave him. A start that is already there is left exactly as it
/// is: an author may have written one, and may have written it after the opening
/// date for a deal that has not begun, which is theirs to say. A start nobody wrote
/// is given by [`unauthored_contract_start`], or left unknown when that cannot be
/// done honestly. Players with no club or no contract are skipped.
pub fn record_opening_contracts(game: &mut Game, club_anchors: &HashMap<String, DateTime<Utc>>) {
    let opening = game.clock.current_date.date_naive();
    let opening_date = opening.format("%Y-%m-%d").to_string();
    for player in &mut game.players {
        let Some(team_id) = player.team_id.as_deref() else {
            continue;
        };
        let start = match player.contract_start().map(str::to_string) {
            Some(authored) => Some(authored),
            None => player
                .contract_end()
                .and_then(parse_contract_date)
                .and_then(|end| {
                    // The contract is his parent club's while he is on loan, so that is
                    // the season it began in.
                    let contract_club = player.contract_club_id().unwrap_or(team_id);
                    let anchor = club_anchors
                        .get(contract_club)
                        .map(|anchor| anchor.date_naive());
                    unauthored_contract_start(anchor, opening, end)
                })
                .map(|start| start.format("%Y-%m-%d").to_string()),
        };
        if let Err(problem) = player.open_initial_contract(&opening_date, start) {
            // An authored contract that ends before it starts is refused by the
            // package check; if one gets here anyway it is left as generation wrote it.
            debug_assert!(false, "an opening contract was refused: {problem:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{club_season_anchors, record_opening_contracts, unauthored_contract_start};
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{NaiveDate, TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;

    fn day(year: i32, month: u32, date: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, date).unwrap()
    }

    // -- the rule ----------------------------------------------------------

    #[test]
    fn a_club_anchor_before_the_opening_date_is_the_start() {
        let start =
            unauthored_contract_start(Some(day(2025, 12, 15)), day(2026, 7, 1), day(2027, 6, 30));
        assert_eq!(start, Some(day(2025, 12, 15)));
    }

    #[test]
    fn an_anchor_after_the_opening_date_is_held_to_the_opening_date() {
        // A northern club's pre-season begins after the 1 July clock. A contract
        // cannot have started after the career did.
        let start =
            unauthored_contract_start(Some(day(2026, 7, 16)), day(2026, 7, 1), day(2027, 6, 30));
        assert_eq!(start, Some(day(2026, 7, 1)));
    }

    #[test]
    fn a_club_with_no_anchor_starts_on_the_opening_date() {
        let start = unauthored_contract_start(None, day(2026, 7, 1), day(2027, 6, 30));
        assert_eq!(start, Some(day(2026, 7, 1)));
    }

    #[test]
    fn a_start_that_would_not_precede_the_end_is_left_unknown() {
        // Ends on the opening day: a derived start would sit on top of it.
        assert_eq!(
            unauthored_contract_start(None, day(2026, 7, 1), day(2026, 7, 1)),
            None
        );
        // Already over before the club's season began.
        assert_eq!(
            unauthored_contract_start(Some(day(2025, 12, 15)), day(2026, 7, 1), day(2025, 6, 30)),
            None
        );
    }

    // -- fixtures ----------------------------------------------------------

    fn attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 50,
            stamina: 50,
            strength: 50,
            agility: 50,
            passing: 50,
            shooting: 50,
            tackling: 50,
            dribbling: 50,
            defending: 50,
            positioning: 50,
            vision: 50,
            decisions: 50,
            composure: 50,
            aggression: 50,
            teamwork: 50,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 50,
        }
    }

    fn club(id: &str, nation: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            format!("{id} FC"),
            id.to_uppercase(),
            nation.to_string(),
            "City".to_string(),
            "Ground".to_string(),
            10_000,
        );
        team.football_nation = nation.to_string();
        team
    }

    fn player_at(id: &str, team_id: Option<&str>, end: Option<&str>) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            format!("Player {id}"),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            attrs(),
        );
        player.team_id = team_id.map(str::to_string);
        player.stage_contract_end(end.map(str::to_string));
        player
    }

    fn league_opening_on(id: &str, club_id: &str, first_fixture: &str) -> League {
        let mut league = League::new(id.to_string(), id.to_string(), 2026, &[club_id.to_string()]);
        league.fixtures.push(Fixture {
            id: format!("{id}-f1"),
            competition_id: id.to_string(),
            matchday: 1,
            date: first_fixture.to_string(),
            home_team_id: club_id.to_string(),
            away_team_id: "opponent".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        });
        league
    }

    /// One Brazilian, one English and one Japanese club on the default 1 July clock.
    fn three_club_game(players: Vec<Player>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("en-1".to_string());

        let mut game = Game::new(
            clock,
            manager,
            vec![club("br-1", "BR"), club("en-1", "ENG"), club("jp-1", "JP")],
            players,
            vec![],
            vec![],
        );
        game.competitions = vec![
            league_opening_on("en-league", "en-1", "2026-08-15"),
            league_opening_on("jp-league", "jp-1", "2026-02-07"),
        ];
        game
    }

    fn start_of(game: &Game, id: &str) -> Option<String> {
        game.players
            .iter()
            .find(|player| player.id == id)
            .unwrap()
            .contract_start()
            .map(str::to_string)
    }

    // -- through the real anchor --------------------------------------------

    #[test]
    fn a_brazilian_club_starts_its_contracts_on_its_own_season_anchor() {
        let mut game = three_club_game(vec![player_at("br-p", Some("br-1"), Some("2027-06-30"))]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        // Brazil's season opens on 15 December of the year before the clock's.
        assert_eq!(start_of(&game, "br-p").as_deref(), Some("2025-12-15"));
    }

    #[test]
    fn a_loaned_player_takes_the_anchor_of_the_club_that_holds_the_contract() {
        // On loan, `team_id` is the borrower but the contract is the parent club's.
        // The two clubs have different anchors (Brazil 15 Dec, England clamped to the
        // 1 Jul opening), so a lookup by the wrong one cannot pass by coincidence.
        let mut loaned = player_at("loanee", Some("en-1"), Some("2027-06-30"));
        loaned.active_loan = Some(domain::player::ActiveLoan {
            parent_team_id: "br-1".to_string(),
            loan_team_id: "en-1".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        let mut game = three_club_game(vec![loaned]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        assert_eq!(
            start_of(&game, "loanee").as_deref(),
            Some("2025-12-15"),
            "started on the borrowing club's anchor instead of its parent's"
        );
    }

    #[test]
    fn a_european_club_is_held_to_the_opening_date() {
        let mut game = three_club_game(vec![player_at("en-p", Some("en-1"), Some("2027-06-30"))]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        // First fixture 15 Aug, less the 30-day pre-season, is 16 Jul: after the
        // 1 Jul clock, so the contract starts when the career does.
        assert_eq!(start_of(&game, "en-p").as_deref(), Some("2026-07-01"));
    }

    #[test]
    fn a_february_calendar_club_starts_on_its_preseason_anchor() {
        let mut game = three_club_game(vec![player_at("jp-p", Some("jp-1"), Some("2027-06-30"))]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        // 7 Feb first fixture less 30 days.
        assert_eq!(start_of(&game, "jp-p").as_deref(), Some("2026-01-08"));
    }

    #[test]
    fn an_authored_start_is_left_exactly_as_written() {
        // An author may write a start after the opening date: a deal that has not
        // begun yet. That is theirs to say, and the clamp is not applied to it.
        let mut signed = player_at("authored", Some("en-1"), Some("2031-06-30"));
        signed.stage_contract_start(Some("2030-07-01".to_string()));
        let mut game = three_club_game(vec![signed]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        assert_eq!(start_of(&game, "authored").as_deref(), Some("2030-07-01"));
    }

    #[test]
    fn people_without_a_club_or_an_end_date_are_not_given_a_start() {
        // The free agent keeps an end date so that having no club is the *only*
        // reason to skip them; with neither, the missing end would pass the test
        // on its own and the club check would be covered by nothing.
        let mut game = three_club_game(vec![
            player_at("free-agent", None, Some("2027-06-30")),
            player_at("no-end", Some("en-1"), None),
        ]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        assert_eq!(start_of(&game, "free-agent"), None);
        assert_eq!(start_of(&game, "no-end"), None);
    }

    #[test]
    fn a_contract_already_over_before_the_anchor_is_left_unknown() {
        // Ended in June 2025; the Brazilian club's season began in December 2025.
        let mut game = three_club_game(vec![player_at("lapsed", Some("br-1"), Some("2025-06-30"))]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        assert_eq!(start_of(&game, "lapsed"), None);
    }

    #[test]
    fn no_start_lands_after_the_date_the_career_opens() {
        // The order `select_team` runs in: read every club's anchor on the default
        // clock, move the clock back to the chosen club's own anchor, then stamp.
        // Anchors have to be read first because Brazil's is worked out from the
        // clock's year, so reading it after the move lands a year too early.
        let mut game = three_club_game(vec![
            player_at("br-p", Some("br-1"), Some("2027-06-30")),
            player_at("en-p", Some("en-1"), Some("2027-06-30")),
            player_at("jp-p", Some("jp-1"), Some("2027-06-30")),
        ]);
        let anchors = club_season_anchors(&game);

        let chosen = Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap();
        game.clock.current_date = chosen;
        game.clock.start_date = chosen;
        record_opening_contracts(&mut game, &anchors);

        let opening = game.clock.current_date.date_naive().to_string();
        for id in ["br-p", "en-p", "jp-p"] {
            let start = start_of(&game, id).expect("every contract here has a knowable start");
            assert!(
                start <= opening,
                "{id} starts {start}, after the career opens on {opening}"
            );
        }
        assert_eq!(start_of(&game, "br-p").as_deref(), Some("2025-12-15"));
    }

    // -- the ledger ---------------------------------------------------------

    fn initial_entries_of(game: &Game, id: &str) -> Vec<domain::player::PlayerMovementEntry> {
        game.players
            .iter()
            .find(|player| player.id == id)
            .unwrap()
            .movement_history
            .iter()
            .filter(|entry| entry.kind == domain::player::PlayerMovementKind::InitialContract)
            .cloned()
            .collect()
    }

    #[test]
    fn opening_a_career_records_one_initial_contract_per_contracted_player() {
        let mut signed = player_at("en-p", Some("en-1"), Some("2027-06-30"));
        signed.stage_wage(8_000);
        let mut game = three_club_game(vec![signed]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        let entries = initial_entries_of(&game, "en-p");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].to_team_id.as_deref(), Some("en-1"));
        assert_eq!(
            entries[0].date, "2026-07-01",
            "recorded on the day the career opens"
        );
        let record = entries[0]
            .contract
            .as_ref()
            .expect("the entry carries the contract");
        assert_eq!(
            record.source,
            domain::contract_ledger::ContractSource::Initial
        );
        assert_eq!(record.start.as_deref(), Some("2026-07-01"));
        assert_eq!(record.end.as_deref(), Some("2027-06-30"));
        assert_eq!(record.weekly_wage, 8_000);
    }

    #[test]
    fn an_opening_start_that_is_unknown_stays_unknown_in_the_entry() {
        // Already over before the Brazilian club's season began, so no start is derived.
        let mut lapsed = player_at("lapsed", Some("br-1"), Some("2025-06-30"));
        lapsed.stage_wage(5_000);
        let mut game = three_club_game(vec![lapsed]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        let entries = initial_entries_of(&game, "lapsed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].contract.as_ref().unwrap().start, None);
    }

    #[test]
    fn an_authored_contract_is_recorded_exactly_as_authored() {
        let mut signed = player_at("authored", Some("en-1"), Some("2031-06-30"));
        signed.stage_contract_start(Some("2030-07-01".to_string()));
        signed.stage_wage(12_345);
        let mut game = three_club_game(vec![signed]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        let record = initial_entries_of(&game, "authored")[0]
            .contract
            .clone()
            .unwrap();
        assert_eq!(record.start.as_deref(), Some("2030-07-01"));
        assert_eq!(record.end.as_deref(), Some("2031-06-30"));
        assert_eq!(record.weekly_wage, 12_345);
    }

    #[test]
    fn a_loaned_player_is_recorded_against_the_club_that_holds_his_contract() {
        let mut loaned = player_at("loanee", Some("en-1"), Some("2027-06-30"));
        loaned.stage_wage(9_000);
        loaned.active_loan = Some(domain::player::ActiveLoan {
            parent_team_id: "br-1".to_string(),
            loan_team_id: "en-1".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 50,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        let mut game = three_club_game(vec![loaned]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        assert_eq!(
            initial_entries_of(&game, "loanee")[0].to_team_id.as_deref(),
            Some("br-1")
        );
    }

    #[test]
    fn players_with_no_club_or_no_contract_get_no_initial_entry() {
        let mut free_agent = player_at("free-agent", None, Some("2027-06-30"));
        free_agent.stage_wage(4_000);
        let mut game = three_club_game(vec![
            free_agent,
            player_at("no-contract", Some("en-1"), None),
        ]);

        let anchors = club_season_anchors(&game);
        record_opening_contracts(&mut game, &anchors);

        assert!(initial_entries_of(&game, "free-agent").is_empty());
        assert!(initial_entries_of(&game, "no-contract").is_empty());
    }

    #[test]
    fn opening_twice_does_not_record_a_second_initial_contract() {
        let mut signed = player_at("en-p", Some("en-1"), Some("2027-06-30"));
        signed.stage_wage(8_000);
        let mut game = three_club_game(vec![signed]);
        let anchors = club_season_anchors(&game);

        record_opening_contracts(&mut game, &anchors);
        record_opening_contracts(&mut game, &anchors);

        assert_eq!(initial_entries_of(&game, "en-p").len(), 1);
    }
}
