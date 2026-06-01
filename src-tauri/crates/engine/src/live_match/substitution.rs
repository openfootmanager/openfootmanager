use crate::event::{EventType, MatchEvent};
use crate::types::{Position, Side, Zone};

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

        let bench = match side {
            Side::Home => &mut self.home_bench,
            Side::Away => &mut self.away_bench,
        };
        let on_idx = bench
            .iter()
            .position(|p| p.id == player_on_id)
            .ok_or("be.error.liveMatch.playerNotOnBench")?;

        let player_on = bench.remove(on_idx);
        let player_off = self.team_mut(side).players.remove(off_idx);

        // Initialize condition for incoming player
        self.player_conditions
            .insert(player_on.id.clone(), player_on.condition as f64);

        self.team_mut(side).players.push(player_on);

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

        let player_on = bench.remove(on_idx);
        let player_off = self.team_mut(side).players.remove(off_idx);

        // Initialize condition for incoming player
        self.player_conditions
            .insert(player_on.id.clone(), player_on.condition as f64);

        self.team_mut(side).players.push(player_on);

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

    /// Apply a formation change: update the formation string and redistribute
    /// outfield player positions to match the new shape.
    pub(super) fn apply_formation(&mut self, side: Side, formation: &str) {
        let (num_def, num_mid, num_fwd) = Self::parse_formation(formation);
        let team = self.team_mut(side);
        team.formation = formation.to_string();

        // Collect outfield players (skip GK) sorted by defensive-ness
        // (defenders first, then midfielders, then forwards) using a simple
        // heuristic: defending+tackling vs shooting+dribbling
        let mut outfield_indices: Vec<usize> = team
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.position != Position::Goalkeeper)
            .map(|(i, _)| i)
            .collect();

        // Sort by defensive score descending (most defensive first)
        outfield_indices.sort_by(|&a, &b| {
            let pa = &team.players[a];
            let pb = &team.players[b];
            let def_a = (pa.defending as u16 + pa.tackling as u16 + pa.strength as u16) as f64;
            let def_b = (pb.defending as u16 + pb.tackling as u16 + pb.strength as u16) as f64;
            def_b
                .partial_cmp(&def_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign positions: first num_def → Defender, next num_mid → Midfielder, rest → Forward
        for (slot, &idx) in outfield_indices.iter().enumerate() {
            let new_pos = if slot < num_def {
                Position::Defender
            } else if slot < num_def + num_mid {
                Position::Midfielder
            } else if slot < num_def + num_mid + num_fwd {
                Position::Forward
            } else {
                // Extra players (e.g. if team has <11 due to red cards) keep current
                continue;
            };
            team.players[idx].position = new_pos;
        }
    }
}
