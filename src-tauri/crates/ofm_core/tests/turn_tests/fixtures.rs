//! Builders for the worlds `turn_tests` advances through: players, clubs,
//! staff, fixtures and hand-written match reports.
//!
//! They live apart from the tests so the test file reads as a list of claims
//! about a day's advance rather than as the scaffolding needed to make one.

use super::*;

pub(crate) fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 60,
        stamina: 60,
        strength: 60,
        agility: 60,
        passing: 60,
        shooting: 60,
        tackling: 60,
        dribbling: 60,
        defending: 60,
        positioning: 60,
        vision: 60,
        decisions: 60,
        composure: 60,
        aggression: 60,
        teamwork: 60,
        leadership: 60,
        handling: 30,
        reflexes: 30,
        aerial: 60,
    }
}

pub(crate) fn gk_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 40,
        stamina: 50,
        strength: 60,
        agility: 70,
        passing: 40,
        shooting: 20,
        tackling: 20,
        dribbling: 20,
        defending: 30,
        positioning: 70,
        vision: 50,
        decisions: 60,
        composure: 70,
        aggression: 30,
        teamwork: 60,
        leadership: 50,
        handling: 80,
        reflexes: 80,
        aerial: 70,
    }
}

pub(crate) fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let attrs = if pos == Position::Goalkeeper {
        gk_attrs()
    } else {
        default_attrs()
    };
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        name.to_string(),
        "1995-01-01".to_string(),
        "England".to_string(),
        pos,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 100;
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
    first_name: &str,
    last_name: &str,
) -> Staff {
    let mut staff = Staff::new(
        id.to_string(),
        first_name.to_string(),
        last_name.to_string(),
        "1980-01-01".to_string(),
        role,
        StaffAttributes {
            coaching: 60,
            judging_ability: 60,
            judging_potential: 60,
            physiotherapy: 20,
        },
    );
    staff.nationality = "England".to_string();
    staff.team_id = Some(team_id.to_string());
    staff
}

pub(crate) fn make_squad(team_id: &str, prefix: &str) -> Vec<Player> {
    let mut players = Vec::new();
    // 1 GK
    players.push(make_player(
        &format!("{}_gk", prefix),
        &format!("{} GK", prefix),
        team_id,
        Position::Goalkeeper,
    ));
    // 4 DEF
    for i in 0..4 {
        players.push(make_player(
            &format!("{}_def{}", prefix, i),
            &format!("{} Def{}", prefix, i),
            team_id,
            Position::Defender,
        ));
    }
    // 4 MID
    for i in 0..4 {
        players.push(make_player(
            &format!("{}_mid{}", prefix, i),
            &format!("{} Mid{}", prefix, i),
            team_id,
            Position::Midfielder,
        ));
    }
    // 2 FWD
    for i in 0..2 {
        players.push(make_player(
            &format!("{}_fwd{}", prefix, i),
            &format!("{} Fwd{}", prefix, i),
            team_id,
            Position::Forward,
        ));
    }
    players
}

pub(crate) fn make_game_with_match() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Test FC");
    let team2 = make_team("team2", "Rival FC");
    let mut players = make_squad("team1", "t1");
    players.extend(make_squad("team2", "t2"));

    let today = date.format("%Y-%m-%d").to_string();
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: today,
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
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };

    let mut game = Game::new(clock, manager, vec![team1, team2], players, vec![], vec![]);
    game.league = Some(league);
    game
}

/// A 22-man squad: two players for each slot of a 4-4-2, so there is a real
/// bench to leave out.
pub(crate) fn make_deep_squad(team_id: &str, prefix: &str) -> Vec<Player> {
    let shape = [
        (Position::Goalkeeper, 1),
        (Position::Defender, 4),
        (Position::Midfielder, 4),
        (Position::Forward, 2),
    ];
    let mut players = Vec::with_capacity(22);
    for tier in ["a", "b"] {
        for (position, count) in &shape {
            for i in 0..*count {
                players.push(make_player(
                    &format!("{prefix}_{tier}_{position:?}{i}"),
                    &format!("{prefix} {tier}{position:?}{i}"),
                    team_id,
                    position.clone(),
                ));
            }
        }
    }
    players
}

pub(crate) fn game_with_deep_squads() -> Game {
    let mut game = make_game_with_match();
    game.players = make_deep_squad("team1", "t1");
    game.players.extend(make_deep_squad("team2", "t2"));
    game
}

