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

/// Every manager the game can produce: the three archetypes at the bottom, the
/// middle and the top of the experience range.
fn every_manager() -> Vec<(String, AiProfile)> {
    let mut all = Vec::new();
    for experience in [20u8, 50, 90] {
        for personality in [
            AiPersonality::Pragmatist,
            AiPersonality::Visionary,
            AiPersonality::Reactive,
        ] {
            all.push((
                format!("{personality:?} on {experience} experience"),
                profile(experience, personality),
            ));
        }
    }
    all
}

/// What the manager did, counted by kind.
#[derive(Default)]
struct Interventions {
    substitutions: usize,
    style_changes: usize,
    formation_changes: usize,
}

impl Interventions {
    fn any(&self) -> bool {
        self.substitutions + self.style_changes + self.formation_changes > 0
    }
}

/// Play out a match in which the home side is `home_goals`–`away_goals` down for
/// as long as it lasts, and report what the home manager did about it.
///
/// The scoreline is re-pinned every minute rather than reached by simulation:
/// the point of these tests is what a manager does in a given position, and a
/// position arrived at by rolling dice makes the test about the seed.
fn a_match_stuck_at(
    home_goals: u8,
    away_goals: u8,
    from_minute: u8,
    profile: &AiProfile,
) -> Interventions {
    let mut state = LiveMatchState::new(
        team("home", eleven("home", 60, 100)),
        team("away", eleven("away", 60, 100)),
        MatchConfig::default(),
        bench("home", 55),
        bench("away", 55),
        false,
    );

    let mut rng = StdRng::seed_from_u64(11);
    let mut did = Interventions::default();

    loop {
        if state.step_minute(&mut rng).is_finished {
            break;
        }
        if state.minute() < from_minute {
            continue;
        }
        state.test_set_score(home_goals, away_goals);
        for cmd in ai_decide(&state, Side::Home, profile, &mut rng) {
            match cmd {
                MatchCommand::Substitute { .. } => did.substitutions += 1,
                MatchCommand::ChangePlayStyle { .. } => did.style_changes += 1,
                MatchCommand::ChangeFormation { .. } => did.formation_changes += 1,
                _ => {}
            }
            let _ = state.apply_command(cmd);
        }
    }

    did
}

// ---------------------------------------------------------------------------
// A manager who is losing does something about it
// ---------------------------------------------------------------------------

/// The report that started this work: *"AI managers seem to be too dumb. They
/// don't change playstyles that much."*
///
/// Two goals down with twenty minutes left and a full bench is the least
/// ambiguous position in football. Every manager in the game reacts to it —
/// there is nothing here for a personality or a thin CV to disagree about, so
/// the sweep is over every profile the game can build.
#[test]
fn two_goals_down_at_seventy_every_manager_uses_the_bench() {
    for (who, profile) in every_manager() {
        let did = a_match_stuck_at(0, 2, 70, &profile);
        assert!(
            did.any(),
            "{who} watched his side lose 0-2 from the seventieth minute with five \
             substitutions and a full bench available, and did not make one \
             substitution, change of style or change of shape."
        );
    }
}

/// The same position, but stop the clock the moment the second half kicks off.
///
/// Half-time is when managers change matches, and the AI used to sit it out
/// entirely: `ai_decide` returned early on any phase that was not being played,
/// and `HalfTime` is a phase of its own.
fn a_first_half_stuck_at(home_goals: u8, away_goals: u8, profile: &AiProfile) -> Interventions {
    let mut state = LiveMatchState::new(
        team("home", eleven("home", 60, 100)),
        team("away", eleven("away", 60, 100)),
        MatchConfig::default(),
        bench("home", 55),
        bench("away", 55),
        false,
    );

    let mut rng = StdRng::seed_from_u64(11);
    let mut did = Interventions::default();

    loop {
        let result = state.step_minute(&mut rng);
        if result.is_finished || state.phase() == MatchPhase::SecondHalf {
            break;
        }
        state.test_set_score(home_goals, away_goals);
        for cmd in ai_decide(&state, Side::Home, profile, &mut rng) {
            match cmd {
                MatchCommand::Substitute { .. } => did.substitutions += 1,
                MatchCommand::ChangePlayStyle { .. } => did.style_changes += 1,
                MatchCommand::ChangeFormation { .. } => did.formation_changes += 1,
                _ => {}
            }
            let _ = state.apply_command(cmd);
        }
    }

    did
}

