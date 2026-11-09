use crate::event::{EventType, MatchEvent};
use crate::types::{PlayerData, PlayerRole, Position, Side, Zone};

use super::{LiveMatchState, MatchPhase, SubstitutionRecord};

// ---------------------------------------------------------------------------
// Substitution mechanics
// ---------------------------------------------------------------------------

impl LiveMatchState {
    pub(super) fn do_substitution(
        &mut self,
        side: Side,
        player_off_id: &str,
        player_on_id: &str,
    ) -> Result<(), String> {
        let subs_made = match side {
            Side::Home => &mut self.home_subs_made,
            Side::Away => &mut self.away_subs_made,
        };

        if *subs_made >= self.max_subs {
            return Err("be.error.liveMatch.maxSubstitutionsReached".into());
        }

        // A shootout is taken by the players on the pitch at the final whistle of extra time,
        // and its kicking order is fixed from them, so nobody comes on or off after it.
        if matches!(
            self.phase,
            MatchPhase::ExtraTimeEnd | MatchPhase::PenaltyShootout
        ) {
            return Err("be.error.liveMatch.noSubstitutionsForShootout".into());
        }

        // Cannot substitute a player who has been sent off
        if self.sent_off.contains(player_off_id) {
            return Err("be.error.liveMatch.cannotSubstituteSentOffPlayer".into());
        }

        let team = self.team_mut(side);
        let off_idx = team
            .players
            .iter()
            .position(|p| p.id == player_off_id)
            .ok_or("be.error.liveMatch.playerNotOnPitch")?;

        // Cannot bring on a player who was already substituted off
        let already_subbed_off: std::collections::HashSet<&str> = self
            .substitutions
            .iter()
            .map(|s| s.player_off_id.as_str())
            .collect();
        if already_subbed_off.contains(player_on_id) {
            return Err("be.error.liveMatch.playerAlreadySubstitutedOff".into());
        }

        // Asked before either list is disturbed, and before the bench is
        // borrowed to take the incoming player off it.
        let nobody_in_goal = self.keeper_on_the_pitch(side).is_none();

        let bench = match side {
            Side::Home => &mut self.home_bench,
            Side::Away => &mut self.away_bench,
        };
        let on_idx = bench
            .iter()
            .position(|p| p.id == player_on_id)
            .ok_or("be.error.liveMatch.playerNotOnBench")?;

        let goes_in_goal = nobody_in_goal && bench[on_idx].position == Position::Goalkeeper;

        let mut player_on = bench.remove(on_idx);
        let player_off = self.team_mut(side).players.remove(off_idx);

        // The XI is slot-aligned (entry i plays formation slot i), so the sub
        // takes over the vacated slot: same index, and the slot's position —
        // players are simulated where they actually play, not where they'd
        // naturally play.
        //
        // Except in goal. A side whose keeper has been sent off makes an
        // outfield player way for a substitute keeper, and inheriting that
        // player's slot would make him a defender: `pick_goalkeeper` looks for
        // `Position::Goalkeeper` and would never find him, so the change would
        // cost a substitution and put nobody in goal. This is the same code
        // path the player's own substitutions take, so it fixes the same
        // change made by hand from the touchline.
        if !goes_in_goal {
            player_on.position = player_off.position;
        }
        // His role comes with him from the bench, where it was judged for the line he played
        // there; in the slot he takes it counts only if that line admits it (the rule the live
        // role command asks), else he plays it as Standard, as the match was built to.
        if !player_on.role.suits(player_on.position) {
            player_on.role = PlayerRole::Standard;
        }

        // Initialize condition for incoming player
        self.player_conditions
            .insert(player_on.id.clone(), player_on.condition as f64);

        self.team_mut(side).players.insert(off_idx, player_on);

        // Move subbed-off player to bench (they can't come back, but we keep them)
        match side {
            Side::Home => self.home_bench.push(player_off),
            Side::Away => self.away_bench.push(player_off),
        }

        *match side {
            Side::Home => &mut self.home_subs_made,
            Side::Away => &mut self.away_subs_made,
        } += 1;

        // Record the substitution
        let evt = MatchEvent::new(
            self.current_minute,
            EventType::Substitution,
            side,
            Zone::Midfield,
        )
        .with_player(player_on_id)
        .with_secondary(player_off_id);
        self.unreported_events.push(evt.clone());
        self.events.push(evt);

        self.substitutions.push(SubstitutionRecord {
            minute: self.current_minute,
            side,
            player_off_id: player_off_id.to_string(),
            player_on_id: player_on_id.to_string(),
        });

        Ok(())
    }

