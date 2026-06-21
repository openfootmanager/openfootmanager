use engine::{PlayerData, PlayStyle, Position, TeamData};
use rand::{Rng, RngExt};

/// Build a synthetic team with per-attribute values centered on `avg_ovr`.
/// Formation is parsed as "4-3-3" → 4 DEF, 3 MID, 3 FWD (plus 1 GK always).
pub fn build_team(
    id: &str,
    name: &str,
    avg_ovr: u8,
    play_style: PlayStyle,
    formation: &str,
    rng: &mut impl Rng,
) -> TeamData {
    let (n_def, n_mid, n_fwd) = parse_formation(formation);
    let mut players = Vec::with_capacity(11);

    players.push(make_player(id, "GK", 1, Position::Goalkeeper, avg_ovr, rng));
    for i in 1..=n_def {
        players.push(make_player(id, "DEF", i, Position::Defender, avg_ovr, rng));
    }
    for i in 1..=n_mid {
        players.push(make_player(id, "MID", i, Position::Midfielder, avg_ovr, rng));
    }
    for i in 1..=n_fwd {
        players.push(make_player(id, "FWD", i, Position::Forward, avg_ovr, rng));
    }

    TeamData {
        id: id.to_string(),
        name: name.to_string(),
        formation: formation.to_string(),
        play_style,
        players,
    }
}

fn parse_formation(formation: &str) -> (u8, u8, u8) {
    let parts: Vec<u8> = formation
        .split('-')
        .filter_map(|s| s.parse::<u8>().ok())
        .collect();

    let result = match parts.len() {
        2 => (parts[0], 0, parts[1]),
        3 => (parts[0], parts[1], parts[2]),
        4 => (parts[0], parts[1] + parts[2], parts[3]),
        _ => (4, 4, 2),
    };

    // Ensure exactly 10 outfield players; fall back to 4-4-2 if not
    if result.0 + result.1 + result.2 != 10 {
        return (4, 4, 2);
    }
    result
}

fn make_player(
    team_id: &str,
    pos_label: &str,
    idx: u8,
    position: Position,
    avg_ovr: u8,
    rng: &mut impl Rng,
) -> PlayerData {
    let base = avg_ovr as f64;

    // Helpers as local fns to avoid impl Trait in closure params
    fn noise(base: f64, rng: &mut impl Rng) -> u8 {
        (base + rng.random_range(-10.0f64..10.0f64)).clamp(10.0, 99.0) as u8
    }
    fn biased(base: f64, offset: f64, rng: &mut impl Rng) -> u8 {
        (base + offset + rng.random_range(-8.0f64..8.0f64)).clamp(10.0, 99.0) as u8
    }

    // Position-specific attribute offsets
    let (shoot_off, tackle_off, pass_off, defend_off, gk_off) = match position {
        Position::Goalkeeper => (-25.0, 0.0, 0.0, 10.0, 20.0),
        Position::Defender => (-18.0, 12.0, -5.0, 18.0, -15.0),
        Position::Midfielder => (-3.0, 5.0, 12.0, 0.0, -15.0),
        Position::Forward => (18.0, -12.0, 3.0, -18.0, -20.0),
    };

    PlayerData {
        id: format!("{team_id}_{pos_label}{idx}"),
        name: format!("{pos_label}{idx}"),
        position,
        ovr: avg_ovr,
        condition: rng.random_range(80u8..=100u8),
        fitness: rng.random_range(65u8..=90u8),
        pace: noise(base, rng),
        stamina: noise(base, rng),
        strength: noise(base, rng),
        agility: noise(base, rng),
        passing: biased(base, pass_off, rng),
        shooting: biased(base, shoot_off, rng),
        tackling: biased(base, tackle_off, rng),
        dribbling: noise(base, rng),
        defending: biased(base, defend_off, rng),
        positioning: noise(base, rng),
        vision: biased(base, pass_off / 2.0, rng),
        decisions: noise(base, rng),
        composure: noise(base, rng),
        aggression: noise(base, rng),
        teamwork: noise(base, rng),
        leadership: noise(base, rng),
        handling: biased(base, gk_off, rng),
        reflexes: biased(base, gk_off, rng),
        aerial: noise(base, rng),
        traits: vec![],
    }
}
