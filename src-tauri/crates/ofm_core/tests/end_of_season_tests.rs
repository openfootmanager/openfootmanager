#[path = "end_of_season_tests/advance_sequence.rs"]
mod advance_sequence;
#[path = "end_of_season_tests/continental_qualification.rs"]
mod continental_qualification;
#[path = "end_of_season_tests/player_statistics.rs"]
mod player_statistics;
#[path = "end_of_season_tests/prizes_and_messages.rs"]
mod prizes_and_messages;
#[path = "end_of_season_tests/promotion_notices.rs"]
mod promotion_notices;
#[path = "end_of_season_tests/retirees.rs"]
mod retirees;
#[path = "end_of_season_tests/season_story.rs"]
mod season_story;

use chrono::{TimeZone, Utc};
use domain::competition_calendar::CalendarMetadata;
use domain::league::{
    Berth, BerthRule, CompetitionFormat, CompetitionScope, CompetitionType, Fixture,
    FixtureCompetition, FixtureStatus, KnockoutRoundState, League, MatchResult, StandingEntry,
};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, PlayerSeasonStats, Position};
use domain::team::{FinancialTransactionKind, Team};
use ofm_core::clock::GameClock;
use ofm_core::end_of_season::{
    advance_to_next_season, berth_qualified_entrants, continental_qualified_entrants,
    expected_fixture_count, is_season_complete, process_end_of_season, resolve_continental_fields,
};
use ofm_core::game::{BoardObjective, Game, ObjectiveType};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_team(id: &str, name: &str) -> Team {
    Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        40_000,
    )
}

fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let attrs = PlayerAttributes {
        pace: 65,
        stamina: 65,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: 65,
        tackling: 65,
        dribbling: 65,
        defending: 65,
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 50,
        teamwork: 65,
        leadership: 50,
        handling: 20,
        reflexes: 30,
        aerial: 60,
    };
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        "1995-01-01".to_string(),
        "GB".to_string(),
        pos,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 90;
    p
}

fn make_completed_fixture(id: &str, home: &str, away: &str, hg: u8, ag: u8) -> Fixture {
    Fixture {
        id: id.to_string(),
        matchday: 1,
        date: "2025-06-01".to_string(),
        home_team_id: home.to_string(),
        away_team_id: away.to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Completed,
        result: Some(MatchResult {
            home_goals: hg,
            away_goals: ag,
            home_scorers: vec![],
            away_scorers: vec![],
            report: None,
            home_penalties: None,
            away_penalties: None,
        }),
        ..Default::default()
    }
}

fn make_standing(
    team_id: &str,
    won: u32,
    drawn: u32,
    lost: u32,
    gf: u32,
    ga: u32,
) -> StandingEntry {
    StandingEntry {
        team_id: team_id.to_string(),
        played: won + drawn + lost,
        won,
        drawn,
        lost,
        goals_for: gf,
        goals_against: ga,
        points: won * 3 + drawn,
    }
}

/// Build a game with a completed season (2 teams, all fixtures done).
fn make_completed_season_game() -> Game {
    let date = Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());
    manager.satisfaction = 60;

    let team1 = make_team("team1", "Test FC");
    let team2 = make_team("team2", "Rival FC");

    let mut p1 = make_player("p1", "Star", "team1", Position::Forward);
    p1.stats = PlayerSeasonStats {
        appearances: 30,
        goals: 20,
        assists: 10,
        clean_sheets: 0,
        avg_rating: 7.5,
        rated_appearances: 30,
        minutes_played: 2700,
        yellow_cards: 3,
        red_cards: 0,
        ..PlayerSeasonStats::default()
    };
    p1.league_stats_by_competition
        .insert("league1".into(), p1.stats.clone());

    let mut p2 = make_player("p2", "Rival", "team2", Position::Forward);
    p2.stats = PlayerSeasonStats {
        appearances: 28,
        goals: 15,
        assists: 8,
        clean_sheets: 0,
        avg_rating: 7.0,
        rated_appearances: 28,
        minutes_played: 2500,
        yellow_cards: 1,
        red_cards: 0,
        ..PlayerSeasonStats::default()
    };
    p2.league_stats_by_competition
        .insert("league1".into(), p2.stats.clone());

    let fixtures = vec![
        make_completed_fixture("f1", "team1", "team2", 2, 1),
        make_completed_fixture("f2", "team2", "team1", 0, 1),
    ];

    // team1 won both: 6 pts, team2 lost both: 0 pts
    let standings = vec![
        make_standing("team1", 2, 0, 0, 3, 1),
        make_standing("team2", 0, 0, 2, 1, 3),
    ];

    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures,
        standings,
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };

    let mut game = Game::new(
        clock,
        manager,
        vec![team1, team2],
        vec![p1, p2],
        vec![],
        vec![],
    );
    game.league = Some(league);
    game
}

fn make_club(id: &str, nation: &str, reputation: u32) -> Team {
    let mut team = make_team(id, id);
    team.football_nation = nation.to_string();
    team.reputation = reputation;
    team
}

/// A single-division domestic league with `clubs` listed strongest-first; the
/// final standings are set so the listed order is the finishing order.
fn first_division(id: &str, country: &str, region: &str, clubs: &[&str]) -> League {
    let total = clubs.len() as u32;
    let standings = clubs
        .iter()
        .enumerate()
        .map(|(rank, club)| {
            let won = total - rank as u32; // strictly decreasing → unique order
            make_standing(club, won, 0, total - won, won * 2, 0)
        })
        .collect();
    League {
        id: id.to_string(),
        name: id.to_string(),
        season: 1,
        kind: CompetitionType::League,
        scope: CompetitionScope::Domestic,
        country_id: Some(country.to_string()),
        region_id: Some(region.to_string()),
        participant_ids: clubs.iter().map(|c| c.to_string()).collect(),
        standings,
        priority: 0,
        ..Default::default()
    }
}

/// A domestic knockout cup whose final has been won by `winner`.
fn domestic_cup(id: &str, country: &str, region: &str, winner: &str, loser: &str) -> League {
    let final_id = format!("{id}-final");
    let mut final_fixture = make_completed_fixture(&final_id, winner, loser, 2, 1);
    final_fixture.competition = FixtureCompetition::Cup;
    League {
        id: id.to_string(),
        name: id.to_string(),
        season: 1,
        kind: CompetitionType::Cup,
        scope: CompetitionScope::Domestic,
        country_id: Some(country.to_string()),
        region_id: Some(region.to_string()),
        fixtures: vec![final_fixture],
        knockout_rounds: vec![KnockoutRoundState {
            id: format!("{id}-final-round"),
            name: "Final".to_string(),
            fixture_ids: vec![final_id],
            bye_team_ids: vec![],
            completed: true,
        }],
        ..Default::default()
    }
}

/// A continental club competition fed by `region`, with a `field_size`-club
/// field (the placeholder entrants are replaced at qualification time).
fn continental_cup(id: &str, region: &str, field_size: usize) -> League {
    League {
        id: id.to_string(),
        name: id.to_string(),
        season: 1,
        kind: CompetitionType::ContinentalClub,
        scope: CompetitionScope::Continental,
        required_region_ids: vec![region.to_string()],
        participant_ids: (0..field_size).map(|i| format!("seed-{i}")).collect(),
        ..Default::default()
    }
}

/// Given a completed national friendly before a neutral-season rollover,
/// When the new calendar is scheduled, then the old result stays intact and five future windows
/// are added exactly once.
#[test]
fn process_end_of_season_keeps_history_and_adds_future_national_windows() {
    use domain::national_team::NationalTeam;

    let mut game = make_completed_season_game();
    // A neutral season so the windows host friendlies: 2027/28 neither stages
    // a World Cup nor hosts any part of the two-season qualifying campaign.
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 20, 12, 0, 0).unwrap());

    let mut nt_a = NationalTeam::new("nt-a".into(), "A".into(), "AAA".into(), None);
    nt_a.squad_player_ids = vec!["p1".into()];
    nt_a.fixtures.push(Fixture {
        postponement: None,
        id: "old-friendly".into(),
        competition_id: "international-friendlies".into(),
        matchday: 1,
        date: "2026-09-09".into(),
        home_team_id: "nt-a".into(),
        away_team_id: "nt-b".into(),
        competition: FixtureCompetition::InternationalNation,
        status: FixtureStatus::Completed,
        result: Some(MatchResult {
            home_goals: 3,
            away_goals: 1,
            ..Default::default()
        }),
    });
    let mut nt_b = NationalTeam::new("nt-b".into(), "B".into(), "BBB".into(), None);
    nt_b.squad_player_ids = vec!["p2".into()];
    game.national_teams = vec![nt_a, nt_b];

    process_end_of_season(&mut game);

    let fixtures: Vec<&Fixture> = game
        .national_teams
        .iter()
        .flat_map(|team| team.fixtures.iter())
        .collect();

    assert_eq!(
        fixtures.len(),
        6,
        "one historical match and five future windows"
    );
    let old: Vec<_> = fixtures
        .iter()
        .filter(|fixture| fixture.id == "old-friendly")
        .collect();
    assert_eq!(old.len(), 1);
    assert_eq!(old[0].status, FixtureStatus::Completed);
    assert_eq!(old[0].date, "2026-09-09");
    let result = old[0].result.as_ref().unwrap();
    assert_eq!((result.home_goals, result.away_goals), (3, 1));
    let future: Vec<_> = fixtures
        .iter()
        .filter(|fixture| fixture.id != "old-friendly")
        .collect();
    assert_eq!(future.len(), 5);
    assert!(
        future
            .iter()
            .all(|fixture| fixture.status == FixtureStatus::Scheduled && fixture.result.is_none())
    );
    assert!(
        future
            .iter()
            .all(|fixture| fixture.date.as_str() > "2027-06-01")
    );
}

