//! What the AI manager decides, and when.
//!
//! `live_match_tests.rs` covers the match machinery; this binary is about the
//! manager standing on the touchline. Every test here builds a situation a
//! supporter would recognise — "two down with twenty minutes left and five subs
//! unused" — and asserts the bench gets used.

use ::engine::ai::{AiPersonality, AiProfile, ai_decide};
use ::engine::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn player(id: &str, position: Position, skill: u8, condition: u8) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: skill,
        condition,
        fitness: 80,
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

/// A conventional 4-4-2, every player at `skill` and `condition`.
fn eleven(prefix: &str, skill: u8, condition: u8) -> Vec<PlayerData> {
    let mut players = vec![player(
        &format!("{prefix}_gk"),
        Position::Goalkeeper,
        skill,
        condition,
    )];
    for i in 0..4 {
        players.push(player(
            &format!("{prefix}_def{i}"),
            Position::Defender,
            skill,
            condition,
        ));
    }
    for i in 0..4 {
        players.push(player(
            &format!("{prefix}_mid{i}"),
            Position::Midfielder,
            skill,
            condition,
        ));
    }
    for i in 0..2 {
        players.push(player(
            &format!("{prefix}_fwd{i}"),
            Position::Forward,
            skill,
            condition,
        ));
    }
    players
}

fn team(id: &str, players: Vec<PlayerData>) -> TeamData {
    TeamData {
        id: id.to_string(),
        name: id.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics: TacticsConfig::default(),
        players,
    }
}

/// One of each position, so every replacement search finds cover.
fn bench(prefix: &str, skill: u8) -> Vec<PlayerData> {
    vec![
        player(
            &format!("{prefix}_sub_gk"),
            Position::Goalkeeper,
            skill,
            100,
        ),
        player(&format!("{prefix}_sub_def"), Position::Defender, skill, 100),
        player(
            &format!("{prefix}_sub_mid"),
            Position::Midfielder,
            skill,
            100,
        ),
        player(&format!("{prefix}_sub_fwd"), Position::Forward, skill, 100),
    ]
}

fn profile(experience: u8, personality: AiPersonality) -> AiProfile {
    AiProfile {
        reputation: 500,
        experience,
        personality,
    }
}

// ---------------------------------------------------------------------------
// The bench is not a place players come back from
// ---------------------------------------------------------------------------

/// A substituted player is pushed back onto the bench Vec — he has to go
/// somewhere, and the UI lists him — but he cannot come back on, and
/// `do_substitution` rejects it.
///
/// That rejection used to be the only thing standing between the AI and an
/// infinite loop, and it worked by accident: the old per-minute lottery would
/// propose the dead swap, get an `Err` back, and roll again next minute. A
/// manager who decides the same thing every time he looks does not get that
/// second chance — he proposes the same impossible substitution at every
/// checkpoint for the rest of the match, and makes none of the ones he could.
///
/// The trap is worst for exactly the player most likely to be taken off: a tired
/// star outranks every fresh reserve on the bench, so he is the first name the
/// replacement search returns.
#[test]
fn a_player_already_taken_off_is_never_asked_to_come_back_on() {
    let mut starters = eleven("home", 60, 100);
    // Two exhausted midfielders. The first is far the best player in the squad,
    // so once he is on the bench he outranks every reserve there.
    for p in starters.iter_mut() {
        if p.id == "home_mid0" {
            p.condition = 20;
            for attr in [
                &mut p.pace,
                &mut p.stamina,
                &mut p.strength,
                &mut p.passing,
                &mut p.shooting,
                &mut p.tackling,
                &mut p.dribbling,
                &mut p.defending,
                &mut p.positioning,
                &mut p.vision,
                &mut p.decisions,
            ] {
                *attr = 95;
            }
        }
        if p.id == "home_mid1" {
            p.condition = 21;
        }
    }

    let mut state = LiveMatchState::new(
        team("home", starters),
        team("away", eleven("away", 60, 100)),
        MatchConfig::default(),
        bench("home", 40),
        bench("away", 40),
        false,
    );

    let mut rng = StdRng::seed_from_u64(9);
    let profile = profile(90, AiPersonality::Pragmatist);
    let mut taken_off: Vec<String> = Vec::new();

    loop {
        if state.step_minute(&mut rng).is_finished {
            break;
        }
        for cmd in ai_decide(&state, Side::Home, &profile, &mut rng) {
            if let MatchCommand::Substitute {
                ref player_off_id,
                ref player_on_id,
                ..
            } = cmd
            {
                assert!(
                    !taken_off.contains(player_on_id),
                    "the manager asked {player_on_id} to come back on at minute {} \
                     after taking him off. He is on the bench because he is finished.",
                    state.minute()
                );
                taken_off.push(player_off_id.clone());
            }
            let _ = state.apply_command(cmd);
        }
    }

    assert!(
        !taken_off.is_empty(),
        "the fixture is meant to force at least one substitution; it forced none, \
         so the assertion above never ran"
    );
}
