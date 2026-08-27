//! What each player in a squad is actually asked to do.
//!
//! `Team.player_roles` was never written for anyone but the human's club, so
//! every AI player converted to [`PlayerRole::Standard`] at the engine boundary
//! and the whole 27-entry role modifier table — worth ±5–15% on a sampled
//! action — was unavailable to every club in the world but one.
//!
//! # A role is earned by shape, not by quality
//!
//! Each specialist role scores as the average of the three attributes it is
//! actually about, and `Standard` scores as the player's own overall average
//! plus a margin. A role is therefore taken only when the player is
//! *distinctively* good at it relative to himself — not when he is good.
//!
//! That distinction is the whole design. Scoring roles on raw attribute totals
//! would have handed every specialist role to the best players and left the
//! squad filler on `Standard`, turning a role into a second rating. Scoring on
//! shape means a modest full-back who happens to be quick is an `AttackingFB`
//! and a better one who is quick at nothing in particular stays `Standard`,
//! which is what the word is for.
//!
//! # Coarse positions are the normal case
//!
//! A generated squad carries the legacy buckets — `Defender`, `Midfielder`,
//! `Forward` — so most players are choosing from eight to twelve candidate
//! roles rather than three or four. That is what makes shape-scoring necessary
//! rather than decorative: within a bucket the candidates are genuinely
//! different jobs, and the attributes are the only thing that separates them.

use domain::player::Player;
use domain::team::{PlayStyle, PlayerRole, Team};

/// How far above his own average a player must score before a specialist role
/// is even on the table, before accounting for how many roles he is choosing
/// between. See [`specialist_margin`].
const SPECIALIST_MARGIN: f64 = 4.0;

/// How much of the margin scales with the number of candidate roles.
///
/// Roughly the spread of a three-attribute average across generated players —
/// the unit in which "how far above his own average" is worth measuring.
const SELECTION_SPREAD: f64 = 5.0;

/// The margin a specialist role must clear, for a position admitting
/// `candidates` of them.
///
/// It grows with the candidate count, and it has to. Picking the best of twelve
/// roles is taking a *maximum*, not a sample: the more jobs a position admits,
/// the more chances a player has to look distinctive at one of them by nothing
/// but the luck of which attributes came out high. Measured against a real
/// generated world with a flat margin of 5, **93.5% of players took a
/// specialist role** — 99.6% of midfielders, who choose between eleven, against
/// 56.5% of goalkeepers, who choose between two. `Standard` had stopped meaning
/// anything, and the gap between the groups was pure candidate count.
///
/// `√(2 ln n)` is where the maximum of `n` draws sits, so this measures a player
/// against the best role his position would throw up by chance rather than
/// against a fixed number.
fn specialist_margin(candidates: usize) -> f64 {
    let n = candidates.max(1) as f64;
    SPECIALIST_MARGIN + SELECTION_SPREAD * (2.0 * n.ln()).sqrt()
}

/// What a club's style is worth when it is choosing between two roles a player
/// could plausibly fill. Deliberately smaller than the margin: a style shades
/// the choice between candidates, it does not manufacture a specialist out of a
/// player who has no case for one.
const STYLE_PREFERENCE: f64 = 3.0;