    /// Give `player_id`, in `side`'s eleven or on its bench, `line_familiarity`. For a caller that
    /// knows where a player now plays better than the engine can: the engine knows his line, not
    /// his slot within it (`PlayerData::line_familiarity`). Whether he was found.
    pub fn set_line_familiarity(
        &mut self,
        side: Side,
        player_id: &str,
        line_familiarity: [f64; 4],
    ) -> bool {
        let (team, bench) = match side {
            Side::Home => (&mut self.home, &mut self.home_bench),
            Side::Away => (&mut self.away, &mut self.away_bench),
        };
        match team
            .players
            .iter_mut()
            .chain(bench.iter_mut())
            .find(|player| player.id == player_id)
        {
            Some(player) => {
                player.line_familiarity = line_familiarity;
                true
            }
            None => false,
        }
    }

    /// Give `player_id`, on `side`'s bench, `role`: the role he plays if he comes on. For a
    /// caller that knows a role the bench copy was built without (a role saved after the match
    /// was built). It is the caller's to check the role against the line he will enter, as
    /// `PlayerRole::suits` does. Whether he was found on the bench.
    pub fn set_bench_role(&mut self, side: Side, player_id: &str, role: PlayerRole) -> bool {
        let bench = match side {
            Side::Home => &mut self.home_bench,
            Side::Away => &mut self.away_bench,
        };
        match bench.iter_mut().find(|player| player.id == player_id) {
            Some(player) => {
                player.role = role;
                true
            }
            None => false,
        }
    }

