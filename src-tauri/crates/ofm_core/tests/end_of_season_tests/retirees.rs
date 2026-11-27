use super::*;

// ---------------------------------------------------------------------------
// Retiree conversion and pool replenishment
// ---------------------------------------------------------------------------

fn make_retired_player(id: &str) -> domain::player::Player {
    use domain::player::{CareerEntry, PlayerAttributes, Position};
    let attrs = PlayerAttributes {
        pace: 55,
        stamina: 55,
        strength: 55,
        agility: 55,
        passing: 70,
        shooting: 60,
        tackling: 65,
        dribbling: 60,
        defending: 65,
        positioning: 72,
        vision: 74,
        decisions: 70,
        composure: 65,
        aggression: 50,
        teamwork: 68,
        leadership: 60,
        handling: 20,
        reflexes: 20,
        aerial: 55,
    };
    let mut p = domain::player::Player::new(
        id.to_string(),
        format!("{} Ret", id),
        format!("Full {}", id),
        "1988-01-01".to_string(),
        "ENG".to_string(),
        Position::Midfielder,
        attrs,
    );
    p.team_id = None;
    p.retired = true;
    p.career.push(CareerEntry {
        season: 1,
        team_id: "team1".to_string(),
        team_name: "Test FC".to_string(),
        appearances: 30,
        goals: 5,
        assists: 8,
        ..Default::default()
    });
    p
}

#[test]
fn retired_player_creates_manager_candidate_with_deterministic_id() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("retiree1"));

    process_end_of_season(&mut game);

    let mgr_id = "mgr_retired_retiree1";
    assert!(
        game.managers.iter().any(|m| m.id == mgr_id),
        "unemployed manager with id '{}' should exist after season end",
        mgr_id
    );
}

#[test]
fn retired_player_creates_scout_candidate_with_deterministic_id() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("retiree2"));

    process_end_of_season(&mut game);

    let scout_id = "staff_retired_scout_retiree2";
    assert!(
        game.staff.iter().any(|s| s.id == scout_id
            && matches!(s.role, domain::staff::StaffRole::Scout)
            && s.team_id.is_none()),
        "unattached scout '{}' should exist after season end",
        scout_id
    );
}

/// Given a retired player converted to an unattached scout, when the season
/// ends, then he is paid the closed scout formula at unattached reputation.
#[test]
fn retired_player_scout_receives_the_closed_weekly_wage() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("retiree_wage"));

    process_end_of_season(&mut game);

    let scout = game
        .staff
        .iter()
        .find(|s| s.id == "staff_retired_scout_retiree_wage")
        .expect("retired scout");
    // Season-end aging can move the player's attributes before archival, so the
    // closed scout formula is applied to the judging scores actually stored on
    // the unattached scout, at unattached reputation 500.
    let judging = scout
        .attributes
        .judging_ability
        .max(scout.attributes.judging_potential);
    assert_eq!(scout.wage, 1_000 + u32::from(judging) * 20 + 500 * 2);
    assert_ne!(scout.wage, 0);
}

#[test]
fn retiree_conversion_is_idempotent() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("idem1"));

    process_end_of_season(&mut game);
    // Build a fresh completed-season state and re-trigger (simulates second season)
    // Manually reset enough state so process_end_of_season can run again
    if let Some(league) = &mut game.league {
        // Mark new season fixtures as completed for the guard check
        for f in league.fixtures.iter_mut() {
            f.status = domain::league::FixtureStatus::Completed;
            f.result = Some(domain::league::MatchResult {
                home_goals: 1,
                away_goals: 0,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
                home_penalties: None,
                away_penalties: None,
            });
        }
    }
    process_end_of_season(&mut game);

    let mgr_count = game
        .managers
        .iter()
        .filter(|m| m.id == "mgr_retired_idem1")
        .count();
    assert_eq!(
        mgr_count, 1,
        "duplicate manager must not be created on second call"
    );

    let scout_count = game
        .staff
        .iter()
        .filter(|s| s.id == "staff_retired_scout_idem1")
        .count();
    assert_eq!(
        scout_count, 1,
        "duplicate scout must not be created on second call"
    );
}