#[test]
fn process_end_of_season_records_history_and_prizes_for_every_division() {
    fn prize_money_total(team: &domain::team::Team) -> i64 {
        team.financial_ledger
            .iter()
            .filter(|t| matches!(t.kind, domain::team::FinancialTransactionKind::PrizeMoney))
            .map(|t| t.amount)
            .sum()
    }

    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    let team3_initial_reputation = game
        .teams
        .iter()
        .find(|t| t.id == "team3")
        .unwrap()
        .reputation;

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0),
            make_standing("team2", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    let team3 = game.teams.iter().find(|t| t.id == "team3").unwrap();

    // Both divisions record season history with within-division positions.
    let team3_record = team3
        .history
        .last()
        .expect("second-division history recorded");
    assert_eq!(team3_record.league_position, 1);
    assert_eq!(team3_record.season, 1);
    // Each record names the division it was won in, so a second-division win is not a title.
    assert_eq!(team3_record.competition_id, "eng-2");
    assert_eq!(team1.history.last().unwrap().competition_id, "eng-1");

    // The second-division champion earns half the top-flight champion's prize.
    assert!(prize_money_total(team1) > 0);
    assert_eq!(prize_money_total(team3) * 2, prize_money_total(team1));

    // Winning its division improves the club's reputation.
    assert!(team3.reputation > team3_initial_reputation);
}

/// Two-division game: div1 = team1 (champion), team2 (relegated);
/// div2 = team3 (promoted), team4. The manager runs `user_team`.
fn make_two_division_game(user_team: &str) -> Game {
    let mut game = make_completed_season_game();
    for player in &mut game.players {
        let stats = player
            .league_stats_by_competition
            .remove("league1")
            .unwrap();
        player
            .league_stats_by_competition
            .insert("eng-1".into(), stats);
    }
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    game.manager.team_id = Some(user_team.to_string());

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0),
            make_standing("team2", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];
    game
}

#[test]
fn relegated_user_receives_a_relegation_message() {
    let mut game = make_two_division_game("team2");

    process_end_of_season(&mut game);

    // Named by the edition the user's new division was renewed to, not by last season plus one.
    let renewed_season = ofm_core::end_of_season::club_league_table(&game, "team2")
        .expect("the user still has a division")
        .season;
    let msg = game
        .messages
        .iter()
        .find(|m| m.id == format!("relegation_{renewed_season}"))
        .expect("relegated user must be told");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.relegation.subject")
    );
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.relegation.body"));
    assert_eq!(
        msg.i18n_params.get("division"),
        Some(&"ENG Second Division".to_string())
    );
    assert!(
        !game.messages.iter().any(|m| m.id.starts_with("promotion_")),
        "a relegated user must not also be congratulated"
    );
}

#[test]
fn promoted_user_receives_a_promotion_message() {
    let mut game = make_two_division_game("team3");

    process_end_of_season(&mut game);

    let renewed_season = ofm_core::end_of_season::club_league_table(&game, "team3")
        .expect("the user still has a division")
        .season;
    let msg = game
        .messages
        .iter()
        .find(|m| m.id == format!("promotion_{renewed_season}"))
        .expect("promoted user must be told");
    assert_eq!(msg.subject_key.as_deref(), Some("be.msg.promotion.subject"));
    assert_eq!(
        msg.i18n_params.get("division"),
        Some(&"ENG First Division".to_string())
    );
}

/// The season preview article the rollover published, and the clubs it names.
fn season_preview(game: &Game) -> (String, Vec<String>, String, String) {
    let preview = game
        .news
        .iter()
        .find(|article| article.category == domain::news::NewsCategory::SeasonPreview)
        .expect("the rollover publishes a season preview");
    let param = |name: &str| preview.i18n_params.get(name).cloned().unwrap_or_default();
    let mut listed: Vec<String> = param("teamList").split(", ").map(str::to_string).collect();
    listed.sort();
    (
        param("teamCount"),
        listed,
        param("favourite"),
        param("darkHorse"),
    )
}

/// Given a two-division world where the user's club stays in the first division and the second
/// division's winner comes up; when the season rolls over; then the preview covers the user's
/// division as it will be next season, not every club in the world (#1179).
#[test]
fn the_season_preview_covers_the_users_division_only() {
    let mut game = make_two_division_game("team1");

    process_end_of_season(&mut game);

    let (count, listed, favourite, dark_horse) = season_preview(&game);
    assert_eq!(count, "2");
    assert_eq!(listed, ["Test FC", "Third FC"]);
    assert!(
        listed.contains(&favourite),
        "favourite {favourite} is not in the division"
    );
    assert!(
        listed.contains(&dark_horse),
        "dark horse {dark_horse} is not in the division"
    );
}

/// Given the user's club relegated to the second division; when the season rolls over; then the
/// preview covers the division the club now plays in.
#[test]
fn a_relegated_users_season_preview_covers_the_division_they_drop_into() {
    let mut game = make_two_division_game("team2");

    process_end_of_season(&mut game);

    let (count, listed, _, _) = season_preview(&game);
    assert_eq!(count, "2");
    assert_eq!(listed, ["Fourth FC", "Rival FC"]);
}

/// Given a world whose user's club plays in no league table, and no legacy league to fall back
/// on; when the season rolls over; then no preview is published: the rollover has no division to
/// summarise and stops before it, so there is never one of "0 teams".
#[test]
fn a_user_with_no_division_gets_no_season_preview() {
    let mut game = make_two_division_game("team1");
    game.teams.push(make_team("team9", "Tableless FC"));
    game.manager.team_id = Some("team9".to_string());
    game.league = None;

    process_end_of_season(&mut game);

    assert!(
        game.news
            .iter()
            .all(|article| article.category != domain::news::NewsCategory::SeasonPreview)
    );
}

/// The two-division world with the user's manager sacked from `last_club`: his stint there opened
/// and closed as the game records it, and the `game.league` mirror put back on the first division,
/// the world's first competition. The game now syncs a sacked manager's mirror to his last club's
/// league (#1296), but the end-of-season screens must follow that club by themselves, not through the
/// mirror (a mirror that is stale, as the turn loop leaves it between syncs, must not change them).
fn sacked_from(last_club: &str) -> Game {
    let mut game = make_two_division_game(last_club);
    game.manager
        .career_history
        .push(domain::manager::ManagerCareerEntry::open(
            last_club.to_string(),
            last_club.to_string(),
            "2024-07-01".to_string(),
        ));
    game.manager.fire("2025-03-01");
    game.sync_legacy_league();
    game.league = game.competitions.first().cloned();
    game
}

/// Given a manager sacked from Fourth FC, a second-division club, and the mirror on the first
///      division (the world's first competition)
/// When the season rolls over
/// Then the preview covers the second division as it will be, Fourth FC's league, not the first
///      division the mirror fell back to (#1270)
#[test]
fn a_sacked_managers_season_preview_covers_his_last_clubs_league() {
    let mut game = sacked_from("team4");

    process_end_of_season(&mut game);

    let (count, listed, _, _) = season_preview(&game);
    assert_eq!(count, "2");
    assert_eq!(listed, ["Fourth FC", "Rival FC"]);
}

/// Given a manager sacked from Fourth FC, with the mirror on the first division
/// When the season rolls over
/// Then the season summary is of the second division, his last club's
#[test]
fn a_sacked_managers_season_summary_is_of_his_last_clubs_league() {
    let mut game = sacked_from("team4");

    let summary = process_end_of_season(&mut game);

    assert_eq!(summary.league_name, "ENG Second Division");
}

/// Given a manager sacked from Fourth FC, with the mirror on the first division, and a
///      first-division striker who outscores the second division
/// When the season's awards are read for the awards screen
/// Then the golden boot is the second division's, his last club's league
#[test]
fn a_sacked_managers_awards_are_his_last_clubs_league() {
    let mut game = sacked_from("team4");
    let mut lower = make_player("p4", "Lower Star", "team3", Position::Forward);
    lower.stats = PlayerSeasonStats {
        appearances: 10,
        goals: 9,
        avg_rating: 7.5,
        rated_appearances: 10,
        minutes_played: 900,
        ..PlayerSeasonStats::default()
    };
    lower
        .league_stats_by_competition
        .insert("eng-2".into(), lower.stats.clone());
    game.players.push(lower);

    let awards = ofm_core::season_awards::user_division_season_awards(&game);

    let top_scorer = awards
        .golden_boot
        .first()
        .map(|winner| winner.player_name.clone());
    assert_eq!(top_scorer.as_deref(), Some("Lower Star"));
}

/// Given a manager sacked from Third FC, the second division's winner, who goes up
/// When the season rolls over
/// Then the board sends him no promotion letter: the club is no longer his
#[test]
fn a_sacked_manager_hears_nothing_of_his_former_clubs_promotion() {
    let mut game = sacked_from("team3");

    process_end_of_season(&mut game);

    assert!(
        game.messages
            .iter()
            .all(|message| !message.id.contains("promot") && !message.id.contains("relegat")),
        "{:?}",
        game.messages.iter().map(|m| &m.id).collect::<Vec<_>>()
    );
}

/// A world whose manager has never had a club (no club, no stint on record: the career stage
/// "awaiting a club"), with the `game.league` mirror synced as the game syncs it, on the first
/// division.
fn never_employed() -> Game {
    let mut game = make_two_division_game("team1");
    game.manager.team_id = None;
    game.manager.career_history.clear();
    game.sync_legacy_league();
    game
}

/// Given an unemployed manager with an older career but no recorded club
/// When the world season rolls over
/// Then no unrelated league schedule is announced.
#[test]
fn no_schedule_notice_without_a_followed_division() {
    let mut game = never_employed();
    game.manager.career_stats.matches_managed = 12;
    process_end_of_season(&mut game);
    assert!(
        game.messages
            .iter()
            .all(|m| !m.id.starts_with("new_season_"))
    );
}

/// Given an unemployed manager with an older career but no recorded club
/// When a completed world season rolls over
/// Then the summary retains the season number without exposing a league.
#[test]
fn no_followed_division_keeps_the_completed_season() {
    let mut game = never_employed();
    game.manager.career_stats.matches_managed = 12;
    let season = game.competitions[0].season;
    let mut cup = game.competitions[0].clone();
    cup.rules.format = domain::league::CompetitionFormat::Knockout;
    cup.season = season + 10;
    game.league = Some(cup);
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.season, season);
    assert!(summary.league_name.is_empty());
}

