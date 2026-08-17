use engine::{PlayStyle, PlayerData, PlayerRole, Position, TacticsConfig, TeamData};
use rand::{Rng, RngExt};

/// Build a synthetic team with per-attribute values centered on `avg_ovr`.
/// Formation is parsed as "4-3-3" → 4 DEF, 3 MID, 3 FWD (plus 1 GK always).
/// Player roles are sampled from position-appropriate distributions.
pub fn build_team(
    id: &str,
    name: &str,
    avg_ovr: u8,
    play_style: PlayStyle,
    formation: &str,
    rng: &mut impl Rng,
) -> TeamData {
    build_team_with_tactics(
        id,
        name,
        avg_ovr,
        play_style,
        formation,
        TacticsConfig::default(),
        rng,
    )
}

pub fn build_team_with_tactics(
    id: &str,
    name: &str,
    avg_ovr: u8,
    play_style: PlayStyle,
    formation: &str,
    tactics: TacticsConfig,
    rng: &mut impl Rng,
) -> TeamData {
    let (n_def, n_mid, n_fwd, used_fallback) = parse_formation(formation);
    TeamData {
        id: id.to_string(),
        name: name.to_string(),
        formation: if used_fallback {
            "4-4-2".to_string()
        } else {
            formation.to_string()
        },
        play_style,
        tactics,
        players: build_group(id, "", avg_ovr, (n_def, n_mid, n_fwd), rng),
    }
}

/// Build a full 22-man squad: a starting XI plus a same-shaped bench.
///
/// Real squads carry a second choice for every slot, which is exactly what the
/// instant-simulation path currently hands to the engine wholesale. The A/B mode
/// needs both halves separately so it can ask what each path does with them.
/// Bench players are `bench_ovr_penalty` points weaker so rotation has a cost.
pub fn build_squad_with_bench(
    id: &str,
    name: &str,
    avg_ovr: u8,
    bench_ovr_penalty: u8,
    play_style: PlayStyle,
    formation: &str,
    rng: &mut impl Rng,
) -> (TeamData, Vec<PlayerData>) {
    let (n_def, n_mid, n_fwd, used_fallback) = parse_formation(formation);
    let shape = (n_def, n_mid, n_fwd);
    let starters = build_group(id, "", avg_ovr, shape, rng);
    let bench = build_group(
        id,
        "sub",
        avg_ovr.saturating_sub(bench_ovr_penalty),
        shape,
        rng,
    );

    let team = TeamData {
        id: id.to_string(),
        name: name.to_string(),
        formation: if used_fallback {
            "4-4-2".to_string()
        } else {
            formation.to_string()
        },
        play_style,
        tactics: TacticsConfig::default(),
        players: starters,
    };
    (team, bench)
}

/// One GK plus the outfield shape, ids namespaced by `tag` so a squad's starters
/// and its bench never collide. `tag` is empty for starters, which keeps their
/// ids byte-identical to what the single-XI builders have always produced — the
/// existing seeded benchmarks stay reproducible.
fn build_group(
    team_id: &str,
    tag: &str,
    avg_ovr: u8,
    shape: (u8, u8, u8),
    rng: &mut impl Rng,
) -> Vec<PlayerData> {
    let (n_def, n_mid, n_fwd) = shape;
    let mut players = Vec::with_capacity(11);

    players.push(make_player(
        team_id,
        &format!("{tag}GK"),
        1,
        1,
        Position::Goalkeeper,
        avg_ovr,
        rng,
    ));
    for i in 1..=n_def {
        players.push(make_player(
            team_id,
            &format!("{tag}DEF"),
            i,
            n_def,
            Position::Defender,
            avg_ovr,
            rng,
        ));
    }
    for i in 1..=n_mid {
        players.push(make_player(
            team_id,
            &format!("{tag}MID"),
            i,
            n_mid,
            Position::Midfielder,
            avg_ovr,
            rng,
        ));
    }
    for i in 1..=n_fwd {
        players.push(make_player(
            team_id,
            &format!("{tag}FWD"),
            i,
            n_fwd,
            Position::Forward,
            avg_ovr,
            rng,
        ));
    }
    players
}

