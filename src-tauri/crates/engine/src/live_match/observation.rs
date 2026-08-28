//! What one touchline can see — the AI manager's view of the match.
//!
//! The sibling of [`super::snapshot`], and deliberately its opposite. A
//! `MatchSnapshot` is built for the UI: it owns everything, because it crosses
//! an IPC boundary and outlives the state it came from. An `AiObservation` is
//! built for a decision taken and discarded inside the same tick, so it borrows.
//!
//! The AI used to take the snapshot, three times a call, once a minute, for each
//! side. Everything about that was wasted except about eight scalars, one team,
//! one bench and the sent-off set — and one part of it was worse than wasted:
//! the snapshot carries the whole accumulated event log, so the cost of asking
//! the manager a question grew with how long the match had been going, and the
//! cost of a match grew with the square of its length.
//!
//! The view is also *side-relative*, which the snapshot could not be. Every
//! caller previously re-derived "my team", "my bench", "my goal difference" and
//! "how long the ball has been in my half" from a neutral view; here they are
//! resolved once.

use std::collections::HashSet;

use super::{LiveMatchState, MatchPhase, SubstitutionRecord};
use crate::types::{PlayerData, Side, TeamData, Zone};

/// Everything one AI manager can see, resolved for that manager's side.
pub(crate) struct AiObservation<'a> {
    pub(crate) phase: MatchPhase,
    pub(crate) minute: u8,
    pub(crate) side: Side,
    /// Goals scored minus goals conceded, from this touchline's point of view.
    pub(crate) goal_diff: i8,
    pub(crate) subs_made: u8,
    pub(crate) max_subs: u8,
    pub(crate) team: &'a TeamData,
    pub(crate) bench: &'a [PlayerData],
    sent_off: &'a HashSet<String>,
    /// Every substitution the match has seen, both sides. Read only to find out
    /// who has already been taken off — see [`AiObservation::available`].
    substitutions: &'a [SubstitutionRecord],
    /// How many of the last ten minutes the ball spent in this side's own half.
    pub(crate) pressure_ticks: usize,
    /// Live per-minute condition, which is not what `TeamData` carries: the
    /// stored value is what the player started with.
    conditions: &'a std::collections::HashMap<String, f64>,
}

impl AiObservation<'_> {
    /// Can this player take part — now, or later from the bench?
    ///
    /// Two ways to be finished with a match: sent off, or already substituted.
    /// The second is easy to miss, because a substituted player is pushed back
    /// onto the bench list (he has to go somewhere, and the UI lists him), where
    /// a tired star still outranks every reserve. `do_substitution` refuses to
    /// bring him on, so a manager who overlooks this does not break the match —
    /// he just keeps proposing the one substitution he cannot make instead of
    /// the ones he can.
    pub(crate) fn available(&self, player: &PlayerData) -> bool {
        !self.sent_off.contains(&player.id)
            && !self
                .substitutions
                .iter()
                .any(|sub| sub.player_off_id == player.id)
    }

    /// This player's condition as the manager sees it right now.
    ///
    /// Rounded to a whole number before being read back as a float, which looks
    /// like a pointless round trip and is not: the snapshot this replaced stored
    /// condition as a `u8`, so every threshold in the AI has always been
    /// compared against a rounded value. Handing over the raw f64 would move
    /// every sub decision sitting within half a point of a threshold, and
    /// reshuffle every seeded match in the game.
    pub(crate) fn condition_of(&self, player: &PlayerData) -> f64 {
        match self.conditions.get(&player.id) {
            Some(live) => f64::from(live.round() as u8),
            None => f64::from(player.condition),
        }
    }
}

impl LiveMatchState {
    /// The view from one touchline.
    pub(crate) fn observe(&self, side: Side) -> AiObservation<'_> {
        let (own_goals, opp_goals, team, subs_made, own_half) = match side {
            Side::Home => (
                self.home_score,
                self.away_score,
                &self.home,
                self.home_subs_made,
                [Zone::HomeBox, Zone::HomeDefense],
            ),
            Side::Away => (
                self.away_score,
                self.home_score,
                &self.away,
                self.away_subs_made,
                [Zone::AwayBox, Zone::AwayDefense],
            ),
        };