/// Given one player who retired this season and one archived seasons ago, both with careers,
/// When the season ends
/// Then only this season's retiree becomes a manager and a scout candidate.
#[test]
fn only_this_seasons_retirees_become_candidates() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("recent"));
    let long_ago = make_retired_player("longago");
    game.retired_players
        .push(domain::retired_player::RetiredPlayer::from_player(
            &long_ago,
            "2001-06-30",
            2001,
        ));

    process_end_of_season(&mut game);

    assert!(game.managers.iter().any(|m| m.id == "mgr_retired_recent"));
    assert!(
        game.staff
            .iter()
            .any(|s| s.id == "staff_retired_scout_recent")
    );
    assert!(game.managers.iter().all(|m| m.id != "mgr_retired_longago"));
    assert!(
        game.staff
            .iter()
            .all(|s| s.id != "staff_retired_scout_longago")
    );
}

/// Given a retired player whose manager conversion has since retired into the archive
/// When the season ends again
/// Then the conversion does not create the manager a second time.
#[test]
fn an_archived_retiree_manager_is_not_converted_again() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("archived1"));
    let mut archived = domain::manager::Manager::new(
        "mgr_retired_archived1".to_string(),
        "Old".to_string(),
        "Boss".to_string(),
        "1950-01-01".to_string(),
        "England".to_string(),
    );
    archived.reputation = 500;
    game.retired_managers.push(archived);

    process_end_of_season(&mut game);

    let records = game
        .retired_managers
        .iter()
        .filter(|m| m.id == "mgr_retired_archived1")
        .count();
    assert_eq!(records, 1, "the retiree was converted a second time");
    assert!(
        game.managers
            .iter()
            .all(|m| m.id != "mgr_retired_archived1")
    );
}

/// Given an unemployed manager past retirement age
/// When the season ends
/// Then he leaves the active pool for the archive, and the market is topped back up.
#[test]
fn an_unemployed_manager_past_retirement_age_leaves_the_pool_at_season_end() {
    let mut game = make_completed_season_game();
    let mut veteran = domain::manager::Manager::new(
        "veteran".to_string(),
        "Old".to_string(),
        "Boss".to_string(),
        "1900-01-01".to_string(),
        "England".to_string(),
    );
    veteran.reputation = 900;
    game.managers.push(veteran);

    process_end_of_season(&mut game);

    assert!(game.managers.iter().all(|m| m.id != "veteran"));
    assert!(game.retired_managers.iter().any(|m| m.id == "veteran"));
}

#[test]
fn retired_player_with_no_career_is_not_converted() {
    let mut game = make_completed_season_game();
    let mut p = make_retired_player("no_career");
    p.career.clear(); // ineligible
    game.players.push(p);

    process_end_of_season(&mut game);

    assert!(
        !game
            .managers
            .iter()
            .any(|m| m.id == "mgr_retired_no_career"),
        "players with no career entries must not become manager candidates"
    );
}

#[test]
fn manager_pool_is_topped_up_to_floor_after_season() {
    let mut game = make_completed_season_game();
    let team_count = game.teams.len(); // 2 teams → floor = 4

    process_end_of_season(&mut game);

    let user_mgr_id = game.manager.id.clone();
    let unemployed = game
        .managers
        .iter()
        .filter(|m| m.id != user_mgr_id && m.team_id.is_none())
        .count();
    assert!(
        unemployed >= team_count * 2,
        "unemployed manager pool ({}) should be >= floor ({})",
        unemployed,
        team_count * 2
    );
}

#[test]
fn scout_pool_is_topped_up_to_floor_after_season() {
    let mut game = make_completed_season_game();
    let team_count = game.teams.len(); // 2 teams → floor = 4

    process_end_of_season(&mut game);

    let unemployed_scouts = game
        .staff
        .iter()
        .filter(|s| s.team_id.is_none() && matches!(s.role, domain::staff::StaffRole::Scout))
        .count();
    assert!(
        unemployed_scouts >= team_count * 2,
        "unemployed scout pool ({}) should be >= floor ({})",
        unemployed_scouts,
        team_count * 2
    );
}

/// A player who retired this season, archived at `ovr` after thirteen seasons of thirty games:
/// the career a generated world's history gives. Archived rather than pushed into `game.players`,
/// because the season end re-derives a squad player's rating from his attributes before he retires.
fn retired_this_season_after_thirteen_seasons(
    game: &Game,
    id: &str,
    ovr: u8,
) -> domain::retired_player::RetiredPlayer {
    let mut player = make_retired_player(id);
    player.ovr = ovr;
    player.career = (1..=13)
        .map(|season| domain::player::CareerEntry {
            season,
            team_id: "team1".to_string(),
            team_name: "Test FC".to_string(),
            appearances: 30,
            goals: 0,
            assists: 0,
            ..Default::default()
        })
        .collect();
    domain::retired_player::RetiredPlayer::from_player(
        &player,
        "2026-05-20",
        game.league.as_ref().map_or(0, |league| league.season),
    )
}

