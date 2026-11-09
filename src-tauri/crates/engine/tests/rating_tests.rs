use ::engine::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn player(id: &str, position: Position, skill: u8) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: skill,
        condition: 90,
        fitness: 75,
        injury_risk: 1.0,
        line_familiarity: [1.0; 4],
        pace: skill,
        stamina: skill,
        strength: skill,
        agility: skill,
        passing: skill,
        shooting: skill,
        tackling: skill,
        dribbling: skill,
        defending: skill,
        positioning: skill,
        vision: skill,
        decisions: skill,
        composure: skill,
        aggression: skill,
        teamwork: skill,
        leadership: skill,
        handling: skill,
        reflexes: skill,
        aerial: skill,
        traits: vec![],
        role: PlayerRole::Standard,
    }
}

fn team(id: &str, skill: u8) -> TeamData {
    let lines = [
        (Position::Goalkeeper, 1),
        (Position::Defender, 4),
        (Position::Midfielder, 4),
        (Position::Forward, 2),
    ];
    let players = lines
        .iter()
        .flat_map(|(position, count)| {
            (0..*count).map(move |n| player(&format!("{id}_{position:?}_{n}"), *position, skill))
        })
        .collect();
    TeamData {
        id: id.to_string(),
        name: id.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics: TacticsConfig::default(),
        players,
    }
}

fn play(seed: u64) -> MatchReport {
    let config = MatchConfig::default();
    simulate_with_rng(
        &team("home", 70),
        &team("away", 70),
        &config,
        &mut StdRng::seed_from_u64(seed),
    )
}

fn ratings_of(report: &MatchReport) -> Vec<(String, f32)> {
    let mut ratings: Vec<_> = report
        .player_stats
        .iter()
        .map(|(id, stats)| (id.clone(), stats.rating))
        .collect();
    ratings.sort_by(|a, b| a.0.cmp(&b.0));
    ratings
}

/// Given the same seed, When a match is played twice, Then every rating matches.
#[test]
fn ratings_are_deterministic_for_a_seed() {
    assert_eq!(ratings_of(&play(11)), ratings_of(&play(11)));
}

/// Given a played match, When reported, Then every starter carries a rating.
#[test]
fn every_starter_is_rated() {
    let report = play(3);
    let rated = report
        .player_stats
        .values()
        .filter(|s| s.rating > 0.0)
        .count();
    assert_eq!(rated, 22);
}

/// Given a match with goals, When rated, Then the scorers out-rate the average starter.
#[test]
fn scorers_rate_above_the_average_player() {
    let mut scorer_total = 0.0;
    let mut scorers = 0.0;
    let mut all_total = 0.0;
    let mut all = 0.0;
    for seed in 0..200 {
        let report = play(seed);
        for stats in report.player_stats.values().filter(|s| s.rating > 0.0) {
            all_total += stats.rating;
            all += 1.0;
            if stats.goals > 0 {
                scorer_total += stats.rating;
                scorers += 1.0;
            }
        }
    }
    assert!(scorers > 0.0);
    assert!(scorer_total / scorers > all_total / all + 0.5);
}

/// Given a match with a bench, When played live, Then an unused substitute has no rating.
#[test]
fn an_unused_substitute_is_unrated() {
    let home = team("home", 70);
    let away = team("away", 70);
    let spare = player("home_spare", Position::Midfielder, 70);
    let mut live = LiveMatchState::new(
        home,
        away,
        MatchConfig::default(),
        vec![spare],
        vec![],
        false,
    );
    let mut rng = StdRng::seed_from_u64(5);
    while !live.is_finished() {
        live.step_minute(&mut rng);
    }
    let report = live.into_report();
    let starters_rated = report
        .player_stats
        .iter()
        .filter(|(id, _)| !id.starts_with("home_spare"))
        .all(|(_, stats)| stats.rating > 0.0);
    assert!(starters_rated, "the live path must rate the starters");
    let spare_rating = report
        .player_stats
        .get("home_spare")
        .map_or(0.0, |s| s.rating);
    assert_eq!(spare_rating, 0.0);
}

/// Given many matches, When all marks are pooled, Then the mean sits where football ratings do
/// and extreme marks are rare.
#[test]
fn the_rating_distribution_is_realistic() {
    let ratings: Vec<f32> = (0..400)
        .flat_map(|seed| play(seed).player_stats.into_values())
        .filter(|s| s.rating > 0.0)
        .map(|s| s.rating)
        .collect();
    let n = ratings.len() as f32;
    let mean = ratings.iter().sum::<f32>() / n;
    let above = |limit: f32| ratings.iter().filter(|r| **r >= limit).count() as f32 / n;
    let below = |limit: f32| ratings.iter().filter(|r| **r < limit).count() as f32 / n;
    assert!((6.0..=6.6).contains(&mean), "mean {mean}");
    assert!(above(8.5) < 0.03, "8.5+: {}", above(8.5));
    assert!(below(4.0) < 0.03, "<4: {}", below(4.0));
}