#[test]
fn two_goals_down_at_half_time_every_manager_uses_the_interval() {
    for (who, profile) in every_manager() {
        let did = a_first_half_stuck_at(0, 2, &profile);
        assert!(
            did.any(),
            "{who} went in two goals down at half time and came back out with the \
             same eleven, the same shape and the same instructions."
        );
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

// ---------------------------------------------------------------------------
// Five substitutions are five substitutions
// ---------------------------------------------------------------------------

/// Eight players, two of each position, so a side can keep going back to it.
fn deep_bench(prefix: &str, skill: u8) -> Vec<PlayerData> {
    let mut all = Vec::new();
    for spare in 0..2 {
        for (tag, position) in [
            ("gk", Position::Goalkeeper),
            ("def", Position::Defender),
            ("mid", Position::Midfielder),
            ("fwd", Position::Forward),
        ] {
            all.push(player(
                &format!("{prefix}_sub_{tag}{spare}"),
                position,
                skill,
                100,
            ));
        }
    }
    all
}

/// The laws allow five; the tactical branches counted to three.
///
/// `max_subs` has been 5 on `LiveMatchState` all along, and the fatigue branch
/// respected it — but the two branches that exist for exactly this situation,
/// chasing a game and protecting a lead, were both written `subs_made < 3`. A
/// side two goals down spent the last half hour with substitutions in its
/// pocket that the laws of the game said it could use.
#[test]
fn a_side_chasing_the_game_uses_more_than_three_substitutions() {
    let mut state = LiveMatchState::new(
        team("home", eleven("home", 60, 100)),
        team("away", eleven("away", 60, 100)),
        MatchConfig::default(),
        deep_bench("home", 55),
        deep_bench("away", 55),
        false,
    );

    let mut rng = StdRng::seed_from_u64(3);
    let manager = profile(90, AiPersonality::Pragmatist);
    let mut made = 0usize;

    loop {
        if state.step_minute(&mut rng).is_finished {
            break;
        }
        state.test_set_score(0, 2);
        for cmd in ai_decide(&state, Side::Home, &manager, &mut rng) {
            let is_substitution = matches!(cmd, MatchCommand::Substitute { .. });
            if state.apply_command(cmd).is_ok() && is_substitution {
                made += 1;
            }
        }
    }

    assert!(
        made > 3,
        "two goals down for the whole match with a bench of eight, the manager \
         made {made} substitutions out of the five he is allowed"
    );
}

// ---------------------------------------------------------------------------
// Who comes off
// ---------------------------------------------------------------------------

/// The victim used to be chosen by where the squad list happened to put him:
/// `candidates.last()` for a chasing substitution, `forwards.first()` for a
/// defensive one. Squad order is an artefact of how the side was assembled, so
/// the same player came off every match and it was never the tired one.
///
/// The manager here is at the top of the experience range, where the close-call
/// misjudgement never fires — this is about who he is aiming at, not about how
/// often he misses.
#[test]
fn the_player_taken_off_is_the_one_with_least_left_to_give() {
    let mut starters = eleven("home", 60, 100);
    // Fifteen points down on the rest of them: comfortably the most spent
    // player on the pitch, and comfortably clear of the fatigue threshold all
    // match, so this is the chasing substitution choosing a victim rather than
    // the exhaustion branch firing.
    for p in starters.iter_mut() {
        if p.id == "home_mid0" {
            p.condition = 85;
        }
    }

    let mut state = LiveMatchState::new(
        team("home", starters),
        team("away", eleven("away", 60, 100)),
        MatchConfig::default(),
        bench("home", 55),
        bench("away", 55),
        false,
    );

    let mut rng = StdRng::seed_from_u64(5);
    let manager = profile(100, AiPersonality::Pragmatist);

    loop {
        if state.step_minute(&mut rng).is_finished {
            break;
        }
        state.test_set_score(0, 2);
        for cmd in ai_decide(&state, Side::Home, &manager, &mut rng) {
            if let MatchCommand::Substitute {
                ref player_off_id, ..
            } = cmd
            {
                assert_eq!(
                    player_off_id, "home_mid0",
                    "the manager took off {player_off_id}, who was as fresh as \
                     everyone else, and left the one player who had been running \
                     on empty since kick-off out there"
                );
                return;
            }
        }
    }

    panic!("two goals down for a whole match and not one substitution was made");
}