/// Given a manager who has never had a club, with the mirror on the first division
/// When the season rolls over
/// Then no preview is published and the summary is empty: he follows no league, and the screens
///      show nothing of the first competition (maintainer, 8 Nov 2026)
#[test]
fn a_manager_who_never_had_a_club_is_shown_no_league() {
    let mut game = never_employed();

    let summary = process_end_of_season(&mut game);

    assert!(
        game.news
            .iter()
            .all(|article| article.category != domain::news::NewsCategory::SeasonPreview)
    );
    assert_eq!(summary.league_name, "");
}

/// Given a manager sacked from Fourth FC, and a manager who has never had a club
/// When the season rolls over
/// Then neither is sent the board's season review or a prize letter: they have no club to review,
///      and the review would read "0th" for a club called nothing; and the awards ceremony article
///      is not published for the never-employed one, who follows no league
#[test]
fn a_manager_with_no_club_is_sent_no_season_review_and_no_ceremony() {
    for mut game in [sacked_from("team4"), never_employed()] {
        process_end_of_season(&mut game);

        let ids: Vec<&str> = game.messages.iter().map(|m| m.id.as_str()).collect();
        assert!(
            ids.iter()
                .all(|id| !id.starts_with("season_end_") && !id.starts_with("season_payout_")),
            "{ids:?}"
        );
    }
    let mut game = never_employed();
    process_end_of_season(&mut game);
    assert!(
        game.news
            .iter()
            .all(|article| !article.id.starts_with("season_awards_"))
    );
}

/// Given a sacked manager whose last club is in no league table, in a world with competitions, and
///      the mirror on the first division
/// When the screens ask for his league
/// Then there is none: not the mirror's first competition, which the club has nothing to do with
#[test]
fn a_followed_club_in_no_table_shows_no_league_in_a_modern_world() {
    let mut game = make_two_division_game("team1");
    game.teams.push(make_team("team9", "Tableless FC"));
    game.manager
        .career_history
        .push(domain::manager::ManagerCareerEntry::open(
            "team9".to_string(),
            "Tableless FC".to_string(),
            "2024-07-01".to_string(),
        ));
    game.manager.fire("2025-03-01");
    game.sync_legacy_league();

    assert!(ofm_core::end_of_season::user_division(&game).is_none());
}

/// Given a manager who has never had a club
/// When the awards screen asks for the season's awards
/// Then there are none: not the first competition's
#[test]
fn a_manager_who_never_had_a_club_is_shown_no_awards() {
    let game = never_employed();

    let awards = ofm_core::season_awards::user_division_season_awards(&game);

    assert!(awards.golden_boot.is_empty());
    assert!(awards.player_of_year.is_empty());
}

/// Given a manager who has never had a club
/// When the season rolls over
/// Then the world still rolls over: every competition moves on to the next season
#[test]
fn the_season_still_rolls_over_for_a_manager_who_never_had_a_club() {
    let mut game = never_employed();
    let seasons_before: Vec<u32> = game.competitions.iter().map(|c| c.season).collect();

    process_end_of_season(&mut game);

    let seasons_after: Vec<u32> = game.competitions.iter().map(|c| c.season).collect();
    assert!(
        seasons_before
            .iter()
            .zip(&seasons_after)
            .all(|(before, after)| after > before),
        "{seasons_before:?} -> {seasons_after:?}"
    );
}

#[test]
fn season_awards_are_scoped_to_the_users_division() {
    let mut game = make_two_division_game("team1");
    // A second-division striker outscores everyone in the user's division.
    let mut lower_star = make_player("p3", "Lower Star", "team3", Position::Forward);
    lower_star.stats = PlayerSeasonStats {
        appearances: 20,
        goals: 30,
        avg_rating: 8.0,
        rated_appearances: 20,
        minutes_played: 1800,
        ..PlayerSeasonStats::default()
    };
    lower_star
        .league_stats_by_competition
        .insert("eng-2".into(), lower_star.stats.clone());
    game.players.push(lower_star);

    let summary = process_end_of_season(&mut game);

    assert_eq!(
        summary.golden_boot_player, "Star",
        "the golden boot belongs to the user's division, not the whole world"
    );
    assert_eq!(summary.golden_boot_goals, 20);
}

#[test]
fn user_staying_in_their_division_gets_no_movement_message() {
    let mut game = make_two_division_game("team1");

    process_end_of_season(&mut game);

    assert!(
        !game
            .messages
            .iter()
            .any(|m| m.id == "promotion_2" || m.id == "relegation_2"),
        "the champion stays up and gets no movement message"
    );
}

#[test]
fn rollover_before_a_world_cup_hosts_qualifying_then_stages_the_cup_from_it() {
    // Spring 2025 rolls into the 2025/26 season, which leads into the 2026 cup.
    let mut game = make_two_division_game("team1");
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2025, 5, 20, 12, 0, 0).unwrap());

    process_end_of_season(&mut game);

    let qualifying = game
        .competitions
        .iter()
        .find(|c| ofm_core::world_cup::is_world_cup_qualifying(c))
        .expect("the pre-cup season hosts qualifying");
    assert!(!qualifying.groups.is_empty());
    // The tournament itself isn't staged yet.
    assert!(
        !game.competitions.iter().any(|c| {
            ofm_core::world_cup::is_world_cup_competition(c)
                && !ofm_core::world_cup::is_world_cup_qualifying(c)
        }),
        "the World Cup waits until the cup summer"
    );
    // No preseason friendlies clash with the qualifying windows.
    let window_dates: Vec<String> = qualifying.fixtures.iter().map(|f| f.date.clone()).collect();
    assert!(
        game.competitions
            .iter()
            .filter(|c| !ofm_core::world_cup::is_world_cup_qualifying(c))
            .flat_map(|c| c.fixtures.iter())
            .all(|f| !window_dates.contains(&f.date)),
        "club fixtures stay off the qualifying window dates"
    );
}

#[test]
fn rollover_stages_a_world_cup_in_cup_years_and_retires_it_afterwards() {
    // make_two_division_game's clock sits in May 2026 — a World Cup summer.
    let mut game = make_two_division_game("team1");

    process_end_of_season(&mut game);

    let cup = game
        .competitions
        .iter()
        .find(|c| ofm_core::world_cup::is_world_cup_competition(c))
        .expect("a 2026 rollover stages the World Cup");
    assert_eq!(cup.name, "World Cup 2026");
    assert_eq!(
        cup.participant_ids.len(),
        48,
        "the 2026 format fields 48 nations"
    );
    assert_eq!(cup.groups.len(), 12);
    assert!(
        !is_season_complete(&game),
        "the freshly regenerated club season is open and the tournament never \
         counts toward season completion"
    );
    // Clubs play no preseason friendlies while the World Cup runs.
    assert!(
        game.competitions
            .iter()
            .flat_map(|c| c.fixtures.iter())
            .all(|f| f.competition != FixtureCompetition::Friendly),
        "no club friendlies may be scheduled during a World Cup summer"
    );

    // The next rollover (summer 2027 — not a cup year) retires the tournament
    // instead of regenerating it, and stages no new one.
    // Finish the intervening club season: an unfinished division is retained,
    // and its old opener must not receive a second, now-past preseason.
    for competition in game
        .competitions
        .iter_mut()
        .filter(|c| c.kind == CompetitionType::League && c.scope == CompetitionScope::Domestic)
    {
        for fixture in &mut competition.fixtures {
            fixture.status = FixtureStatus::Completed;
            fixture.result = Some(MatchResult {
                home_goals: 1,
                away_goals: 0,
                ..Default::default()
            });
        }
    }
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);
    assert!(
        game.competitions
            .iter()
            .all(|c| !ofm_core::world_cup::is_world_cup_competition(c)),
        "last cycle's World Cup must be retired at the next rollover"
    );
    // Ordinary summers bring the preseason friendlies back.
    assert!(
        game.competitions
            .iter()
            .flat_map(|c| c.fixtures.iter())
            .any(|f| f.competition == FixtureCompetition::Friendly),
        "non-cup summers schedule preseason friendlies as before"
    );
}

#[test]
fn a_two_season_qualifying_campaign_survives_the_rollover_and_feeds_the_cup() {
    // Spring 2024 rolls into 2024/25 — two summers before the 2026 cup — so
    // the full home-and-away qualifying campaign gets under way.
    let mut game = make_two_division_game("team1");
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2024, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);

    let (qualifying_id, first_season_days) = {
        let qualifying = game
            .competitions
            .iter()
            .find(|c| ofm_core::world_cup::is_world_cup_qualifying(c))
            .expect("the campaign starts two seasons out");
        assert_eq!(qualifying.season, 2026);
        let windows = ofm_core::national_team::international_window_dates(
            Utc.with_ymd_and_hms(2024, 8, 1, 0, 0, 0).unwrap(),
        );
        (
            qualifying.id.clone(),
            ofm_core::national_team::international_window_span_dates(&windows),
        )
    };

    // Play the first season's share of the campaign only up to the club
    // season's end in May: the rollover outruns the June 2025 window.
    for date in first_season_days
        .iter()
        .filter(|date| date.as_str() < "2025-06-01")
    {
        ofm_core::world_cup::process_world_cup_fixtures_due(&mut game, date);
    }
    let played_midway = game
        .competitions
        .iter()
        .find(|c| c.id == qualifying_id)
        .unwrap()
        .fixtures
        .iter()
        .filter(|f| f.status == FixtureStatus::Completed)
        .count();
    assert!(played_midway > 0, "the campaign's first half is played");

    // The intermediate rollover (summer 2025) carries the campaign over
    // instead of retiring it: same competition, results intact, and its
    // remaining fixtures re-anchored onto the new season's windows.
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2025, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);
    let second_season_days = {
        let qualifying = game
            .competitions
            .iter()
            .find(|c| ofm_core::world_cup::is_world_cup_qualifying(c))
            .expect("an in-progress campaign survives the rollover");
        assert_eq!(qualifying.id, qualifying_id, "the same edition continues");
        let played_after = qualifying
            .fixtures
            .iter()
            .filter(|f| f.status == FixtureStatus::Completed)
            .count();
        assert!(
            played_after > played_midway,
            "played results carry over and the outrun June window is settled"
        );
        let windows = ofm_core::national_team::international_window_dates(
            Utc.with_ymd_and_hms(2025, 8, 1, 0, 0, 0).unwrap(),
        );
        let window_days: std::collections::HashSet<String> =
            ofm_core::national_team::international_window_span_dates(&windows)
                .into_iter()
                .collect();
        assert!(
            qualifying
                .fixtures
                .iter()
                .filter(|f| f.status == FixtureStatus::Scheduled)
                .all(|f| window_days.contains(&f.date)),
            "the campaign's second half sits on the new season's windows"
        );
        ofm_core::national_team::international_window_span_dates(&windows)
    };

    // Play the campaign through the March window only: the club season ends in
    // mid-May, *before* the playoff's June dates — the realistic flow.
    for date in second_season_days
        .iter()
        .filter(|date| date.as_str() < "2026-06-01")
    {
        ofm_core::world_cup::process_world_cup_fixtures_due(&mut game, date);
    }
    assert!(
        game.competitions
            .iter()
            .any(ofm_core::world_cup::is_world_cup_playoff),
        "the finished groups stage the June playoff"
    );

    // The cup-summer rollover settles the still-scheduled June playoff, then
    // derives the 48-team field from the finished campaign and retires both
    // the campaign and its playoff.
    game.clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
    process_end_of_season(&mut game);
    assert!(
        game.news.iter().any(|a| a.id == "world_cup_playoff_2026"),
        "the playoff bracket was played out, not dropped"
    );
    let cup = game
        .competitions
        .iter()
        .find(|c| ofm_core::world_cup::is_world_cup_competition(c))
        .expect("the cup summer stages the finals");
    assert_eq!(cup.participant_ids.len(), 48, "a full qualified field");
    assert!(
        !game
            .competitions
            .iter()
            .any(|c| ofm_core::world_cup::is_world_cup_qualifying(c)
                || ofm_core::world_cup::is_world_cup_playoff(c)),
        "the finished campaign and playoff are retired with the rollover"
    );
}