/// Given many matches, When marks are averaged per position, Then no role is
/// systematically favoured: the engine gives each role different events to be
/// marked on, and `typical_work` is what keeps them level.
#[test]
fn no_position_is_systematically_over_or_under_rated() {
    let mut totals: std::collections::BTreeMap<String, (f32, f32)> = Default::default();
    for seed in 0..300 {
        for (id, stats) in play(seed).player_stats {
            let position = id.split('_').nth(1).unwrap().to_string();
            let entry = totals.entry(position).or_default();
            entry.0 += stats.rating;
            entry.1 += 1.0;
        }
    }
    assert_eq!(totals.len(), 4);
    let means: Vec<(String, f32)> = totals
        .into_iter()
        .map(|(position, (sum, count))| (position, sum / count))
        .collect();
    for (position, mean) in &means {
        assert!((5.9..=6.6).contains(mean), "{position} averaged {mean}");
    }
    let highest = means.iter().map(|m| m.1).fold(f32::MIN, f32::max);
    let lowest = means.iter().map(|m| m.1).fold(f32::MAX, f32::min);
    assert!(
        highest - lowest < 0.65,
        "positions spread {lowest}..{highest}"
    );
}

/// Given a live match with one substitution at minute 30, When it finishes, Then both the player
/// who came on and the player who went off are rated, whichever list they now sit in.
#[test]
fn a_substitute_who_came_on_and_the_player_he_replaced_are_both_rated() {
    let mut live = LiveMatchState::new(
        team("home", 70),
        team("away", 70),
        MatchConfig::default(),
        vec![player("home_spare", Position::Midfielder, 70)],
        vec![],
        false,
    );
    let mut rng = StdRng::seed_from_u64(5);
    for _ in 0..30 {
        live.step_minute(&mut rng);
    }
    live.apply_command(MatchCommand::Substitute {
        side: Side::Home,
        player_off_id: "home_Midfielder_0".to_string(),
        player_on_id: "home_spare".to_string(),
    })
    .expect("the substitution is legal");
    while !live.is_finished() {
        live.step_minute(&mut rng);
    }
    let report = live.into_report();

    assert!(report.player_stats["home_spare"].rating > 0.0, "came on");
    assert!(
        report.player_stats["home_Midfielder_0"].rating > 0.0,
        "went off"
    );
}

/// A live match from `seed` in which the home keeper is replaced at 85 minutes by a reserve
/// keeper, played to the end.
fn keeper_replaced_at_85(seed: u64) -> MatchReport {
    let mut live = LiveMatchState::new(
        team("home", 70),
        team("away", 70),
        MatchConfig::default(),
        vec![player("home_reserve_gk", Position::Goalkeeper, 70)],
        vec![],
        false,
    );
    let mut rng = StdRng::seed_from_u64(seed);
    while live.minute() < 85 {
        live.step_minute(&mut rng);
    }
    live.apply_command(MatchCommand::Substitute {
        side: Side::Home,
        player_off_id: "home_Goalkeeper_0".to_string(),
        player_on_id: "home_reserve_gk".to_string(),
    })
    .expect("the substitution is legal");
    while !live.is_finished() {
        live.step_minute(&mut rng);
    }
    live.into_report()
}

/// Given a live match in which the home keeper is replaced at 85 minutes by a reserve keeper,
/// and the away side does not score (the first such seed: the report counts its goals from the
/// match's events, so a score set by hand would not reach it),
/// When the report is drawn up,
/// Then the keeper who played 85 minutes kept the clean sheet and the reserve did not.
#[test]
fn the_keeper_in_goal_for_most_of_a_clean_sheet_keeps_it() {
    let report = (1..=200)
        .map(keeper_replaced_at_85)
        .find(|report| report.away_goals == 0)
        .expect("in 200 seeds the away side fails to score at least once");

    assert!(report.player_stats["home_Goalkeeper_0"].clean_sheet);
    assert!(!report.player_stats["home_reserve_gk"].clean_sheet);
    assert!(
        !report.player_stats["home_Defender_0"].clean_sheet,
        "a keeper's flag only"
    );
}

/// A live match from `seed` in which the home keeper is sent off before kick-off with no
/// reserve keeper, so the first player left on the pitch goes in goal; played to the end.
/// Returns the report and the id of the outfielder who stood in goal.
fn keeper_sent_off_before_kick_off(seed: u64) -> (MatchReport, String) {
    let mut live = LiveMatchState::new(
        team("home", 70),
        team("away", 70),
        MatchConfig::default(),
        vec![],
        vec![],
        false,
    );
    live.test_send_off("home_Goalkeeper_0");
    let in_goal = live
        .team(Side::Home)
        .players
        .iter()
        .find(|player| player.id != "home_Goalkeeper_0")
        .map(|player| player.id.clone())
        .expect("an outfielder is left to go in goal");
    let mut rng = StdRng::seed_from_u64(seed);
    while !live.is_finished() {
        live.step_minute(&mut rng);
    }
    (live.into_report(), in_goal)
}

/// Given a live match in which the home keeper is sent off before kick-off, an outfielder goes
/// in goal for the whole match, and the away side does not score (the first such seed),
/// When the report is drawn up,
/// Then the outfielder in goal kept the clean sheet and the keeper sent off did not: it goes to
/// whoever stood in goal, not to whoever is listed as a keeper.
#[test]
fn an_outfielder_in_goal_for_most_of_a_clean_sheet_keeps_it() {
    let (report, in_goal) = (1..=400)
        .map(keeper_sent_off_before_kick_off)
        .find(|(report, _)| report.away_goals == 0)
        .expect("in 400 seeds the away side fails to score at least once");

    assert!(report.player_stats[&in_goal].clean_sheet);
    assert!(!report.player_stats["home_Goalkeeper_0"].clean_sheet);
}
