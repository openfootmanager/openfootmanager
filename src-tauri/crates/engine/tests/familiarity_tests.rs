//! What the engine does with `PlayerData::line_familiarity`: the factor on a player's match
//! rating for the line he is in now. The caller (ofm_core's bridge) decides the factors; these
//! tests pin how the engine reads them.

use engine::live_match::{LiveMatchState, MatchCommand};
use engine::{
    MatchConfig, PlayStyle, PlayerData, PlayerRole, Position, Side, TacticsConfig, TeamData,
};

fn player(id: &str, position: Position) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: id.to_string(),
        position,
        ovr: 70,
        condition: 100,
        fitness: 100,
        injury_risk: 1.0,
        line_familiarity: [1.0; 4],
        pace: 70,
        stamina: 70,
        strength: 70,
        agility: 70,
        passing: 70,
        shooting: 70,
        tackling: 70,
        dribbling: 70,
        defending: 70,
        positioning: 70,
        vision: 70,
        decisions: 70,
        composure: 70,
        aggression: 70,
        teamwork: 70,
        leadership: 70,
        handling: 70,
        reflexes: 70,
        aerial: 70,
        traits: vec![],
        role: PlayerRole::Standard,
    }
}

/// A 4-4-2 whose outfielders are `{prefix}_1` .. `{prefix}_10` (1-4 defend, 5-8 midfield).
fn side(prefix: &str) -> TeamData {
    let mut players = vec![player(&format!("{prefix}_gk"), Position::Goalkeeper)];
    for number in 1..=10 {
        let position = match number {
            1..=4 => Position::Defender,
            5..=8 => Position::Midfielder,
            _ => Position::Forward,
        };
        players.push(player(&format!("{prefix}_{number}"), position));
    }
    TeamData {
        id: prefix.to_string(),
        name: prefix.to_string(),
        formation: "4-4-2".to_string(),
        play_style: PlayStyle::Balanced,
        tactics: TacticsConfig::default(),
        players,
    }
}

/// The home player `id`'s factor in the line the live match has him in now.
fn home_factor(state: &LiveMatchState, id: &str) -> f64 {
    state
        .snapshot()
        .home_team
        .players
        .iter()
        .find(|player| player.id == id)
        .map(PlayerData::familiarity_factor)
        .expect("the player is on the home side")
}

/// Given a player written before `line_familiarity` existed (an old save, or a caller that
/// does not set it),
/// When he is read,
/// Then he is at home in every line: his factor is 1.0 wherever he plays, so he plays exactly as
/// before the field existed.
#[test]
fn a_player_without_line_familiarity_is_at_home_in_every_line() {
    let mut json = serde_json::to_value(player("p", Position::Defender)).expect("it serialises");
    json.as_object_mut()
        .expect("a player is an object")
        .remove("line_familiarity");
    let mut read: PlayerData = serde_json::from_value(json).expect("it reads without the field");
    assert_eq!(read.line_familiarity, [1.0; 4]);
    for position in [
        Position::Goalkeeper,
        Position::Defender,
        Position::Midfielder,
        Position::Forward,
    ] {
        read.position = position;
        assert_eq!(read.familiarity_factor(), 1.0);
    }
}

/// Given a player whose factors are 0.6 in goal, 0.7 in defence, 0.8 in midfield, 0.9 up front,
/// When he stands in each line,
/// Then his factor is that line's entry.
#[test]
fn a_players_factor_is_the_entry_for_the_line_he_is_in() {
    let mut versatile = player("p", Position::Goalkeeper);
    versatile.line_familiarity = [0.6, 0.7, 0.8, 0.9];
    let factors = [
        Position::Goalkeeper,
        Position::Defender,
        Position::Midfielder,
        Position::Forward,
    ]
    .map(|position| {
        versatile.position = position;
        versatile.familiarity_factor()
    });
    assert_eq!(factors, [0.6, 0.7, 0.8, 0.9]);
}

/// Given a four-man midfield of equal players, one of whom carries 0.5 in midfield,
/// When the midfield's strength is read, as an average and with nobody off the pitch,
/// Then it falls by an eighth: that player counts at half, a quarter of the line.
#[test]
fn a_misplaced_midfielder_weighs_his_line_down_by_his_factor() {
    let mut team = side("t");
    let full = team.midfield_rating();
    team.players[5].line_familiarity[2] = 0.5;
    let misplaced = team.midfield_rating();
    assert_eq!(misplaced - full, -full / 8.0);
    let skill = |p: &PlayerData| p.passing;
    assert_eq!(
        team.line_strength(Position::Midfielder, skill, &|_| false, 4),
        70.0 * 3.5 / 4.0
    );
}

/// Given a home midfielder who carries 0.5 up front,
/// When the home side changes to 4-3-3 and he is the one moved forward,
/// Then the live match reads his forward factor from then on.
#[test]
fn a_change_of_shape_moves_a_player_onto_his_factor_for_the_new_line() {
    let mut home = side("h");
    for midfielder in &mut home.players[5..=8] {
        midfielder.line_familiarity[3] = 0.5;
        midfielder.shooting = 60;
    }
    // The best-suited to go forward: the change of shape moves him and no one else.
    home.players[8].shooting = 95;
    let mut state = LiveMatchState::new(
        home,
        side("a"),
        MatchConfig::default(),
        vec![],
        vec![],
        false,
    );
    assert_eq!(home_factor(&state, "h_8"), 1.0);

    state
        .apply_command(MatchCommand::ChangeFormation {
            side: Side::Home,
            formation: "4-3-3".to_string(),
        })
        .expect("a change of shape is always accepted");

    assert_eq!(home_factor(&state, "h_8"), 0.5);
}

/// Given a bench defender who carries 0.7 in midfield,
/// When he comes on for a home midfielder,
/// Then he takes the midfielder's line and the live match reads his midfield factor.
#[test]
fn a_substitute_plays_at_his_factor_for_the_line_he_takes() {
    let mut substitute = player("h_sub", Position::Defender);
    substitute.line_familiarity[2] = 0.7;
    let mut state = LiveMatchState::new(
        side("h"),
        side("a"),
        MatchConfig::default(),
        vec![substitute],
        vec![],
        false,
    );

    state
        .apply_command(MatchCommand::Substitute {
            side: Side::Home,
            player_off_id: "h_5".to_string(),
            player_on_id: "h_sub".to_string(),
        })
        .expect("the substitution is allowed");

    assert_eq!(home_factor(&state, "h_sub"), 0.7);
}