/// A manager with no club of their own — between jobs, or managing only a
/// national side — has no division to gate on, so every league table must
/// finish before the season can roll over. This is the path the test below
/// used to take by accident, because its primary league listed no participants.
#[test]
fn season_waits_for_every_league_when_the_user_has_no_division() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    let finished = game.league.clone().expect("a primary league");
    let mut other = League {
        id: "far-1".to_string(),
        name: "Far Away League".to_string(),
        country_id: Some("BR".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("ff1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    other.fixtures[0].status = FixtureStatus::Scheduled;
    other.fixtures[0].result = None;
    // No club: the manager is unemployed.
    game.manager.team_id = None;
    game.competitions = vec![finished, other];

    assert!(
        !is_season_complete(&game),
        "with no division of their own, every league must finish first"
    );
}

#[test]
fn season_not_complete_while_another_division_is_unfinished() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    // div1 (the primary) is fully played; div2 still has a scheduled fixture.
    //
    // div1 must name its participants and its country, or the user is not found
    // in any league and this exercises the unknown-user fallback below instead
    // of the gate it claims to test.
    let mut div1 = game.league.clone().unwrap();
    div1.participant_ids = vec!["team1".to_string(), "team2".to_string()];
    div1.country_id = Some("ENG".to_string());
    let mut div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![
            make_completed_fixture("d2f1", "team3", "team4", 3, 0),
            make_completed_fixture("d2f2", "team4", "team3", 0, 1),
        ],
        standings: vec![
            make_standing("team3", 2, 0, 0, 4, 0),
            make_standing("team4", 0, 0, 2, 0, 4),
        ],
        ..Default::default()
    };
    div2.fixtures[1].status = FixtureStatus::Scheduled;
    div2.fixtures[1].result = None;
    // Dated ahead of the clock, so the day loop can still reach it. A fixture
    // the clock has already passed is stale, not pending, and deliberately does
    // not hold the season open.
    div2.fixtures[1].date = "2026-06-10".to_string();
    game.competitions = vec![div1, div2];

    assert!(
        !is_season_complete(&game),
        "season must wait for every division, not just the primary league"
    );

    // Finish the second division and the season completes.
    game.competitions[1].fixtures[1].status = FixtureStatus::Completed;
    game.competitions[1].fixtures[1].result = Some(MatchResult {
        home_goals: 0,
        away_goals: 1,
        home_scorers: vec![],
        away_scorers: vec![],
        report: None,
        home_penalties: None,
        away_penalties: None,
    });
    assert!(is_season_complete(&game));
}

#[test]
fn summary_reflects_the_users_division_when_not_in_the_primary_competition() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));
    // The user manages the second-division champion, not a primary-league club.
    game.manager.team_id = Some("team3".to_string());

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0),
            make_standing("team2", 0, 0, 2, 0, 3),
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];

    let summary = process_end_of_season(&mut game);

    assert_eq!(summary.league_name, "ENG Second Division");
    assert_eq!(summary.competition_id, "eng-2");
    assert_eq!(summary.user_position, 1);
    assert_eq!(summary.champion_id, "team3");
    assert_eq!(summary.total_teams, 2);
    assert_eq!(
        game.manager.career_stats.trophies, 1,
        "winning your division counts as a trophy"
    );

    // The payout message matches the tier-scaled ledger (half the top-flight
    // champion's 5,000,000).
    let payout = game
        .messages
        .iter()
        .find(|m| m.id == "season_payout_1")
        .expect("payout message for the user's division");
    assert_eq!(
        payout.i18n_params.get("amount"),
        Some(&"2500000".to_string())
    );
}

#[test]
fn process_end_of_season_promotes_and_relegates_between_divisions() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    let div1 = League {
        id: "eng-1".to_string(),
        name: "ENG First Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![
            make_completed_fixture("d1f1", "team1", "team2", 2, 0),
            make_completed_fixture("d1f2", "team2", "team1", 0, 1),
        ],
        standings: vec![
            make_standing("team1", 2, 0, 0, 3, 0), // champion
            make_standing("team2", 0, 0, 2, 0, 3), // relegated
        ],
        ..Default::default()
    };
    let div2 = League {
        id: "eng-2".to_string(),
        name: "ENG Second Division".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 1,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("d2f1", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0), // promoted
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    game.league = Some(div1.clone());
    game.competitions = vec![div1, div2];

    process_end_of_season(&mut game);

    let new_div1 = game.competitions.iter().find(|c| c.id == "eng-1").unwrap();
    let new_div2 = game.competitions.iter().find(|c| c.id == "eng-2").unwrap();

    // Two-club divisions -> one up / one down: team2 relegated, team3 promoted.
    assert!(new_div1.participant_ids.contains(&"team3".to_string()));
    assert!(!new_div1.participant_ids.contains(&"team2".to_string()));
    assert!(new_div2.participant_ids.contains(&"team2".to_string()));
    assert!(!new_div2.participant_ids.contains(&"team3".to_string()));

    // Fixtures regenerated for the new season (calendar year, not sequential).
    assert_eq!(new_div1.season, 2026);
    assert!(
        new_div1
            .fixtures
            .iter()
            .any(|f| f.status == FixtureStatus::Scheduled)
    );
}

/// Every first division berths its top finishers into the same continental cup,
/// which is what `create_game` builds. Those leagues qualify *for* the cup; they
/// do not feed clubs *into* it, so a shared continental target must not take
/// them out of their own domestic ladder the way sibling regional groups are
/// taken out of it. Two countries, because one feeder is never a sibling and so
/// a single-country world hides the bug entirely.
#[test]
fn a_shared_continental_berth_does_not_detach_first_divisions_from_their_ladder() {
    let mut game = make_completed_season_game();
    for id in [
        "eng1", "eng2", "eng3", "eng4", "esp1", "esp2", "esp3", "esp4",
    ] {
        game.teams.push(make_club(id, &id[..3].to_uppercase(), 100));
    }

    let continental_berth = || Berth {
        target: "ccc".to_string(),
        rule: BerthRule::PositionRange { from: 1, to: 2 },
        fallback_to: None,
    };
    let tier = |id: &str, country: &str, priority: u32, clubs: [&str; 2]| {
        let mut division = first_division(id, country, "europe", &clubs);
        division.priority = priority;
        division.fixtures = vec![make_completed_fixture(
            &format!("{id}-f1"),
            clubs[0],
            clubs[1],
            2,
            0,
        )];
        division
    };

    let mut eng_d1 = tier("eng-d1", "ENG", 0, ["eng1", "eng2"]);
    eng_d1.berths = vec![continental_berth()];
    let eng_d2 = tier("eng-d2", "ENG", 1, ["eng3", "eng4"]);
    let mut esp_d1 = tier("esp-d1", "ES", 2, ["esp1", "esp2"]);
    esp_d1.berths = vec![continental_berth()];
    let esp_d2 = tier("esp-d2", "ES", 3, ["esp3", "esp4"]);

    // Group-and-knockout, as `create_game` builds it: the cup is not a league
    // table, so it can never be a promotion destination.
    let mut cup = continental_cup("ccc", "europe", 4);
    cup.rules.format = CompetitionFormat::GroupAndKnockout;

    game.league = Some(eng_d1.clone());
    game.competitions = vec![eng_d1, eng_d2, esp_d1, esp_d2, cup];

    process_end_of_season(&mut game);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    // Both countries, so the fix cannot be a one-off that happens to spare
    // whichever league sorts first.
    for prefix in ["eng", "esp"] {
        let d1 = &by_id(&format!("{prefix}-d1")).participant_ids;
        let d2 = &by_id(&format!("{prefix}-d2")).participant_ids;
        assert!(
            d2.contains(&format!("{prefix}2")),
            "{prefix}2 finished bottom of the top flight and should be relegated: \
             d1={d1:?} d2={d2:?}"
        );
        assert!(
            d1.contains(&format!("{prefix}3")),
            "{prefix}3 won the second division and should be promoted: d1={d1:?} d2={d2:?}"
        );
    }

    // Qualification still resolves in the same rollover — narrowing the sibling
    // rule must not cost the cup its field.
    let field = &by_id("ccc").participant_ids;
    assert!(
        field.contains(&"eng1".to_string()) && field.contains(&"esp1".to_string()),
        "both champions should have qualified for the continental cup: {field:?}"
    );
}