    /// Pre-match swap: exchange a starting player with a bench player without
    /// counting as a substitution. Only valid during PreKickOff phase.
    pub(super) fn do_pre_match_swap(
        &mut self,
        side: Side,
        player_off_id: &str,
        player_on_id: &str,
    ) -> Result<(), String> {
        let team = self.team_mut(side);
        let off_idx = team
            .players
            .iter()
            .position(|p| p.id == player_off_id)
            .ok_or("be.error.liveMatch.playerNotInStartingXi")?;

        let bench = match side {
            Side::Home => &mut self.home_bench,
            Side::Away => &mut self.away_bench,
        };
        let on_idx = bench
            .iter()
            .position(|p| p.id == player_on_id)
            .ok_or("be.error.liveMatch.playerNotOnBench")?;

        let mut player_on = bench.remove(on_idx);
        let player_off = self.team_mut(side).players.remove(off_idx);

        // The XI is slot-aligned (entry i plays formation slot i). Removing the
        // outgoing player and pushing the incoming one to the END shifted every
        // later starter into a different slot and dropped the newcomer into the
        // last one — the lineup visibly "reorganized" after a swap. Keep the
        // vacated index and adopt the slot's position instead.
        player_on.position = player_off.position;

        // Initialize condition for incoming player
        self.player_conditions
            .insert(player_on.id.clone(), player_on.condition as f64);

        self.team_mut(side).players.insert(off_idx, player_on);

        // Move swapped-out player to bench
        match side {
            Side::Home => self.home_bench.push(player_off),
            Side::Away => self.away_bench.push(player_off),
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Formation mechanics
    // -----------------------------------------------------------------------

    /// Parse a formation string like "4-4-2" into (defenders, midfielders, forwards).
    pub(super) fn parse_formation(formation: &str) -> (usize, usize, usize) {
        let parts: Vec<usize> = formation
            .split('-')
            .filter_map(|s| s.parse().ok())
            .collect();
        match parts.len() {
            3 => (parts[0], parts[1], parts[2]),
            4 => (parts[0], parts[1] + parts[2], parts[3]), // e.g. 4-2-3-1
            _ => (4, 4, 2),                                 // fallback
        }
    }

    /// Apply a formation change with as few players moving as possible.
    ///
    /// A player stays in his line while the new shape has room for him. A line with too many
    /// passes its best-suited player one line toward the line that is short, so a back five takes
    /// its extra defender from midfield, never from the strikers, and a change to the same shape
    /// moves nobody. The eleven is then regrouped back to front, so player i keeps playing slot i
    /// of the new formation, which is how the pitch is drawn.
    pub(super) fn apply_formation(&mut self, side: Side, formation: &str) {
        let (num_def, num_mid, num_fwd) = Self::parse_formation(formation);
        let on_the_pitch: Vec<bool> = self
            .team_ref(side)
            .players
            .iter()
            .map(|player| self.is_on_the_pitch(side, &player.id))
            .collect();
        let team = self.team_mut(side);
        team.formation = formation.to_string();
        rebalance_lines(
            &mut team.players,
            &on_the_pitch,
            [num_def, num_mid, num_fwd],
        );
        team.players
            .sort_by_key(|player| line_rank(player.position));
    }
}

const OUTFIELD_LINES: [Position; 3] = [Position::Defender, Position::Midfielder, Position::Forward];

fn line_rank(position: Position) -> usize {
    match position {
        Position::Goalkeeper => 0,
        Position::Defender => 1,
        Position::Midfielder => 2,
        Position::Forward => 3,
    }
}

/// How well a player would fill a line, from the attributes that line asks most of.
fn suitability(player: &PlayerData, line: Position) -> u16 {
    let sum = |values: [u8; 3]| values.iter().map(|&value| u16::from(value)).sum();
    match line {
        Position::Defender => sum([player.defending, player.tackling, player.strength]),
        Position::Midfielder => sum([player.passing, player.vision, player.decisions]),
        Position::Forward => sum([player.shooting, player.dribbling, player.pace]),
        Position::Goalkeeper => 0,
    }
}

/// Move outfielders between lines until each holds `wanted` players or no line has one to
/// spare. Every move is one line, toward the nearest line that is short; a gap two lines away
/// is filled through the middle, and a player who has already moved once is the last choice to
/// move again, so the defender who steps into midfield is not then pushed up front.
///
/// Only players `on_the_pitch` (indexed as `players`) count or move: a player sent off leaves
/// his line one short, and stays where he was.
fn rebalance_lines(players: &mut [PlayerData], on_the_pitch: &[bool], wanted: [usize; 3]) {
    let mut moved: Vec<usize> = Vec::new();
    // Each move closes one place of the gap between two lines, so ten outfielders need at most
    // twenty; the bound only guards against a shape this loop cannot reach.
    for _ in 0..20 {
        let counts = OUTFIELD_LINES.map(|line| {
            players
                .iter()
                .zip(on_the_pitch)
                .filter(|&(player, &playing)| playing && player.position == line)
                .count()
        });
        let Some(short) = (0..3).find(|&line| counts[line] < wanted[line]) else {
            return;
        };
        let Some(spare) = (0..3)
            .filter(|&line| counts[line] > wanted[line])
            .min_by_key(|&line| line.abs_diff(short))
        else {
            return;
        };
        let destination = if spare < short { spare + 1 } else { spare - 1 };
        let from = OUTFIELD_LINES[spare];
        let to = OUTFIELD_LINES[destination];
        let Some(mover) = players
            .iter()
            .enumerate()
            .filter(|&(index, player)| {
                on_the_pitch.get(index) == Some(&true) && player.position == from
            })
            .rev()
            .max_by_key(|(index, player)| (!moved.contains(index), suitability(player, to)))
            .map(|(index, _)| index)
        else {
            return;
        };
        players[mover].position = to;
        moved.push(mover);
    }
}

#[cfg(test)]
mod line_familiarity_tests {
    use crate::live_match::LiveMatchState;
    use crate::types::{
        MatchConfig, PlayStyle, PlayerData, PlayerRole, Position, Side, TacticsConfig, TeamData,
    };

    pub(super) fn player(id: &str, position: Position) -> PlayerData {
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

    pub(super) fn side(prefix: &str) -> TeamData {
        TeamData {
            id: prefix.to_string(),
            name: prefix.to_string(),
            formation: "4-4-2".to_string(),
            play_style: PlayStyle::Balanced,
            tactics: TacticsConfig::default(),
            players: vec![
                player(&format!("{prefix}_gk"), Position::Goalkeeper),
                player(&format!("{prefix}_d"), Position::Defender),
            ],
        }
    }

    fn state() -> LiveMatchState {
        LiveMatchState::new(
            side("h"),
            side("a"),
            MatchConfig::default(),
            vec![player("h_sub", Position::Defender)],
            vec![player("a_sub", Position::Defender)],
            false,
        )
    }

    const REFIT: [f64; 4] = [0.9, 0.95, 1.0, 0.9];

    /// Given a home defender in the eleven,
    /// When his familiarity is set,
    /// Then he carries it, and the away side's players do not.
    #[test]
    fn a_player_in_the_eleven_takes_the_familiarity_he_is_given() {
        let mut state = state();

        assert!(state.set_line_familiarity(Side::Home, "h_d", REFIT));

        assert_eq!(state.team(Side::Home).players[1].line_familiarity, REFIT);
        assert_eq!(state.team(Side::Away).players[1].line_familiarity, [1.0; 4]);
    }

    /// Given a home substitute on the bench,
    /// When his familiarity is set,
    /// Then he carries it.
    #[test]
    fn a_player_on_the_bench_takes_the_familiarity_he_is_given() {
        let mut state = state();

        assert!(state.set_line_familiarity(Side::Home, "h_sub", REFIT));

        assert_eq!(state.bench(Side::Home)[0].line_familiarity, REFIT);
    }

    /// Given a home substitute on the bench,
    /// When he is given a role,
    /// Then he carries it, and nobody in the eleven or on the other bench does.
    #[test]
    fn a_player_on_the_bench_takes_the_role_he_is_given() {
        let mut state = state();

        assert!(state.set_bench_role(Side::Home, "h_sub", PlayerRole::Stopper));

        assert_eq!(state.bench(Side::Home)[0].role, PlayerRole::Stopper);
        assert_eq!(state.bench(Side::Away)[0].role, PlayerRole::Standard);
        assert_eq!(state.team(Side::Home).players[1].role, PlayerRole::Standard);
    }

    /// Given a home player in the eleven,
    /// When a bench role is asked for him,
    /// Then he is not found there and keeps his role.
    #[test]
    fn a_player_in_the_eleven_is_not_on_the_bench() {
        let mut state = state();

        assert!(!state.set_bench_role(Side::Home, "h_d", PlayerRole::Stopper));

        assert_eq!(state.team(Side::Home).players[1].role, PlayerRole::Standard);
    }

    /// Given the away side's substitute,
    /// When the home side's familiarity is set for him,
    /// Then nobody is found and nothing changes.
    #[test]
    fn a_player_not_on_that_side_is_not_found() {
        let mut state = state();

        assert!(!state.set_line_familiarity(Side::Home, "a_sub", REFIT));

        assert_eq!(state.bench(Side::Away)[0].line_familiarity, [1.0; 4]);
    }
}

#[cfg(test)]
mod substituted_role_tests {
    use super::line_familiarity_tests::{player, side};
    use crate::live_match::{LiveMatchState, MatchCommand};
    use crate::types::{MatchConfig, PlayerRole, Position, Side};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// The role of the player who comes on for the home defender, `bench` waiting on the home
    /// bench with `role`.
    fn role_after_coming_on(bench: Position, role: PlayerRole) -> PlayerRole {
        let mut substitute = player("h_sub", bench);
        substitute.role = role;
        let mut state = LiveMatchState::new(
            side("h"),
            side("a"),
            MatchConfig::default(),
            vec![substitute],
            vec![player("a_sub", Position::Defender)],
            false,
        );
        state.step_minute(&mut StdRng::seed_from_u64(1));
        state
            .apply_command(MatchCommand::Substitute {
                side: Side::Home,
                player_off_id: "h_d".to_string(),
                player_on_id: "h_sub".to_string(),
            })
            .expect("the substitution is legal");
        state
            .team(Side::Home)
            .players
            .iter()
            .find(|player| player.id == "h_sub")
            .map(|player| player.role)
            .expect("he is on the pitch")
    }

    /// Given a Poacher on the bench,
    /// When he comes on for a defender and takes his slot in the back line,
    /// Then he plays the slot on Standard: a Poacher is not a defender's role, and carrying it
    /// there would put the Poacher's defensive penalty on a defence.
    #[test]
    fn a_role_the_slot_does_not_admit_is_dropped_by_a_substitution() {
        assert_eq!(
            role_after_coming_on(Position::Forward, PlayerRole::Poacher),
            PlayerRole::Standard
        );
    }

    /// Given an Attacking Full-Back on the bench,
    /// When he comes on for a defender,
    /// Then he keeps the role: the slot he takes admits it.
    #[test]
    fn a_role_the_slot_admits_is_kept_by_a_substitution() {
        assert_eq!(
            role_after_coming_on(Position::Defender, PlayerRole::AttackingFB),
            PlayerRole::AttackingFB
        );
    }
}