/// Given a star and a squad player who retired this season after thirteen seasons each, rated 85
///      and 62
/// When the season ends and they become manager candidates
/// Then they start at 681 and 382, on the 200-700 scale every manager is created on and apart by
///      their ability (#1160: both started at 900, above every manager the world opened with)
#[test]
fn retirees_start_managing_on_the_created_scale_by_their_ability() {
    let mut game = make_completed_season_game();
    let star = retired_this_season_after_thirteen_seasons(&game, "star", 85);
    let squad = retired_this_season_after_thirteen_seasons(&game, "squad", 62);
    game.retired_players.extend([star, squad]);

    process_end_of_season(&mut game);

    let reputation_of = |id: &str| {
        game.managers
            .iter()
            .find(|m| m.id == id)
            .map(|m| (m.reputation, m.satisfaction, m.fan_approval))
    };
    assert_eq!(reputation_of("mgr_retired_star"), Some((681, 50, 50)));
    assert_eq!(reputation_of("mgr_retired_squad"), Some((382, 50, 50)));
}

/// Given a Belgian retiree, when the season ends, then his scout identity is ready immediately.
#[test]
fn created_staff_have_football_identity_from_retirees() {
    let mut game = make_completed_season_game();
    let mut retired = retired_this_season_after_thirteen_seasons(&game, "identity", 70);
    retired.nationality = "BE".into();
    game.retired_players.push(retired);

    process_end_of_season(&mut game);

    let scout = game
        .staff
        .iter()
        .find(|s| s.id == "staff_retired_scout_identity")
        .unwrap();
    assert_eq!(scout.football_nation, "BE");
    assert_eq!(scout.birth_country.as_deref(), Some("BE"));
}

#[test]
fn retiree_scout_judging_attributes_derived_from_player_attrs() {
    let mut game = make_completed_season_game();
    game.players.push(make_retired_player("attr_test"));

    process_end_of_season(&mut game);

    let scout = game
        .staff
        .iter()
        .find(|s| s.id == "staff_retired_scout_attr_test")
        .expect("scout should exist");
    // The scout is built from the attributes the player retired with, which a last season of
    // ageing has already moved, so the expectation is read off the archived player.
    let retired = game
        .retired_players
        .iter()
        .find(|record| record.id == "attr_test")
        .expect("the player retired into the archive")
        .to_player();
    let (vision, decisions) = (
        u16::from(retired.attributes.vision),
        u16::from(retired.attributes.decisions),
    );
    let (positioning, teamwork) = (
        u16::from(retired.attributes.positioning),
        u16::from(retired.attributes.teamwork),
    );
    assert_eq!(
        u16::from(scout.attributes.judging_ability),
        (vision + decisions) / 2
    );
    assert_eq!(
        u16::from(scout.attributes.judging_potential),
        (positioning + teamwork) / 2
    );
}

#[test]
fn process_end_of_season_refreshes_transfer_budget_from_finance() {
    let mut game = make_completed_season_game();
    // Simulate a season where the user emptied their transfer envelope and
    // another club still has an unspent chunk. Both should be reset to a
    // fresh envelope sized to post-prize finance — the "unspent chunk" is
    // deliberately blown away to match the real-football convention of a
    // new board grant each season.
    let team1_idx = game.teams.iter().position(|t| t.id == "team1").unwrap();
    game.teams[team1_idx].finance = 10_000_000;
    game.teams[team1_idx].transfer_budget = 0;
    let team2_idx = game.teams.iter().position(|t| t.id == "team2").unwrap();
    game.teams[team2_idx].finance = 2_000_000;
    game.teams[team2_idx].transfer_budget = 750_000;

    process_end_of_season(&mut game);

    // Formula matches worldgen (generator/mod.rs:543): 15% of finance.
    // We assert the invariant against post-prize finance so the test stays
    // valid regardless of how much prize money each team receives.
    let team1 = game.teams.iter().find(|t| t.id == "team1").unwrap();
    let team2 = game.teams.iter().find(|t| t.id == "team2").unwrap();
    assert_eq!(team1.transfer_budget, (team1.finance as f64 * 0.15) as i64);
    assert_eq!(team2.transfer_budget, (team2.finance as f64 * 0.15) as i64);
    // Team1 went in with an empty envelope; the refill must have run.
    assert!(team1.transfer_budget > 0);
}