/// Two sibling regional groups feeding one Central League through PositionRange
/// berths. Goes through `process_end_of_season` so deleting the resolve/apply
/// call sites in `regenerate_competitions_for_new_season` fails this test.
#[test]
fn process_end_of_season_merges_regional_groups_into_a_berth_fed_central() {
    use std::collections::HashSet;

    let mut game = make_completed_season_game();
    game.manager.hire("c1".to_string());
    for id in [
        "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "n1", "n2", "n3", "n4", "s1", "s2", "s3",
        "s4", "i1", "i2", "d1", "d2",
    ] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }

    let standing = |id: &str, points: u32| {
        let mut entry = make_standing(id, 1, 0, 0, 1, 0);
        entry.points = points;
        entry.played = 1;
        entry
    };
    let finished_table = |id: &str, priority: u32, rows: &[(&str, u32)], berths: Vec<Berth>| {
        let ids: Vec<String> = rows.iter().map(|(club, _)| club.to_string()).collect();
        League {
            id: id.to_string(),
            name: id.to_string(),
            country_id: Some("BR".to_string()),
            priority,
            season: 1,
            participant_ids: ids.clone(),
            standings: rows
                .iter()
                .map(|(club, points)| standing(club, *points))
                .collect(),
            berths,
            ..Default::default()
        }
    };
    let place_berth = |target: &str, from: u32, to: u32| Berth {
        target: target.to_string(),
        rule: BerthRule::PositionRange { from, to },
        fallback_to: None,
    };

    let central = finished_table(
        "central",
        0,
        &[
            ("c1", 80),
            ("c2", 70),
            ("c3", 60),
            ("c4", 50),
            ("c5", 40),
            ("c6", 30),
            ("c7", 20),
            ("c8", 10),
        ],
        vec![],
    );
    let north = finished_table(
        "north",
        1,
        &[("n1", 40), ("n2", 30), ("n3", 20), ("n4", 10)],
        vec![place_berth("central", 1, 2)],
    );
    let south = finished_table(
        "south",
        1,
        &[("s1", 40), ("s2", 30), ("s3", 20), ("s4", 10)],
        vec![place_berth("central", 1, 2)],
    );
    // A plain pair below the groups, so the linear ladder has real work to do
    // in the same rollover as the berth merge. Without it neither pass can
    // observe the other and the two could run in either order.
    let interior = finished_table("interior", 2, &[("i1", 20), ("i2", 10)], vec![]);
    let deep = finished_table("deep", 3, &[("d1", 20), ("d2", 10)], vec![]);
    game.league = Some(central.clone());
    game.competitions = vec![central, north, south, interior, deep];

    process_end_of_season(&mut game);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    let central_ids = &by_id("central").participant_ids;
    assert_eq!(
        central_ids.len(),
        8,
        "Central League keeps its authored size: {central_ids:?}"
    );
    let central_set: HashSet<&str> = central_ids.iter().map(String::as_str).collect();
    assert!(
        ["n1", "n2", "s1", "s2"]
            .iter()
            .all(|club| central_set.contains(club)),
        "regional place-getters must be promoted through process_end_of_season: {central_ids:?}"
    );
    assert!(
        ["c5", "c6", "c7", "c8"]
            .iter()
            .all(|club| !central_set.contains(club)),
        "Central dropouts must leave: {central_ids:?}"
    );

    let north_set: HashSet<&str> = by_id("north")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    let south_set: HashSet<&str> = by_id("south")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert!(!north_set.contains("n1") && !north_set.contains("n2"));
    assert!(!south_set.contains("s1") && !south_set.contains("s2"));
    assert!(
        central_set.is_disjoint(&north_set)
            && central_set.is_disjoint(&south_set)
            && north_set.is_disjoint(&south_set),
        "a club must not appear in two tables after the berth merge: central={central_set:?} north={north_set:?} south={south_set:?}"
    );
    let dropouts: HashSet<&str> = north_set
        .union(&south_set)
        .copied()
        .filter(|id| id.starts_with('c'))
        .collect();
    assert_eq!(dropouts, HashSet::from(["c5", "c6", "c7", "c8"]));

    assert!(
        by_id("central")
            .fixtures
            .iter()
            .any(|fixture| fixture.status == FixtureStatus::Scheduled),
        "rollover must regenerate Central fixtures after the berth merge"
    );
    assert!(
        by_id("north")
            .fixtures
            .iter()
            .any(|fixture| fixture.status == FixtureStatus::Scheduled)
            && by_id("south")
                .fixtures
                .iter()
                .any(|fixture| fixture.status == FixtureStatus::Scheduled),
        "feeders must also receive a new schedule"
    );

    // The berth-fed groups leave the ladder, but the plain pair below them
    // still swaps in the very same rollover.
    let interior_set: HashSet<&str> = by_id("interior")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    let deep_set: HashSet<&str> = by_id("deep")
        .participant_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        interior_set,
        HashSet::from(["i1", "d1"]),
        "the tier below the groups still promotes: {:?}",
        by_id("interior").participant_ids
    );
    assert_eq!(
        deep_set,
        HashSet::from(["d2", "i2"]),
        "the tier below the groups still relegates: {:?}",
        by_id("deep").participant_ids
    );
}

/// A feeder that both sends clubs up through berths and swaps with the tier
/// below is touched by both rollover passes. Pins their order and their
/// agreement: run through `process_end_of_season` so reordering the calls in
/// `end_of_season/mod.rs`, or reading the frozen table instead of the live
/// roster, leaves a club registered with two competitions.
#[test]
fn process_end_of_season_never_leaves_a_club_in_two_leagues() {
    use std::collections::HashSet;

    let mut game = make_completed_season_game();
    game.manager.hire("c1".to_string());
    for id in ["c1", "c2", "c3", "c4", "m1", "m2", "l1", "l2"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }

    let table = |id: &str, priority: u32, rows: &[(&str, u32)], berths: Vec<Berth>| {
        let ids: Vec<String> = rows.iter().map(|(club, _)| club.to_string()).collect();
        League {
            id: id.to_string(),
            name: id.to_string(),
            country_id: Some("BR".to_string()),
            priority,
            season: 1,
            participant_ids: ids,
            standings: rows
                .iter()
                .map(|(club, points)| {
                    let mut entry = make_standing(club, 1, 0, 0, 1, 0);
                    entry.points = *points;
                    entry.played = 1;
                    entry
                })
                .collect(),
            berths,
            ..Default::default()
        }
    };

    let central = table(
        "central",
        0,
        &[("c1", 40), ("c2", 30), ("c3", 20), ("c4", 10)],
        vec![],
    );
    let mid = table(
        "mid",
        1,
        &[("m1", 20), ("m2", 10)],
        vec![Berth {
            target: "central".to_string(),
            rule: BerthRule::PositionRange { from: 1, to: 2 },
            fallback_to: None,
        }],
    );
    let low = table("low", 2, &[("l1", 20), ("l2", 10)], vec![]);
    game.league = Some(central.clone());
    game.competitions = vec![central, mid, low];

    process_end_of_season(&mut game);

    let by_id = |id: &str| game.competitions.iter().find(|c| c.id == id).expect(id);
    let roster = |id: &str| -> HashSet<&str> {
        by_id(id)
            .participant_ids
            .iter()
            .map(String::as_str)
            .collect()
    };
    let (central_set, mid_set, low_set) = (roster("central"), roster("mid"), roster("low"));

    assert!(
        central_set.is_disjoint(&mid_set)
            && central_set.is_disjoint(&low_set)
            && mid_set.is_disjoint(&low_set),
        "a club must play for exactly one league: central={:?} mid={:?} low={:?}",
        by_id("central").participant_ids,
        by_id("mid").participant_ids,
        by_id("low").participant_ids
    );
    for (id, size) in [("central", 4), ("mid", 2), ("low", 2)] {
        assert_eq!(
            by_id(id).participant_ids.len(),
            size,
            "{id} keeps its authored size: {:?}",
            by_id(id).participant_ids
        );
    }
    assert!(
        low_set.contains("m2"),
        "the ladder relegated m2, so the berth merge must not also promote it: {:?}",
        by_id("low").participant_ids
    );
    assert!(
        central_set.contains("m1"),
        "the place-getter still registered with mid is promoted: {:?}",
        by_id("central").participant_ids
    );
}

// ---------------------------------------------------------------------------
// is_season_complete
// ---------------------------------------------------------------------------

#[test]
fn expected_fixture_count_covers_odd_team_counts() {
    // Each leg is n * (n - 1) / 2 fixtures; odd leagues play it out with byes.
    assert_eq!(expected_fixture_count(4, 2), Some(12));
    assert_eq!(expected_fixture_count(5, 2), Some(20));
    assert_eq!(expected_fixture_count(4, 1), Some(6));
    assert_eq!(expected_fixture_count(5, 1), Some(10));
    assert_eq!(expected_fixture_count(1, 2), None);
}

/// The user's league of `club_count` clubs, scheduled over `legs` legs, every fixture played and
/// won by the home side. `stored_legs` is what the league's calendar says it was authored with;
/// `None` is a save from before it was stored. With `in_competitions`, the league is also the
/// world's one competition, the path every new career takes; without, it sits only in the legacy
/// `game.league` slot.
fn round_robin_season_game(
    club_count: usize,
    legs: u8,
    stored_legs: Option<u8>,
    in_competitions: bool,
) -> Game {
    let mut game = make_completed_season_game();
    for index in 3..=club_count {
        game.teams
            .push(make_team(&format!("team{index}"), &format!("Club {index}")));
    }
    let clubs: Vec<String> = (1..=club_count)
        .map(|index| format!("team{index}"))
        .collect();
    let start = Utc.with_ymd_and_hms(2025, 8, 1, 0, 0, 0).unwrap();
    let mut fixtures = ofm_core::schedule::build_round_robin_fixtures_with(
        "league1",
        &clubs,
        start,
        FixtureCompetition::League,
        legs,
        7,
    );
    let mut standings: Vec<StandingEntry> = clubs
        .iter()
        .map(|id| StandingEntry::new(id.clone()))
        .collect();
    for fixture in &mut fixtures {
        fixture.status = FixtureStatus::Completed;
        fixture.result = Some(MatchResult {
            home_goals: 1,
            away_goals: 0,
            home_scorers: vec![],
            away_scorers: vec![],
            report: None,
            home_penalties: None,
            away_penalties: None,
        });
        for entry in &mut standings {
            if entry.team_id == fixture.home_team_id {
                entry.record_result(1, 0);
            } else if entry.team_id == fixture.away_team_id {
                entry.record_result(0, 1);
            }
        }
    }
    let league = game.league.as_mut().unwrap();
    league.participant_ids = clubs;
    league.fixtures = fixtures;
    league.standings = standings;
    league.calendar = stored_legs.map(|legs| CalendarMetadata {
        league_legs: Some(legs),
        ..CalendarMetadata::default()
    });
    if in_competitions {
        game.competitions = vec![league.clone()];
    }
    game
}