/// The three attributes a role is actually about.
///
/// Three rather than one so a role reads as a job description, and three rather
/// than eight so the profiles stay distinguishable — averaging enough
/// attributes together turns every role into the player's overall rating again.
fn role_profile(role: &PlayerRole) -> [fn(&domain::player::PlayerAttributes) -> u8; 3] {
    use PlayerRole as R;
    match role {
        // Nobody's profile; scored separately against the player's own average.
        R::Standard => [|a| a.decisions, |a| a.teamwork, |a| a.composure],

        R::BallPlayingKeeper => [|a| a.passing, |a| a.composure, |a| a.vision],
        R::SweeperKeeper => [|a| a.pace, |a| a.positioning, |a| a.decisions],

        R::Stopper => [|a| a.tackling, |a| a.aggression, |a| a.strength],
        R::CoverCB => [|a| a.pace, |a| a.positioning, |a| a.decisions],
        R::BallPlayingCB => [|a| a.passing, |a| a.vision, |a| a.composure],

        R::AttackingFB => [|a| a.pace, |a| a.dribbling, |a| a.stamina],
        R::DefensiveFB => [|a| a.tackling, |a| a.positioning, |a| a.defending],
        R::InvertedFB => [|a| a.passing, |a| a.vision, |a| a.decisions],
        R::WingBack => [|a| a.stamina, |a| a.pace, |a| a.teamwork],

        R::AnchorMan => [|a| a.positioning, |a| a.defending, |a| a.decisions],
        R::BallWinner => [|a| a.tackling, |a| a.aggression, |a| a.stamina],
        R::DeepLyingPlaymaker => [|a| a.passing, |a| a.vision, |a| a.composure],

        R::BoxToBox => [|a| a.stamina, |a| a.strength, |a| a.teamwork],
        R::Carrilero => [|a| a.teamwork, |a| a.positioning, |a| a.passing],
        R::Mezzala => [|a| a.dribbling, |a| a.vision, |a| a.shooting],

        R::AdvancedPlaymaker => [|a| a.vision, |a| a.passing, |a| a.decisions],
        R::ShadowStriker => [|a| a.shooting, |a| a.pace, |a| a.composure],

        R::WideForward => [|a| a.pace, |a| a.dribbling, |a| a.shooting],
        R::InsideForward => [|a| a.dribbling, |a| a.shooting, |a| a.agility],
        R::InvertedWinger => [|a| a.passing, |a| a.vision, |a| a.agility],

        R::Poacher => [|a| a.shooting, |a| a.positioning, |a| a.composure],
        R::TargetMan => [|a| a.strength, |a| a.aerial, |a| a.shooting],
        R::DeepLyingForward => [|a| a.passing, |a| a.vision, |a| a.teamwork],
        R::False9 => [|a| a.vision, |a| a.dribbling, |a| a.decisions],
        R::PressingForward => [|a| a.stamina, |a| a.aggression, |a| a.pace],
        R::CompleteForward => [|a| a.shooting, |a| a.dribbling, |a| a.strength],
    }
}

/// The roles a club of this style asks for when the players allow it.
///
/// One list per style rather than a separate profile table per style: a role
/// means the same thing everywhere, and what changes between clubs is which
/// jobs they want doing.
fn style_preferences(play_style: &PlayStyle) -> &'static [PlayerRole] {
    use PlayerRole as R;
    match play_style {
        PlayStyle::Balanced => &[],
        PlayStyle::Attacking => &[
            R::AttackingFB,
            R::ShadowStriker,
            R::InsideForward,
            R::CompleteForward,
        ],
        PlayStyle::Defensive => &[R::DefensiveFB, R::AnchorMan, R::Stopper, R::TargetMan],
        PlayStyle::Possession => &[
            R::BallPlayingCB,
            R::InvertedFB,
            R::DeepLyingPlaymaker,
            R::AdvancedPlaymaker,
            R::False9,
        ],
        PlayStyle::Counter => &[R::CoverCB, R::AnchorMan, R::WideForward, R::Poacher],
        PlayStyle::HighPress => &[R::Stopper, R::BallWinner, R::WingBack, R::PressingForward],
    }
}

fn mean_of(values: impl IntoIterator<Item = u8>) -> f64 {
    let values: Vec<u8> = values.into_iter().collect();
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64
}

/// The player's own average across everything an outfielder is judged on.
///
/// Goalkeeping attributes are left out on purpose. A goalkeeper's `shooting` of
/// 20 would drag his average down far enough that every keeper cleared the
/// specialist margin on anything, and an outfielder's `handling` of 30 would do
/// the same in reverse.
fn player_baseline(player: &Player) -> f64 {
    let a = &player.attributes;
    let outfield = [
        a.pace,
        a.stamina,
        a.strength,
        a.agility,
        a.passing,
        a.dribbling,
        a.positioning,
        a.vision,
        a.decisions,
        a.composure,
        a.teamwork,
        a.aerial,
    ];
    let specialised = if player.position.to_group_position() == domain::player::Position::Goalkeeper
    {
        vec![a.handling, a.reflexes]
    } else {
        vec![a.shooting, a.tackling, a.defending, a.aggression]
    };
    mean_of(outfield.into_iter().chain(specialised))
}

