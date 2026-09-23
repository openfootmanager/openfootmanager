//! Builders for the training ground: players of a chosen age, clubs, staff
//! with a chosen coaching and physio rating, and a small world to train in.

use super::*;

pub(crate) fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
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
    }
}

pub(crate) fn make_player(id: &str, name: &str, team_id: &str, dob: &str) -> Player {
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        dob.to_string(),
        "GB".to_string(),
        Position::Midfielder,
        default_attrs(),
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 80;
    p
}

pub(crate) fn make_team(id: &str, name: &str) -> Team {
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

pub(crate) fn make_staff(
    id: &str,
    team_id: &str,
    role: StaffRole,
    coaching: u8,
    physio: u8,
) -> Staff {
    let mut s = Staff::new(
        id.to_string(),
        "Staff".to_string(),
        id.to_string(),
        "1980-01-01".to_string(),
        role,
        StaffAttributes {
            coaching,
            judging_ability: 50,
            judging_potential: 50,
            physiotherapy: physio,
        },
    );
    s.team_id = Some(team_id.to_string());
    s.nationality = "GB".to_string();
    s
}

pub(crate) fn make_game() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap(); // Monday
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let mut team1 = make_team("team1", "Test FC");
    team1.training_focus = TrainingFocus::Physical;
    team1.training_intensity = TrainingIntensity::Medium;
    team1.training_schedule = TrainingSchedule::Balanced;

    // Young player (age ~21)
    let p1 = make_player("p1", "Young", "team1", "2004-03-15");
    // Prime player (age ~27)
    let p2 = make_player("p2", "Prime", "team1", "1998-06-10");
    // Old player (age ~35)
    let p3 = make_player("p3", "Veteran", "team1", "1990-01-01");

    let coach = make_staff("coach1", "team1", StaffRole::Coach, 80, 30);
    let physio = make_staff("physio1", "team1", StaffRole::Physio, 30, 80);

    Game::new(
        clock,
        manager,
        vec![team1],
        vec![p1, p2, p3],
        vec![coach, physio],
        vec![],
    )
}

/// An AI club matching the user's club on everything `process_training` reads:
/// training settings and staff. Only the manager differs.
pub(crate) fn add_mirror_ai_team(game: &mut Game) {
    let mut team2 = make_team("team2", "AI FC");
    team2.training_focus = TrainingFocus::Physical;
    team2.training_intensity = TrainingIntensity::Medium;
    team2.training_schedule = TrainingSchedule::Balanced;
    game.teams.push(team2);
    game.staff
        .push(make_staff("coach2", "team2", StaffRole::Coach, 80, 30));
    game.staff
        .push(make_staff("physio2", "team2", StaffRole::Physio, 30, 80));
}

/// Give the user's club a single fixture `days` from the clock's current date.
pub(crate) fn schedule_user_fixture_in(game: &mut Game, days: i64) {
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};

    let date = (game.clock.current_date + chrono::Duration::days(days))
        .format("%Y-%m-%d")
        .to_string();
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date,
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
            ..Default::default()
        }],
        standings: vec![
            StandingEntry::new("team1".to_string()),
            StandingEntry::new("team2".to_string()),
        ],
        ..Default::default()
    });
}

pub(crate) fn condition_after(
    schedule: TrainingSchedule,
    focus: TrainingFocus,
    weekday: u32,
) -> u8 {
    let mut game = make_game();
    game.teams[0].training_schedule = schedule;
    game.teams[0].training_focus = focus;
    for p in game.players.iter_mut() {
        p.condition = 50;
    }
    training::process_training(&mut game, weekday);
    game.players[0].condition
}