fn single_round_robin_season_game(stored_legs: Option<u8>) -> Game {
    round_robin_season_game(4, 1, stored_legs, false)
}

fn league_fixture_count(game: &Game) -> usize {
    game.league
        .as_ref()
        .unwrap()
        .fixtures
        .iter()
        .filter(|fixture| fixture.counts_for_league_standings())
        .count()
}

/// Given the user's league authored as a single round robin of four clubs, all six fixtures
/// played; when the season's end is checked and the season advanced; then it is complete and
/// the career moves on (#1177: it waited for twelve fixtures that would never exist).
#[test]
fn a_single_round_robin_league_completes_when_every_fixture_is_played() {
    let mut game = single_round_robin_season_game(Some(1));
    assert_eq!(league_fixture_count(&game), 6);

    assert!(is_season_complete(&game));
    assert!(advance_to_next_season(&mut game).is_ok());
}

/// Given a finished single round robin; when the season rolls over; then the next season is
/// scheduled as a single round robin again, not a double one.
#[test]
fn a_single_round_robin_league_is_rescheduled_as_a_single_round_robin() {
    let mut game = single_round_robin_season_game(Some(1));

    advance_to_next_season(&mut game).expect("the season should roll over");

    assert_eq!(league_fixture_count(&game), 6);
}

/// Given a league with no stored legs (no calendar) holding one leg's fixtures, all played, as a
/// save from before the legs were stored has it; when its season's end is checked as it stands, it
/// is not complete: the default is the double round robin. When the load path gives it its
/// calendar, as loading that save does; then the fixtures prove it single, the season is complete,
/// and it comes back as a single round robin (#1177).
#[test]
fn a_save_from_before_stored_legs_proves_a_single_round_robin_on_load() {
    let mut game = round_robin_season_game(4, 1, None, true);
    assert!(!is_season_complete(&game));

    for league in game.competitions.iter_mut().chain(game.league.as_mut()) {
        ofm_core::calendar_identity::ensure_competition_calendar(league);
    }
    assert_eq!(
        game.competitions[0].calendar.as_ref().unwrap().league_legs,
        Some(1)
    );

    assert!(is_season_complete(&game));
    advance_to_next_season(&mut game).expect("the season should roll over");
    let league = &game.competitions[0];
    assert_eq!(
        league
            .fixtures
            .iter()
            .filter(|fixture| fixture.counts_for_league_standings())
            .count(),
        6
    );
}

/// Given a save from before the authored legs were stored, whose league holds a double round
/// robin's twelve fixtures with one still to play; when the load path gives it its calendar and
/// the season's end is checked; then its legs stay unknown, it is not complete, and once all
/// twelve are played it rolls over into twelve again.
#[test]
fn a_league_with_no_stored_legs_and_a_double_round_robin_needs_all_of_it() {
    let mut game = round_robin_season_game(4, 2, None, false);
    if let Some(league) = game.league.as_mut() {
        ofm_core::calendar_identity::ensure_competition_calendar(league);
    }
    assert_eq!(
        game.league
            .as_ref()
            .unwrap()
            .calendar
            .as_ref()
            .unwrap()
            .league_legs,
        None
    );
    let last = game.league.as_ref().unwrap().fixtures.len() - 1;
    game.league.as_mut().unwrap().fixtures[last].status = FixtureStatus::Scheduled;
    assert!(!is_season_complete(&game));

    game.league.as_mut().unwrap().fixtures[last].status = FixtureStatus::Completed;
    assert!(is_season_complete(&game));
    advance_to_next_season(&mut game).expect("the season should roll over");
    assert_eq!(league_fixture_count(&game), 12);
}

/// Given a five-club single round robin (each club has a bye) stored as `legs: 1`, held in the
/// world's competitions as every new career's league is; when the season's end is checked and it
/// rolls over; then it is complete and its ten fixtures come back as ten.
#[test]
fn a_single_round_robin_in_the_worlds_competitions_completes_with_an_odd_field() {
    let mut game = round_robin_season_game(5, 1, Some(1), true);
    assert_eq!(league_fixture_count(&game), 10);

    assert!(is_season_complete(&game));
    advance_to_next_season(&mut game).expect("the season should roll over");
    let league = &game.competitions[0];
    assert_eq!(
        league
            .fixtures
            .iter()
            .filter(|fixture| fixture.counts_for_league_standings())
            .count(),
        10
    );
}

#[test]
fn season_complete_when_all_fixtures_completed() {
    let game = make_completed_season_game();
    assert!(is_season_complete(&game));
}

#[test]
fn season_not_complete_with_scheduled_fixtures() {
    // One of the two league fixtures is still Scheduled.
    // has_full_schedule returns true (2 == 2), but .all(Completed) returns false.
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        league.fixtures[1].status = FixtureStatus::Scheduled;
        league.fixtures[1].result = None;
    }
    assert!(!is_season_complete(&game));
}

#[test]
fn season_not_complete_with_no_league() {
    let mut game = make_completed_season_game();
    game.league = None;
    assert!(!is_season_complete(&game));
}

#[test]
fn season_not_complete_with_empty_fixtures() {
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        league.fixtures.clear();
    }
    assert!(!is_season_complete(&game));
}

#[test]
fn season_not_complete_with_truncated_completed_fixture_list() {
    let mut game = make_completed_season_game();
    game.teams.push(make_team("team3", "Third FC"));
    game.teams.push(make_team("team4", "Fourth FC"));

    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team1", 1, 0, 0, 2, 0),
            make_standing("team4", 1, 0, 0, 1, 0),
            make_standing("team3", 0, 0, 1, 0, 1),
            make_standing("team2", 0, 0, 1, 0, 2),
        ];
        league.fixtures = vec![
            Fixture {
                id: "f1".to_string(),
                matchday: 1,
                date: "2026-08-01".to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 2,
                    away_goals: 0,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                    home_penalties: None,
                    away_penalties: None,
                }),
                ..Default::default()
            },
            Fixture {
                id: "f2".to_string(),
                matchday: 1,
                date: "2026-08-01".to_string(),
                home_team_id: "team3".to_string(),
                away_team_id: "team4".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 0,
                    away_goals: 1,
                    home_scorers: vec![],
                    away_scorers: vec![],
                    report: None,
                    home_penalties: None,
                    away_penalties: None,
                }),
                ..Default::default()
            },
        ];
    }

    assert!(
        !is_season_complete(&game),
        "A truncated fixture list must not count as a completed season"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — summary
// ---------------------------------------------------------------------------

#[test]
fn summary_has_correct_champion() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.champion_id, "team1");
    assert_eq!(summary.champion_name, "Test FC");
    assert_eq!(summary.season, 1);
}

#[test]
fn summary_has_correct_user_position() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    // team1 (user) is champion
    assert_eq!(summary.user_position, 1);
    assert_eq!(summary.user_points, 6);
    assert_eq!(summary.user_won, 2);
    assert_eq!(summary.user_drawn, 0);
    assert_eq!(summary.user_lost, 0);
}

#[test]
fn summary_has_correct_goals() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.user_goals_for, 3);
    assert_eq!(summary.user_goals_against, 1);
}

#[test]
fn summary_total_teams() {
    let mut game = make_completed_season_game();
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.total_teams, 2);
}

// ---------------------------------------------------------------------------
// process_end_of_season — history recording
// ---------------------------------------------------------------------------

#[test]
fn team_history_recorded() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    assert_eq!(team1.history.len(), 1);
    let record = &team1.history[0];
    assert_eq!(record.season, 1);
    assert_eq!(record.league_position, 1);
    assert_eq!(record.won, 2);
    assert_eq!(record.drawn, 0);
    assert_eq!(record.lost, 0);

    let team2 = game.teams.iter().find(|t| t.id == "team2").unwrap();
    assert_eq!(team2.history.len(), 1);
    assert_eq!(team2.history[0].league_position, 2);
}