fn sample_role(
    position: Position,
    slot_idx: u8,
    total_in_position: u8,
    rng: &mut impl Rng,
) -> PlayerRole {
    match position {
        Position::Goalkeeper => {
            const ROLES: [PlayerRole; 3] = [
                PlayerRole::Standard,
                PlayerRole::BallPlayingKeeper,
                PlayerRole::SweeperKeeper,
            ];
            ROLES[rng.random_range(0usize..3)]
        }
        Position::Defender => {
            // FB count: 0 for 3-back, 2 for 4-back and 5-back.
            // CB slots are the first (total - fb_count) slots.
            let fb_count = if total_in_position <= 3 { 0u8 } else { 2u8 };
            let cb_count = total_in_position - fb_count;
            if slot_idx <= cb_count {
                // CB slots
                const ROLES: [PlayerRole; 3] = [
                    PlayerRole::Stopper,
                    PlayerRole::CoverCB,
                    PlayerRole::BallPlayingCB,
                ];
                ROLES[rng.random_range(0usize..3)]
            } else {
                // FB/WB slots
                const ROLES: [PlayerRole; 4] = [
                    PlayerRole::AttackingFB,
                    PlayerRole::DefensiveFB,
                    PlayerRole::WingBack,
                    PlayerRole::InvertedFB,
                ];
                ROLES[rng.random_range(0usize..4)]
            }
        }
        Position::Midfielder => {
            if slot_idx == 1 {
                // Holding/DM slot
                const ROLES: [PlayerRole; 3] = [
                    PlayerRole::AnchorMan,
                    PlayerRole::BallWinner,
                    PlayerRole::DeepLyingPlaymaker,
                ];
                ROLES[rng.random_range(0usize..3)]
            } else {
                const ROLES: [PlayerRole; 5] = [
                    PlayerRole::BoxToBox,
                    PlayerRole::Mezzala,
                    PlayerRole::Carrilero,
                    PlayerRole::InvertedWinger,
                    PlayerRole::WideForward,
                ];
                ROLES[rng.random_range(0usize..5)]
            }
        }
        Position::Forward => {
            const ROLES: [PlayerRole; 6] = [
                PlayerRole::Poacher,
                PlayerRole::TargetMan,
                PlayerRole::CompleteForward,
                PlayerRole::False9,
                PlayerRole::DeepLyingForward,
                PlayerRole::PressingForward,
            ];
            ROLES[rng.random_range(0usize..6)]
        }
    }
}

fn parse_formation(formation: &str) -> (u8, u8, u8, bool) {
    let parts: Vec<u8> = formation
        .split('-')
        .filter_map(|s| s.parse::<u8>().ok())
        .collect();

    let result = match parts.len() {
        2 => (parts[0], 0, parts[1]),
        3 => (parts[0], parts[1], parts[2]),
        4 => (parts[0], parts[1] + parts[2], parts[3]),
        _ => return (4, 4, 2, true),
    };

    // Ensure exactly 10 outfield players; fall back to 4-4-2 if not
    if result.0 + result.1 + result.2 != 10 {
        return (4, 4, 2, true);
    }
    (result.0, result.1, result.2, false)
}

/// `label` is the player's position label, already carrying any group tag —
/// `"GK"` for a starter, `"subGK"` for his deputy.
fn make_player(
    team_id: &str,
    label: &str,
    idx: u8,
    total_in_position: u8,
    position: Position,
    avg_ovr: u8,
    rng: &mut impl Rng,
) -> PlayerData {
    let base = avg_ovr as f64;

    fn noise(base: f64, rng: &mut impl Rng) -> u8 {
        (base + rng.random_range(-10.0f64..10.0f64)).clamp(10.0, 99.0) as u8
    }
    fn biased(base: f64, offset: f64, rng: &mut impl Rng) -> u8 {
        (base + offset + rng.random_range(-8.0f64..8.0f64)).clamp(10.0, 99.0) as u8
    }

    let (shoot_off, tackle_off, pass_off, defend_off, gk_off) = match position {
        Position::Goalkeeper => (-25.0, 0.0, 0.0, 10.0, 20.0),
        Position::Defender => (-18.0, 12.0, -5.0, 18.0, -15.0),
        Position::Midfielder => (-3.0, 5.0, 12.0, 0.0, -15.0),
        Position::Forward => (18.0, -12.0, 3.0, -18.0, -20.0),
    };

    let role = sample_role(position, idx, total_in_position, rng);

    PlayerData {
        id: format!("{team_id}_{label}{idx}"),
        name: format!("{label}{idx}"),
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
        role,
    }
}