/// The eleven a club actually put out, by the field that records it.
pub(crate) fn fielded_count(game: &Game, team_id: &str) -> usize {
    game.players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.stats.appearances > 0)
        .count()
}

/// A club with a squad and no fixture, parked on the given condition.
pub(crate) fn add_idle_club(game: &mut Game, team_id: &str, condition: u8) {
    game.teams.push(make_team(team_id, "Idle FC"));
    let mut squad = make_squad(team_id, team_id);
    for player in squad.iter_mut() {
        player.condition = condition;
    }
    game.players.extend(squad);
}

pub(crate) fn condition_of(game: &Game, team_id: &str) -> Vec<u8> {
    game.players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .map(|p| p.condition)
        .collect()
}

pub(crate) fn make_game_without_match_today() -> Game {
    let mut game = make_game_with_match();
    if let Some(league) = &mut game.league {
        league.fixtures[0].date = "2025-06-16".to_string();
    }
    game
}

pub(crate) fn empty_report(home_goals: u8, away_goals: u8) -> MatchReport {
    MatchReport {
        home_goals,
        away_goals,
        home_stats: TeamStats::default(),
        away_stats: TeamStats::default(),
        events: vec![],
        goals: vec![],
        player_stats: HashMap::new(),
        home_possession: 50.0,
        total_minutes: 90,
        home_penalties: None,
        away_penalties: None,
    }
}

pub(crate) fn report_with_scorer(
    home_goals: u8,
    away_goals: u8,
    scorer_id: &str,
    side: Side,
) -> MatchReport {
    let mut player_stats = HashMap::new();
    player_stats.insert(
        scorer_id.to_string(),
        PlayerMatchStats {
            minutes_played: 90,
            goals: if side == Side::Home {
                home_goals
            } else {
                away_goals
            },
            assists: 0,
            shots: 3,
            shots_on_target: 2,
            passes_completed: 30,
            passes_attempted: 35,
            tackles_won: 2,
            interceptions: 1,
            fouls_committed: 1,
            yellow_cards: 0,
            red_cards: 0,
            rating: 7.5,
        },
    );
    let goals = (0..home_goals)
        .map(|i| GoalDetail {
            minute: 10 + i * 20,
            scorer_id: if side == Side::Home {
                scorer_id.to_string()
            } else {
                "other".to_string()
            },
            assist_id: None,
            goal_source: engine::report::GoalSource::OpenPlay,
            side: Side::Home,
        })
        .chain((0..away_goals).map(|i| GoalDetail {
            minute: 15 + i * 20,
            scorer_id: if side == Side::Away {
                scorer_id.to_string()
            } else {
                "other".to_string()
            },
            assist_id: None,
            goal_source: engine::report::GoalSource::OpenPlay,
            side: Side::Away,
        }))
        .collect();

    MatchReport {
        home_goals,
        away_goals,
        home_stats: TeamStats::default(),
        away_stats: TeamStats::default(),
        events: vec![],
        goals,
        player_stats,
        home_possession: 55.0,
        total_minutes: 90,
        home_penalties: None,
        away_penalties: None,
    }
}

/// Creates a match report where all 22 players played the full 90 minutes.
/// Use this for stamina depletion tests.
pub(crate) fn full_squad_report(home_goals: u8, away_goals: u8) -> MatchReport {
    let prefixes = ["t1_gk", "t2_gk"];
    let mut player_stats: HashMap<String, PlayerMatchStats> = HashMap::new();
    // Add GKs
    for prefix in &prefixes {
        player_stats.insert(
            prefix.to_string(),
            PlayerMatchStats {
                minutes_played: 90,
                ..Default::default()
            },
        );
    }
    // Add outfield players
    for prefix in ["t1", "t2"] {
        for i in 0..4 {
            player_stats.insert(
                format!("{}_def{}", prefix, i),
                PlayerMatchStats {
                    minutes_played: 90,
                    ..Default::default()
                },
            );
            player_stats.insert(
                format!("{}_mid{}", prefix, i),
                PlayerMatchStats {
                    minutes_played: 90,
                    ..Default::default()
                },
            );
        }
        for i in 0..2 {
            player_stats.insert(
                format!("{}_fwd{}", prefix, i),
                PlayerMatchStats {
                    minutes_played: 90,
                    ..Default::default()
                },
            );
        }
    }
    MatchReport {
        home_goals,
        away_goals,
        home_stats: TeamStats::default(),
        away_stats: TeamStats::default(),
        events: vec![],
        goals: vec![],
        player_stats,
        home_possession: 50.0,
        total_minutes: 90,
        home_penalties: None,
        away_penalties: None,
    }
}