/// The role this player is best suited to, given what his club is trying to do.
///
/// Deterministic: no RNG, no iteration-order dependence. Ties fall to the
/// earlier role in [`Position::valid_roles`], which lists `Standard` first, so a
/// player with nothing to distinguish him keeps it.
///
/// [`Position::valid_roles`]: domain::player::Position::valid_roles
fn role_for(player: &Player, play_style: &PlayStyle) -> PlayerRole {
    let baseline = player_baseline(player);
    let preferred = style_preferences(play_style);
    let candidates = player.position.valid_roles().len().saturating_sub(1);

    let mut best = PlayerRole::Standard;
    let mut best_score = baseline + specialist_margin(candidates);

    for role in player.position.valid_roles() {
        if *role == PlayerRole::Standard {
            continue;
        }
        let profile = role_profile(role);
        let mut score = mean_of(
            profile
                .iter()
                .map(|attribute| attribute(&player.attributes)),
        );
        if preferred.contains(role) {
            score += STYLE_PREFERENCE;
        }
        if score > best_score {
            best_score = score;
            best = role.clone();
        }
    }

    best
}

/// Give every player in the squad the job his attributes and his club's style
/// argue for, replacing whatever was there.
///
/// `Standard` players are left out of the map rather than stored explicitly:
/// the engine treats a missing entry as `Standard`, and writing them would put
/// a row in the saved JSON for every player who has nothing special to say.
///
/// Takes an iterator rather than a slice because its two callers hold the squad
/// differently: the generator has it as an owned block, the weekly review has it
/// as a filtered borrow out of the whole world's player list.
pub(crate) fn assign_squad_roles<'a>(
    team: &mut Team,
    players: impl IntoIterator<Item = &'a Player>,
) {
    team.player_roles.clear();
    for player in players {
        let role = role_for(player, &team.play_style);
        if role != PlayerRole::Standard {
            team.player_roles.insert(player.id.clone(), role);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{PlayerAttributes, Position};

    fn flat_attrs(level: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: level,
            stamina: level,
            strength: level,
            agility: level,
            passing: level,
            shooting: level,
            tackling: level,
            dribbling: level,
            defending: level,
            positioning: level,
            vision: level,
            decisions: level,
            composure: level,
            aggression: level,
            teamwork: level,
            leadership: level,
            handling: level,
            reflexes: level,
            aerial: level,
        }
    }

    fn player(id: &str, position: Position, attributes: PlayerAttributes) -> Player {
        Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-01-01".to_string(),
            "England".to_string(),
            position,
            attributes,
        )
    }

    fn club(play_style: PlayStyle) -> Team {
        let mut team = Team::new(
            "club".to_string(),
            "Club".to_string(),
            "CLB".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            20_000,
        );
        team.play_style = play_style;
        team
    }

    #[test]
    fn a_player_who_is_good_at_nothing_in_particular_stays_standard() {
        let flat = player("flat", Position::Midfielder, flat_attrs(80));
        assert_eq!(
            role_for(&flat, &PlayStyle::Balanced),
            PlayerRole::Standard,
            "an 80-rated player with no shape must not be handed a specialism"
        );
    }

    /// The point of scoring shape rather than quality: quality alone must not
    /// buy a role, or `Standard` becomes a synonym for "reserve".
    #[test]
    fn raising_every_attribute_together_does_not_buy_a_specialism() {
        for level in [40, 60, 80, 95] {
            let flat = player("flat", Position::Forward, flat_attrs(level));
            assert_eq!(
                role_for(&flat, &PlayStyle::Balanced),
                PlayerRole::Standard,
                "a flat {level}-rated forward took a specialist role"
            );
        }
    }

    #[test]
    fn a_quick_direct_forward_is_not_the_same_player_as_a_big_one() {
        let mut quick = flat_attrs(60);
        quick.pace = 90;
        quick.dribbling = 90;
        quick.shooting = 85;
        let winger = player("quick", Position::Forward, quick);

        let mut big = flat_attrs(60);
        big.strength = 92;
        big.aerial = 92;
        big.shooting = 85;
        let target = player("big", Position::Forward, big);

        assert_eq!(
            role_for(&winger, &PlayStyle::Balanced),
            PlayerRole::WideForward
        );
        assert_eq!(
            role_for(&target, &PlayStyle::Balanced),
            PlayerRole::TargetMan
        );
    }

    #[test]
    fn a_destroyer_and_a_passer_get_different_jobs_in_the_same_midfield() {
        let mut destroyer_attrs = flat_attrs(60);
        destroyer_attrs.tackling = 90;
        destroyer_attrs.aggression = 88;
        destroyer_attrs.stamina = 85;
        let destroyer = player("destroyer", Position::Midfielder, destroyer_attrs);

        let mut passer_attrs = flat_attrs(60);
        passer_attrs.passing = 90;
        passer_attrs.vision = 90;
        passer_attrs.composure = 85;
        let passer = player("passer", Position::Midfielder, passer_attrs);

        assert_eq!(
            role_for(&destroyer, &PlayStyle::Balanced),
            PlayerRole::BallWinner
        );
        assert_eq!(
            role_for(&passer, &PlayStyle::Balanced),
            PlayerRole::DeepLyingPlaymaker
        );
    }

    /// Style breaks a tie between jobs a player could do either of. It must not
    /// reach further than that.
    #[test]
    fn a_clubs_style_shades_a_close_call_without_overruling_the_player() {
        // Equally good at winning the ball and at reading the game: the two
        // profiles score within the style bonus of each other.
        let mut attrs = flat_attrs(60);
        attrs.tackling = 82;
        attrs.aggression = 82;
        attrs.stamina = 82;
        attrs.positioning = 84;
        attrs.defending = 84;
        attrs.decisions = 84;
        let all_rounder = player("both", Position::Midfielder, attrs);

        assert_eq!(
            role_for(&all_rounder, &PlayStyle::HighPress),
            PlayerRole::BallWinner,
            "a pressing side wants the ball won"
        );
        assert_eq!(
            role_for(&all_rounder, &PlayStyle::Counter),
            PlayerRole::AnchorMan,
            "a counter-attacking side wants the space in front of the back four held"
        );

        // But no style can talk a player with no case at all into a specialism.
        let flat = player("flat", Position::Midfielder, flat_attrs(70));
        assert_eq!(role_for(&flat, &PlayStyle::HighPress), PlayerRole::Standard);
    }

    #[test]
    fn a_keeper_is_only_ever_given_a_keepers_job() {
        let mut sweeper_attrs = flat_attrs(60);
        sweeper_attrs.pace = 88;
        sweeper_attrs.positioning = 88;
        sweeper_attrs.decisions = 88;
        let keeper = player("gk", Position::Goalkeeper, sweeper_attrs);

        let role = role_for(&keeper, &PlayStyle::Possession);
        assert!(
            Position::Goalkeeper.admits_role(&role),
            "a goalkeeper was given {role:?}"
        );
    }

    #[test]
    fn every_assigned_role_is_one_the_position_admits() {
        let positions = [
            Position::Goalkeeper,
            Position::Defender,
            Position::Midfielder,
            Position::Forward,
            Position::CenterBack,
            Position::RightBack,
            Position::DefensiveMidfielder,
            Position::AttackingMidfielder,
            Position::LeftWinger,
            Position::Striker,
        ];
        let styles = [
            PlayStyle::Balanced,
            PlayStyle::Attacking,
            PlayStyle::Defensive,
            PlayStyle::Possession,
            PlayStyle::Counter,
            PlayStyle::HighPress,
        ];
        // Sweep one standout attribute at a time so every profile gets its turn
        // at winning, in every position, under every style.
        let standouts: [fn(&mut PlayerAttributes); 8] = [
            |a| a.pace = 99,
            |a| a.strength = 99,
            |a| a.passing = 99,
            |a| a.shooting = 99,
            |a| a.tackling = 99,
            |a| a.vision = 99,
            |a| a.stamina = 99,
            |a| a.dribbling = 99,
        ];

        for position in &positions {
            for style in &styles {
                for standout in standouts {
                    let mut attributes = flat_attrs(55);
                    standout(&mut attributes);
                    let candidate = player("p", position.clone(), attributes);
                    let role = role_for(&candidate, style);
                    assert!(
                        position.admits_role(&role),
                        "{position:?} under {style:?} was given {role:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_same_squad_gets_the_same_roles_every_time() {
        let mut attrs = flat_attrs(62);
        attrs.pace = 88;
        let players = vec![
            player("a", Position::Defender, attrs.clone()),
            player("b", Position::Midfielder, flat_attrs(70)),
            player("c", Position::Forward, attrs),
        ];

        let mut first = club(PlayStyle::Counter);
        assign_squad_roles(&mut first, &players);
        let mut second = club(PlayStyle::Counter);
        assign_squad_roles(&mut second, &players);

        assert_eq!(first.player_roles, second.player_roles);
    }

    #[test]
    fn assigning_twice_does_not_accumulate_stale_roles() {
        let players = vec![player("gone", Position::Forward, {
            let mut a = flat_attrs(60);
            a.strength = 95;
            a.aerial = 95;
            a
        })];
        let mut team = club(PlayStyle::Balanced);
        assign_squad_roles(&mut team, &players);
        assert!(team.player_roles.contains_key("gone"));

        // The package path replaces generated players with authored ones and
        // re-runs this, so a role left behind would belong to a player who is
        // no longer at the club.
        assign_squad_roles(&mut team, &[]);
        assert!(team.player_roles.is_empty());
    }

    /// One high attribute is luck, not a shape.
    ///
    /// A midfielder chooses between eleven specialisms, and eleven three-attribute
    /// averages give a lucky player plenty of chances to top his own average at
    /// one of them. This is the case that says the margin has to scale with the
    /// candidate count: against a real generated world it did not, and 99.6% of
    /// midfielders came out as specialists.
    #[test]
    fn one_good_attribute_is_not_enough_to_earn_a_specialism() {
        for standout in [
            |a: &mut PlayerAttributes| a.pace = 90,
            |a: &mut PlayerAttributes| a.passing = 90,
            |a: &mut PlayerAttributes| a.tackling = 90,
        ] {
            let mut attributes = flat_attrs(60);
            standout(&mut attributes);
            let lucky = player("lucky", Position::Midfielder, attributes);
            assert_eq!(
                role_for(&lucky, &PlayStyle::Balanced),
                PlayerRole::Standard,
                "one attribute out of nineteen bought a job title"
            );
        }
    }

    #[test]
    fn a_squad_of_ordinary_players_is_not_wall_to_wall_specialists() {
        // Three players who fit a whole job description, plus three who are not
        // distinctive at all: the three must stay Standard.
        let mut players = Vec::new();
        for (index, standout) in [
            // BallWinner, DeepLyingPlaymaker, BoxToBox — the full profile each,
            // because that is what a shape is.
            |a: &mut PlayerAttributes| {
                a.tackling = 92;
                a.aggression = 92;
                a.stamina = 92;
            },
            |a: &mut PlayerAttributes| {
                a.passing = 92;
                a.vision = 92;
                a.composure = 92;
            },
            |a: &mut PlayerAttributes| {
                a.stamina = 92;
                a.strength = 92;
                a.teamwork = 92;
            },
        ]
        .into_iter()
        .enumerate()
        {
            let mut attributes = flat_attrs(60);
            standout(&mut attributes);
            players.push(player(
                &format!("shaped{index}"),
                Position::Midfielder,
                attributes,
            ));
        }
        for index in 0..3 {
            players.push(player(
                &format!("flat{index}"),
                Position::Midfielder,
                flat_attrs(60),
            ));
        }

        let mut team = club(PlayStyle::Balanced);
        assign_squad_roles(&mut team, &players);

        assert_eq!(
            team.player_roles.len(),
            3,
            "only the three players with a shape should have a job title"
        );
    }
}
