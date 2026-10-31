use crate::event::{EventType, MatchEvent};
use crate::types::{PlayerData, Position, Side, Zone};

use super::{LiveMatchState, SubstitutionRecord};

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
        let team = self.team_mut(side);
        team.formation = formation.to_string();
        rebalance_lines(&mut team.players, [num_def, num_mid, num_fwd]);
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
fn rebalance_lines(players: &mut [PlayerData], wanted: [usize; 3]) {
    let mut moved: Vec<usize> = Vec::new();
    // Each move closes one place of the gap between two lines, so ten outfielders need at most
    // twenty; the bound only guards against a shape this loop cannot reach.
    for _ in 0..20 {
        let counts = OUTFIELD_LINES.map(|line| {
            players
                .iter()
                .filter(|player| player.position == line)
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
            .filter(|(_, player)| player.position == from)
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