pub(crate) fn make_round_summary_game() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Leaders FC");
    let team2 = make_team("team2", "Underdogs FC");
    let team3 = make_team("team3", "Chargers FC");
    let team4 = make_team("team4", "City FC");

    let mut players = make_squad("team1", "t1");
    players.extend(make_squad("team2", "t2"));
    players.extend(make_squad("team3", "t3"));
    players.extend(make_squad("team4", "t4"));

    let today = date.format("%Y-%m-%d").to_string();
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![
            Fixture {
                id: "fix1".to_string(),
                matchday: 7,
                date: today.clone(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(domain::league::MatchResult {
                    home_goals: 0,
                    away_goals: 1,
                    home_scorers: vec![],
                    away_scorers: vec![domain::league::GoalEvent {
                        player_id: "t2_fwd0".to_string(),
                        minute: 77,
                    }],
                    report: None,
                    home_penalties: None,
                    away_penalties: None,
                }),
                ..Default::default()
            },
            Fixture {
                id: "fix2".to_string(),
                matchday: 7,
                date: today,
                home_team_id: "team3".to_string(),
                away_team_id: "team4".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(domain::league::MatchResult {
                    home_goals: 2,
                    away_goals: 0,
                    home_scorers: vec![
                        domain::league::GoalEvent {
                            player_id: "t3_fwd0".to_string(),
                            minute: 20,
                        },
                        domain::league::GoalEvent {
                            player_id: "t3_fwd0".to_string(),
                            minute: 72,
                        },
                    ],
                    away_scorers: vec![],
                    report: None,
                    home_penalties: None,
                    away_penalties: None,
                }),
                ..Default::default()
            },
        ],
        standings: vec![
            standing_entry("team1", 11, 30, 21, 11),
            standing_entry("team2", 11, 23, 21, 11),
            standing_entry("team3", 11, 31, 18, 8),
            standing_entry("team4", 11, 27, 19, 14),
        ],
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };

    let mut game = Game::new(
        clock,
        manager,
        vec![team1, team2, team3, team4],
        players,
        vec![],
        vec![],
    );
    game.league = Some(league);

    set_team_overall(&mut game, "team1", 90);
    set_team_overall(&mut game, "team2", 50);
    set_team_overall(&mut game, "team3", 74);
    set_team_overall(&mut game, "team4", 72);

    game.players
        .iter_mut()
        .for_each(|player| match player.id.as_str() {
            "t1_fwd0" => player.stats.goals = 5,
            "t2_fwd0" => player.stats.goals = 3,
            "t3_fwd0" => player.stats.goals = 6,
            _ => {}
        });

    game
}

pub(crate) fn standing_entry(
    team_id: &str,
    played: u32,
    points: u32,
    goals_for: u32,
    goals_against: u32,
) -> StandingEntry {
    StandingEntry {
        team_id: team_id.to_string(),
        played,
        won: 0,
        drawn: 0,
        lost: 0,
        goals_for,
        goals_against,
        points,
    }
}

pub(crate) fn set_team_overall(game: &mut Game, team_id: &str, overall: u8) {
    game.players
        .iter_mut()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .for_each(|player| set_player_overall(player, overall));
}

pub(crate) fn set_player_overall(player: &mut Player, overall: u8) {
    player.attributes.pace = overall;
    player.attributes.stamina = overall;
    player.attributes.strength = overall;
    player.attributes.passing = overall;
    player.attributes.shooting = overall;
    player.attributes.tackling = overall;
    player.attributes.dribbling = overall;
    player.attributes.defending = overall;
    player.attributes.positioning = overall;
    player.attributes.vision = overall;
    player.attributes.decisions = overall;
}

pub(crate) fn previous_round_standings() -> Vec<StandingEntry> {
    vec![
        standing_entry("team1", 10, 30, 30, 20),
        standing_entry("team3", 10, 28, 29, 18),
        standing_entry("team4", 10, 27, 19, 12),
        standing_entry("team2", 10, 20, 22, 21),
    ]
}
