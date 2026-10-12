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