#[test]
fn team_form_cleared() {
    let mut game = make_completed_season_game();
    // Give team1 some form
    game.teams
        .iter_mut()
        .find(|t| t.id == "team1")
        .unwrap()
        .form = vec!["W".to_string(), "W".to_string()];

    process_end_of_season(&mut game);

    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    assert!(
        team1.form.is_empty(),
        "Form should be cleared after season end"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — player career/stats reset
// ---------------------------------------------------------------------------

#[test]
fn player_career_entry_added() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let p1 = game.players.iter().find(|p| p.id == "p1").unwrap();
    assert_eq!(p1.career.len(), 1);
    let entry = &p1.career[0];
    assert_eq!(entry.season, 1);
    assert_eq!(entry.appearances, 30);
    assert_eq!(entry.goals, 20);
    assert_eq!(entry.assists, 10);
}

#[test]
fn season_end_ages_players_and_retires_out_of_contract_veterans() {
    let mut game = make_completed_season_game();
    let veteran = game
        .players
        .iter_mut()
        .find(|player| player.id == "p1")
        .unwrap();
    veteran.date_of_birth = "1988-01-01".to_string();
    veteran.stage_contract_end(Some("2026-05-01".to_string()));
    veteran.attributes.pace = 16;
    veteran.attributes.passing = 70;

    process_end_of_season(&mut game);

    assert!(
        game.players.iter().all(|player| player.id != "p1"),
        "a retired veteran leaves the roster"
    );
    let veteran = game
        .retired_players
        .iter()
        .find(|record| record.id == "p1")
        .expect("older out-of-contract veterans should retire into the archive")
        .to_player();
    assert_eq!(
        veteran.team_id, None,
        "retired players should leave their club"
    );
    assert!(
        veteran.attributes.pace < 16,
        "seasonal aging should reduce veteran pace"
    );
    assert!(
        veteran.attributes.passing < 70,
        "technique declines too, more slowly than pace, at thirty-eight"
    );
}

#[test]
fn player_with_zero_appearances_no_career_entry() {
    let mut game = make_completed_season_game();
    // Add a player with 0 appearances
    let p3 = make_player("p3", "Bench", "team1", Position::Defender);
    game.players.push(p3);

    process_end_of_season(&mut game);

    let p3 = game.players.iter().find(|p| p.id == "p3").unwrap();
    assert!(
        p3.career.is_empty(),
        "No career entry for 0-appearance player"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — manager career
// ---------------------------------------------------------------------------

#[test]
fn manager_career_stats_updated() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    assert_eq!(game.manager.career_stats.matches_managed, 2);
    assert_eq!(game.manager.career_stats.wins, 2);
    assert_eq!(game.manager.career_stats.draws, 0);
    assert_eq!(game.manager.career_stats.losses, 0);
}

/// Given a finished season in which Rival FC (team2), managed by another manager, lost both matches
/// When the season ends
/// Then his record has that season too, not only the user's: two matches, two defeats, last place
#[test]
fn another_clubs_manager_has_his_season_on_his_record() {
    let mut game = make_completed_season_game();
    let mut rival_boss = Manager::new(
        "mgr2".to_string(),
        "Rival".to_string(),
        "Boss".to_string(),
        "1978-01-01".to_string(),
        "England".to_string(),
    );
    rival_boss.hire("team2".to_string());
    game.managers.push(rival_boss);
    // A generated world's league, which the rollover can archive: its clubs, and a calendar that
    // says it is a double round robin.
    let league = game.league.as_mut().unwrap();
    league.participant_ids = vec!["team1".to_string(), "team2".to_string()];
    league.calendar = Some(domain::competition_calendar::CalendarMetadata {
        league_legs: Some(2),
        ..Default::default()
    });
    for fixture in &mut league.fixtures {
        fixture.competition_id = league.id.clone();
    }

    process_end_of_season(&mut game);

    let rival_boss = game.managers.iter().find(|m| m.id == "mgr2").unwrap();
    assert_eq!(rival_boss.career_stats.matches_managed, 2);
    assert_eq!(rival_boss.career_stats.losses, 2);
    assert_eq!(rival_boss.career_stats.best_finish, Some(2));
    assert_eq!(game.manager.career_stats.matches_managed, 2);
}

#[test]
fn manager_trophy_awarded_for_first_place() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);
    assert_eq!(game.manager.career_stats.trophies, 1);
}

/// Given the user's manager also listed in `game.managers` (the managers list and a live match's
///      AI profile read him there), in step with `game.manager`
/// When his season is credited at the season's end
/// Then the listed copy carries the same record and satisfaction, not last season's until the
///      next day's sync
#[test]
fn the_users_listed_record_takes_his_season_too() {
    let mut game = make_completed_season_game();
    game.sync_user_manager_record();

    process_end_of_season(&mut game);

    // The record as the managers list shows it: totals, then each stint's club, end and matches.
    let record = |manager: &Manager| {
        let stats = &manager.career_stats;
        (
            (
                stats.matches_managed,
                stats.wins,
                stats.draws,
                stats.losses,
                stats.trophies,
                stats.best_finish,
            ),
            manager
                .career_history
                .iter()
                .map(|stint| (stint.team_id.clone(), stint.end_date.clone(), stint.matches))
                .collect::<Vec<_>>(),
            manager.satisfaction,
        )
    };
    let listed = game
        .managers
        .iter()
        .find(|manager| manager.id == game.manager_id)
        .expect("the user's manager is listed");
    assert_eq!(record(listed), record(&game.manager));
    assert_eq!(listed.career_stats.trophies, 1);
}

#[test]
fn manager_no_trophy_for_non_first() {
    let mut game = make_completed_season_game();
    // Swap standings so team2 is first
    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team2", 2, 0, 0, 3, 1),
            make_standing("team1", 0, 0, 2, 1, 3),
        ];
    }
    process_end_of_season(&mut game);
    assert_eq!(game.manager.career_stats.trophies, 0);
}

#[test]
fn manager_best_finish_set() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);
    assert_eq!(game.manager.career_stats.best_finish, Some(1));
}

#[test]
fn manager_career_history_entry_created() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    assert_eq!(game.manager.career_history.len(), 1);
    let entry = &game.manager.career_history[0];
    assert_eq!(entry.team_id, "team1");
    assert_eq!(entry.matches, 2);
    assert_eq!(entry.wins, 2);
    assert_eq!(entry.best_league_position, Some(1));
}

#[test]
fn manager_career_history_entry_updated_on_second_season() {
    let mut game = make_completed_season_game();
    // Add pre-existing career history entry
    game.manager
        .career_history
        .push(domain::manager::ManagerCareerEntry {
            team_id: "team1".to_string(),
            team_name: "Test FC".to_string(),
            start_date: "2025-08-01".to_string(),
            end_date: None,
            matches: 10,
            wins: 5,
            draws: 3,
            losses: 2,
            best_league_position: Some(3),
        });

    process_end_of_season(&mut game);

    // Should update existing entry, not create new
    assert_eq!(game.manager.career_history.len(), 1);
    let entry = &game.manager.career_history[0];
    assert_eq!(entry.matches, 12); // 10 + 2
    assert_eq!(entry.wins, 7); // 5 + 2
    assert_eq!((entry.draws, entry.losses), (3, 2)); // a season without either adds none
    assert_eq!(entry.best_league_position, Some(1)); // improved from 3 to 1
}

// ---------------------------------------------------------------------------
// process_end_of_season — next season generation
// ---------------------------------------------------------------------------

#[test]
fn new_league_generated() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let league = game.league.as_ref().unwrap();
    assert_eq!(league.season, 2026, "Should be season 2026");
    assert!(
        !league.fixtures.is_empty(),
        "Should have fixtures for new season"
    );
    // All fixtures should be Scheduled
    assert!(
        league
            .fixtures
            .iter()
            .all(|f| f.status == FixtureStatus::Scheduled),
        "All new fixtures should be Scheduled"
    );
}

#[test]
fn board_objectives_cleared() {
    let mut game = make_completed_season_game();
    game.board_objectives.push(BoardObjective {
        id: "obj1".to_string(),
        objective_type: ObjectiveType::LeaguePosition,
        description: "Finish top 2".to_string(),
        target: 2,
        met: true,
    });

    process_end_of_season(&mut game);
    assert!(
        game.board_objectives.is_empty(),
        "Objectives should be cleared"
    );
}

#[test]
fn news_cleared() {
    let mut game = make_completed_season_game();
    game.news.push(domain::news::NewsArticle::new(
        "n1".to_string(),
        "Old news".to_string(),
        "...".to_string(),
        "Source".to_string(),
        "2025-01-01".to_string(),
        domain::news::NewsCategory::MatchReport,
    ));

    process_end_of_season(&mut game);
    assert!(
        game.news.iter().all(|article| article.id != "n1"),
        "Old news from the previous season should be cleared"
    );
    assert!(
        game.news
            .iter()
            .any(|article| article.category == domain::news::NewsCategory::SeasonPreview),
        "A season preview should be added for the new season"
    );
}

#[test]
fn season_awards_article_added_when_marquee_winners_exist() {
    // The fixture has a top scorer (Star, 20 goals) and a clear POTY (Star, 7.5 rating).
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let awards_article = game
        .news
        .iter()
        .find(|article| article.id == "season_awards_1");
    assert!(
        awards_article.is_some(),
        "A season awards article should be published when there are marquee winners"
    );
}

#[test]
fn season_awards_article_references_top_scorer_and_their_team() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let awards_article = game
        .news
        .iter()
        .find(|article| article.id == "season_awards_1")
        .expect("awards article should exist");

    assert_eq!(awards_article.body, "");
    assert_eq!(
        awards_article.headline_key.as_deref(),
        Some("be.news.seasonAwards.headline")
    );
    assert_eq!(
        awards_article.body_key.as_deref(),
        Some("be.news.seasonAwards.bodyBoth")
    );
    assert_eq!(
        awards_article.i18n_params.get("goldenBootWinner"),
        Some(&"Star".to_string())
    );
    assert_eq!(
        awards_article.i18n_params.get("goldenBootTeam"),
        Some(&"Test FC".to_string())
    );
    assert!(
        awards_article.player_ids.contains(&"p1".to_string()),
        "Awards article should link the winning player by id"
    );
    assert!(
        awards_article.team_ids.contains(&"team1".to_string()),
        "Awards article should link the winning club by id"
    );
}

#[test]
fn season_awards_article_not_added_when_no_marquee_winners() {
    // Strip players' goals and ratings so there's no Golden Boot / POTY winner.
    let mut game = make_completed_season_game();
    for player in game.players.iter_mut() {
        player.stats.goals = 0;
        player.stats.assists = 0;
        player.stats.avg_rating = 0.0;
        player
            .league_stats_by_competition
            .insert("league1".into(), player.stats.clone());
    }
    process_end_of_season(&mut game);

    assert!(
        game.news
            .iter()
            .all(|article| article.id != "season_awards_1"),
        "No awards article should be published when no marquee award has a winner"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — board message variations
// ---------------------------------------------------------------------------

#[test]
fn champion_gets_congratulatory_message() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .unwrap();
    assert_eq!(msg.body, "");
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.seasonReview.body.champion")
    );
}

#[test]
fn mid_table_gets_appropriate_message() {
    let mut game = make_completed_season_game();
    // Ten clubs, the user fifth: past the top four but still in the upper half.
    for i in 3..=10 {
        game.teams
            .push(make_team(&format!("team{i}"), &format!("Team{i} FC")));
    }
    if let Some(league) = &mut game.league {
        let mut standings = Vec::new();
        for (index, i) in (2..=10).enumerate() {
            standings.push(make_standing(
                &format!("team{i}"),
                27 - index as u32 * 3,
                0,
                0,
                20,
                10,
            ));
        }
        // The user finishes fifth of ten, between team5 and team6 on points.
        standings.insert(4, make_standing("team1", 16, 0, 0, 15, 12));
        league.standings = standings;
    }

    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.user_position, 5);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .expect("the board sends its season review");
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.seasonReview.body.midTable"),
        "fifth of ten should use the midTable body key"
    );
}