        AiObservation {
            phase: self.phase,
            minute: self.current_minute,
            side,
            goal_diff: own_goals as i8 - opp_goals as i8,
            subs_made,
            max_subs: self.max_subs,
            team,
            bench: self.bench(side),
            sent_off: &self.sent_off,
            substitutions: &self.substitutions,
            pressure_ticks: self
                .recent_zones
                .iter()
                .filter(|zone| own_half.contains(zone))
                .count(),
            conditions: &self.player_conditions,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::ai::{AiPersonality, AiProfile, ai_decide};
    use crate::live_match::LiveMatchState;
    use crate::live_match::snapshot::SNAPSHOTS_BUILT;
    use crate::types::{
        MatchConfig, PlayStyle, PlayerData, PlayerRole, Position, Side, TacticsConfig, TeamData,
    };

    fn snapshots_built_during(body: impl FnOnce()) -> usize {
        let before = SNAPSHOTS_BUILT.with(Cell::get);
        body();
        SNAPSHOTS_BUILT.with(Cell::get) - before
    }

    fn make_player(id: &str, position: Position) -> PlayerData {
        PlayerData {
            id: id.to_string(),
            name: id.to_string(),
            position,
            ovr: 70,
            condition: 90,
            fitness: 75,
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

    fn make_team(id: &str) -> TeamData {
        let mut players = vec![make_player(&format!("{id}_gk"), Position::Goalkeeper)];
        for i in 0..4 {
            players.push(make_player(&format!("{id}_def{i}"), Position::Defender));
        }
        for i in 0..4 {
            players.push(make_player(&format!("{id}_mid{i}"), Position::Midfielder));
        }
        for i in 0..2 {
            players.push(make_player(&format!("{id}_fwd{i}"), Position::Forward));
        }
        TeamData {
            id: id.to_string(),
            name: id.to_string(),
            formation: "4-4-2".to_string(),
            play_style: PlayStyle::Balanced,
            tactics: TacticsConfig::default(),
            players,
        }
    }

    fn make_bench(id: &str) -> Vec<PlayerData> {
        vec![
            make_player(&format!("{id}_sub_gk"), Position::Goalkeeper),
            make_player(&format!("{id}_sub_def"), Position::Defender),
            make_player(&format!("{id}_sub_mid"), Position::Midfielder),
            make_player(&format!("{id}_sub_fwd"), Position::Forward),
        ]
    }

    fn make_match() -> LiveMatchState {
        LiveMatchState::new(
            make_team("home"),
            make_team("away"),
            MatchConfig::default(),
            make_bench("home"),
            make_bench("away"),
            false,
        )
    }

    fn profile() -> AiProfile {
        AiProfile {
            reputation: 500,
            experience: 50,
            personality: AiPersonality::Pragmatist,
        }
    }

    /// The instrument, before the measurements that lean on it.
    ///
    /// Every assertion below is that a count came out zero, and a counter that
    /// was never wired into `snapshot()` would satisfy all of them while proving
    /// nothing at all. This is the test that fails if the increment is deleted.
    #[test]
    fn the_counter_sees_a_snapshot_being_built() {
        let state = make_match();

        let built = snapshots_built_during(|| {
            std::hint::black_box(state.snapshot());
        });

        assert_eq!(
            built, 1,
            "one call to snapshot() must register as one snapshot, or the \
             zero counts asserted below mean nothing"
        );
    }

    /// Asking the manager a question must not copy the match.
    ///
    /// Not "must copy it less" — none. The observation borrows, so there is no
    /// number of snapshots here that would be correct, and a threshold would
    /// only give something room to creep back underneath it.
    #[test]
    fn asking_the_manager_does_not_copy_the_match() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut state = make_match();
        let profile = profile();

        // Late in the match, where the event log a snapshot carries is longest
        // and the manager has the most to weigh.
        while state.minute() < 85 && !state.is_finished() {
            state.step_minute(&mut rng);
        }

        let built = snapshots_built_during(|| {
            let mut ask = StdRng::seed_from_u64(1);
            std::hint::black_box(ai_decide(&state, Side::Home, &profile, &mut ask));
        });

        assert_eq!(
            built, 0,
            "one AI decision built {built} snapshot(s) of the match. The manager \
             is meant to be reading the match, not copying it."
        );
    }

    /// A per-call cost too small to notice on its own is still charged about a
    /// hundred and eighty times a match — and every match in the world takes
    /// this path now, rather than one a matchday.
    #[test]
    fn a_whole_match_of_decisions_does_not_copy_the_match() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut state = make_match();
        let profile = profile();

        let built = snapshots_built_during(|| {
            loop {
                if state.step_minute(&mut rng).is_finished {
                    break;
                }
                let minute = state.minute();
                let mut ask = StdRng::seed_from_u64(u64::from(minute));
                std::hint::black_box(ai_decide(&state, Side::Home, &profile, &mut ask));
                std::hint::black_box(ai_decide(&state, Side::Away, &profile, &mut ask));
            }
        });

        assert_eq!(
            built, 0,
            "a match's worth of AI decisions built {built} snapshot(s)"
        );
    }
}