#[test]
fn bottom_half_gets_concerned_message() {
    let mut game = make_completed_season_game();
    // Add enough teams so that finishing last (10th of 10) triggers the disappointed branch
    // (user_position > 4 AND user_position > total_teams / 2)
    for i in 3..=10 {
        let tid = format!("team{}", i);
        game.teams.push(make_team(&tid, &format!("Team{} FC", i)));
    }
    if let Some(league) = &mut game.league {
        let mut standings = Vec::new();
        for i in 2..=10 {
            standings.push(make_standing(&format!("team{}", i), 10, 2, 6, 20, 15));
        }
        // team1 (user) finishes dead last
        standings.push(make_standing("team1", 0, 0, 18, 2, 40));
        league.standings = standings;
    }

    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .unwrap();
    assert_eq!(msg.body, "");
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.seasonReview.body.lowerHalf")
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — no league edge case
// ---------------------------------------------------------------------------

#[test]
fn no_league_returns_default_summary() {
    let date = Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
    // No league set
    let summary = process_end_of_season(&mut game);
    assert_eq!(summary.season, 0);
    assert!(summary.league_name.is_empty());
}

// ---------------------------------------------------------------------------
// process_end_of_season — satisfaction adjustment
// ---------------------------------------------------------------------------

#[test]
fn satisfaction_unchanged_when_the_board_set_no_objectives() {
    let mut game = make_completed_season_game();
    let initial_sat = game.manager.satisfaction;
    process_end_of_season(&mut game);
    // Nothing was asked, so nothing is judged.
    assert_eq!(game.manager.satisfaction, initial_sat);
}

/// The test above only ever proved that zero objectives change nothing, which
/// stays true however the evaluation is written — replacing the whole
/// calculation with a constant zero left it green. These two do the judging.
#[test]
fn meeting_the_board_objective_raises_satisfaction() {
    let mut game = make_completed_season_game();
    // Objectives are marked met as the season runs; the rollover only counts
    // them. team1 won its league, so this one came in.
    game.board_objectives.push(BoardObjective {
        id: "obj1".to_string(),
        objective_type: ObjectiveType::LeaguePosition,
        description: "Finish top 2".to_string(),
        target: 2,
        met: true,
    });
    let before = game.manager.satisfaction;

    process_end_of_season(&mut game);

    assert!(
        game.manager.satisfaction > before,
        "meeting the board's objective should please them: {before} -> {}",
        game.manager.satisfaction
    );
}

#[test]
fn missing_the_board_objective_lowers_satisfaction() {
    let mut game = make_completed_season_game();
    // team2 finished bottom, so the title the board asked for was not won.
    game.manager.hire("team2".to_string());
    game.board_objectives.push(BoardObjective {
        id: "obj1".to_string(),
        objective_type: ObjectiveType::LeaguePosition,
        description: "Win the league".to_string(),
        target: 1,
        met: false,
    });
    let before = game.manager.satisfaction;

    process_end_of_season(&mut game);

    assert!(
        game.manager.satisfaction < before,
        "team2 finished last and missed the objective: {before} -> {}",
        game.manager.satisfaction
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — team reputation updates
// ---------------------------------------------------------------------------

#[test]
fn low_reputation_champion_gains_reputation() {
    let mut game = make_completed_season_game();
    let team1 = game
        .teams
        .iter_mut()
        .find(|team| team.id == "team1")
        .unwrap();
    team1.reputation = 320;

    process_end_of_season(&mut game);

    let updated_team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert!(
        updated_team1.reputation > 320,
        "Winning the league from a low starting reputation should raise the club's reputation"
    );
}

#[test]
fn high_reputation_bottom_side_loses_reputation() {
    let mut game = make_completed_season_game();

    for i in 3..=10 {
        let team_id = format!("team{}", i);
        game.teams
            .push(make_team(&team_id, &format!("Team{} FC", i)));
    }

    game.teams
        .iter_mut()
        .find(|team| team.id == "team1")
        .unwrap()
        .reputation = 860;

    if let Some(league) = &mut game.league {
        let mut standings = Vec::new();
        for i in 2..=10 {
            standings.push(make_standing(&format!("team{}", i), 10, 2, 6, 20, 15));
        }
        standings.push(make_standing("team1", 0, 0, 18, 2, 40));
        league.standings = standings;
    }

    process_end_of_season(&mut game);

    let updated_team1 = game.teams.iter().find(|team| team.id == "team1").unwrap();
    assert!(
        updated_team1.reputation < 860,
        "A high-reputation club collapsing to the bottom should lose reputation"
    );
}

// ---------------------------------------------------------------------------
// is_season_complete — season not started guard
// ---------------------------------------------------------------------------

#[test]
fn season_not_complete_when_no_matches_played() {
    // Full schedule exists but all fixtures are still Scheduled (preseason state).
    // is_season_complete must return false — we must not trigger end-of-season
    // processing before the campaign has even begun.
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        for fixture in &mut league.fixtures {
            fixture.status = FixtureStatus::Scheduled;
            fixture.result = None;
        }
        for standing in &mut league.standings {
            *standing = StandingEntry::new(standing.team_id.clone());
        }
    }
    assert!(
        !is_season_complete(&game),
        "Season with no matches played must not be considered complete"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — message dates
// ---------------------------------------------------------------------------

#[test]
fn season_end_messages_dated_on_last_fixture_date() {
    // make_completed_season_game() sets the clock to 2026-05-20 but both league
    // fixtures are dated 2025-06-01 (see make_completed_fixture).
    // End-of-season messages must be dated on the last completed fixture date
    // (2025-06-01), not on the clock date (2026-05-20).
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let board_msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .expect("season_end_1 message must be present");
    assert_eq!(
        board_msg.date, "2025-06-01",
        "Board review must be dated on the last fixture date, not the clock date"
    );

    let payout_msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_payout_1")
        .expect("season_payout_1 message must be present");
    assert_eq!(
        payout_msg.date, "2025-06-01",
        "Prize money message must be dated on the last fixture date"
    );

    let schedule_msg = game
        .messages
        .iter()
        .find(|m| m.id == "new_season_2")
        .expect("new_season_2 message must be present");
    assert_eq!(
        schedule_msg.date, "2025-06-01",
        "New season schedule message must be dated on the last fixture date"
    );
}

// ---------------------------------------------------------------------------
// process_end_of_season — i18n on end-of-season messages
// ---------------------------------------------------------------------------

#[test]
fn season_end_board_message_has_i18n_keys() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .expect("season_end_1 message must be present");

    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.seasonReview.subject"),
        "Board review subject must have i18n key"
    );
    assert!(
        msg.body_key.is_some(),
        "Board review body must have an i18n key"
    );
    assert!(
        msg.body_key
            .as_deref()
            .unwrap_or("")
            .starts_with("be.msg.seasonReview.body."),
        "Board review body key must be under be.msg.seasonReview.body, got: {:?}",
        msg.body_key
    );
    assert!(
        msg.i18n_params.contains_key("season"),
        "Board review i18n params must contain 'season'"
    );
    assert!(
        msg.i18n_params.contains_key("team"),
        "Board review i18n params must contain 'team'"
    );
    assert!(
        msg.i18n_params.contains_key("points"),
        "Board review i18n params must contain 'points'"
    );
}

#[test]
fn season_end_board_message_has_sender_i18n() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .expect("season_end_1 message must be present");

    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.boardOfDirectors"),
        "Board review sender must have i18n key"
    );
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.chairman"),
        "Board review sender role must have i18n key"
    );
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn season_end_new_schedule_message_has_i18n_keys() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "new_season_2")
        .expect("new_season_2 message must be present");

    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.newSeasonSchedule.subject"),
        "New season schedule subject must have i18n key"
    );
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.newSeasonSchedule.body"),
        "New season schedule body must have i18n key"
    );
    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.leagueOffice"),
        "New season schedule sender must have i18n key"
    );
    assert_eq!(
        msg.sender_role_key.as_deref(),
        Some("be.role.competitionSecretary"),
        "New season schedule sender role must have i18n key"
    );
    assert!(
        msg.i18n_params.contains_key("season"),
        "New season schedule i18n params must contain 'season'"
    );
    assert_eq!(msg.body, "");
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
}

#[test]
fn season_end_payout_message_has_i18n_keys() {
    let mut game = make_completed_season_game();
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_payout_1")
        .expect("season_payout_1 message must be present");

    assert_eq!(msg.subject, "");
    assert_eq!(msg.body, "");
    assert_eq!(msg.sender, "");
    assert_eq!(msg.sender_role, "");
    assert_eq!(
        msg.subject_key.as_deref(),
        Some("be.msg.seasonPayout.subject")
    );
    assert_eq!(msg.body_key.as_deref(), Some("be.msg.seasonPayout.body"));
    assert_eq!(
        msg.sender_key.as_deref(),
        Some("be.sender.boardOfDirectors")
    );
    assert_eq!(msg.sender_role_key.as_deref(), Some("be.role.chairman"));
    assert_eq!(msg.i18n_params.get("season"), Some(&"1".to_string()));
    assert_eq!(msg.i18n_params.get("position"), Some(&"1".to_string()));
}

#[test]
fn season_end_board_message_top_four_uses_correct_body_key() {
    let mut game = make_completed_season_game();
    // Make team1 finish 2nd (top-4 branch)
    if let Some(league) = &mut game.league {
        league.standings = vec![
            make_standing("team2", 2, 0, 0, 3, 1),
            make_standing("team1", 0, 0, 2, 1, 3),
        ];
    }
    process_end_of_season(&mut game);

    let msg = game
        .messages
        .iter()
        .find(|m| m.id == "season_end_1")
        .unwrap();
    assert_eq!(
        msg.body_key.as_deref(),
        Some("be.msg.seasonReview.body.topFour"),
        "2nd-place finish should use topFour body key"
    );
    assert!(
        msg.i18n_params.get("position") == Some(&"2".to_string()),
        "topFour position must be the bare number 2"
    );
    // The place is a bare number: the frontend writes it as an ordinal in the player's language, so
    // no English suffix ("2nd", "21th") is sent from here.
    assert!(
        !msg.i18n_params.contains_key("suffix"),
        "topFour must not carry an English ordinal suffix"
    );
}
