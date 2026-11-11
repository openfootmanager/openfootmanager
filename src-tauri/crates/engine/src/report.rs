use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::event::{EventType, MatchEvent};
use crate::types::{Side, Zone};

// ---------------------------------------------------------------------------
// TeamStats — aggregate stats for one side
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamStats {
    pub goals: u8,
    pub shots: u16,
    pub shots_on_target: u16,
    pub shots_off_target: u16,
    pub shots_blocked: u16,
    pub passes_completed: u16,
    pub passes_intercepted: u16,
    pub tackles: u16,
    pub interceptions: u16,
    pub fouls: u16,
    pub corners: u16,
    pub free_kicks: u16,
    pub penalties: u16,
    pub yellow_cards: u8,
    pub red_cards: u8,
    pub possession_ticks: u32,
}

impl TeamStats {
    pub fn pass_accuracy(&self) -> f64 {
        let total = self.passes_completed as f64 + self.passes_intercepted as f64;
        if total == 0.0 {
            return 0.0;
        }
        self.passes_completed as f64 / total * 100.0
    }
}

// ---------------------------------------------------------------------------
// PlayerMatchStats — individual player performance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerMatchStats {
    pub minutes_played: u8,
    pub goals: u8,
    pub assists: u8,
    pub shots: u8,
    pub shots_on_target: u8,
    pub passes_completed: u8,
    pub passes_attempted: u8,
    pub tackles_won: u8,
    pub interceptions: u8,
    pub fouls_committed: u8,
    pub yellow_cards: u8,
    pub red_cards: u8,
    /// Match rating 0.0–10.0, computed after the match.
    pub rating: f32,
    /// He kept a clean sheet: in goal for most of the match and his side conceded none. Set
    /// with the ratings (`MatchReport::assign_ratings`), so the mark and the season's clean
    /// sheets count by one rule.
    #[serde(default)]
    pub clean_sheet: bool,
    /// The minutes he stood in goal, whoever he is: the keeper, or an outfielder who went in
    /// after the keeper was sent off with no other left. `None` for a report that did not
    /// track it (the instant engine's), where a goalkeeper's minutes are his minutes in goal.
    #[serde(default)]
    pub minutes_in_goal: Option<u8>,
}

// ---------------------------------------------------------------------------
// GoalSource — how a goal was created (distinct from event.rs GoalContext which tracks narrative)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalSource {
    OpenPlay,
    Corner,
    FreeKick,
    Penalty,
}

// ---------------------------------------------------------------------------
// GoalDetail — enriched goal info for the report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDetail {
    pub minute: u8,
    pub scorer_id: String,
    pub assist_id: Option<String>,
    pub goal_source: GoalSource,
    pub side: Side,
}

// ---------------------------------------------------------------------------
// MatchReport — the complete output of a simulated match
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchReport {
    pub home_goals: u8,
    pub away_goals: u8,
    pub home_stats: TeamStats,
    pub away_stats: TeamStats,
    pub events: Vec<MatchEvent>,
    pub goals: Vec<GoalDetail>,
    pub player_stats: HashMap<String, PlayerMatchStats>,
    /// Possession percentage for the home team (0–100).
    pub home_possession: f64,
    /// Total simulated minutes (90 + stoppage).
    pub total_minutes: u8,
    /// Penalty-shootout score when the match went to one; `None` otherwise.
    /// Shootout kicks are never counted in `home_goals`/`away_goals`.
    #[serde(default)]
    pub home_penalties: Option<u8>,
    #[serde(default)]
    pub away_penalties: Option<u8>,
}

impl MatchReport {
    /// Build the report from the raw event log and possession counters.
    pub fn from_events(
        events: Vec<MatchEvent>,
        home_possession_ticks: u32,
        away_possession_ticks: u32,
        total_minutes: u8,
    ) -> Self {
        Self::from_events_with_players(
            events,
            home_possession_ticks,
            away_possession_ticks,
            total_minutes,
            Vec::new(),
        )
    }

    /// Build the report while also assigning minutes played for tracked players.
    pub fn from_events_with_players(
        events: Vec<MatchEvent>,
        home_possession_ticks: u32,
        away_possession_ticks: u32,
        total_minutes: u8,
        tracked_player_ids: Vec<String>,
    ) -> Self {
        let mut home_stats = TeamStats::default();
        let mut away_stats = TeamStats::default();
        let mut goals = Vec::new();
        let mut player_stats: HashMap<String, PlayerMatchStats> = HashMap::new();

        home_stats.possession_ticks = home_possession_ticks;
        away_stats.possession_ticks = away_possession_ticks;

        // State machine to determine goal source from preceding set-piece event.
        // Tracks (event_type, side) so a set piece earned by one team doesn't
        // accidentally attribute a goal scored by the other.
        let mut last_set_piece: Option<(EventType, Side)> = None;

        for event in &events {
            let stats = match event.side {
                Side::Home => &mut home_stats,
                Side::Away => &mut away_stats,
            };

            // Track set-piece window: reset on events that clear the opportunity
            match &event.event_type {
                EventType::Corner => last_set_piece = Some((EventType::Corner, event.side)),
                EventType::FreeKick => {
                    // Only dangerous free kicks count: the taking side must be in their attacking
                    // third (opponent's defensive third). A free kick in HomeDefense is only
                    // dangerous when Away is taking it, and vice-versa.
                    let dangerous_zone = match event.side {
                        Side::Home => Zone::AwayDefense,
                        Side::Away => Zone::HomeDefense,
                    };
                    if event.zone == dangerous_zone {
                        last_set_piece = Some((EventType::FreeKick, event.side));
                    }
                }
                // Defensive events clear the set-piece window
                EventType::ShotOffTarget
                | EventType::ShotBlocked
                | EventType::ShotSaved
                | EventType::PenaltyMiss
                | EventType::Clearance
                | EventType::Interception
                | EventType::PassIntercepted
                | EventType::GoalKick => last_set_piece = None,
                _ => {}
            }

            // Update player stats helper
            let pid = event.player_id.as_deref().unwrap_or("");

            match &event.event_type {
                EventType::Goal => {
                    stats.goals += 1;
                    stats.shots += 1;
                    stats.shots_on_target += 1;
                    let source = match last_set_piece.take() {
                        Some((EventType::Corner, sp_side)) if sp_side == event.side => {
                            GoalSource::Corner
                        }
                        Some((EventType::FreeKick, sp_side)) if sp_side == event.side => {
                            GoalSource::FreeKick
                        }
                        _ => GoalSource::OpenPlay,
                    };
                    goals.push(GoalDetail {
                        minute: event.minute,
                        scorer_id: pid.to_string(),
                        assist_id: event.secondary_player_id.clone(),
                        goal_source: source,
                        side: event.side,
                    });
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.goals += 1;
                        ps.shots += 1;
                        ps.shots_on_target += 1;
                    }
                    if let Some(ref assist_id) = event.secondary_player_id {
                        let ps = player_stats.entry(assist_id.clone()).or_default();
                        ps.assists += 1;
                    }
                }
                EventType::PenaltyGoal => {
                    stats.goals += 1;
                    stats.shots += 1;
                    stats.shots_on_target += 1;
                    stats.penalties += 1;
                    last_set_piece = None;
                    goals.push(GoalDetail {
                        minute: event.minute,
                        scorer_id: pid.to_string(),
                        assist_id: None,
                        goal_source: GoalSource::Penalty,
                        side: event.side,
                    });
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.goals += 1;
                        ps.shots += 1;
                        ps.shots_on_target += 1;
                    }
                }
                EventType::PenaltyMiss => {
                    stats.shots += 1;
                    stats.penalties += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.shots += 1;
                    }
                }
                EventType::ShotOnTarget | EventType::ShotSaved => {
                    stats.shots += 1;
                    stats.shots_on_target += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.shots += 1;
                        ps.shots_on_target += 1;
                    }
                }
                EventType::ShotOffTarget => {
                    stats.shots += 1;
                    stats.shots_off_target += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.shots += 1;
                    }
                }
                EventType::ShotBlocked => {
                    stats.shots += 1;
                    stats.shots_blocked += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.shots += 1;
                    }
                }
                EventType::PassCompleted => {
                    stats.passes_completed += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.passes_completed += 1;
                        ps.passes_attempted += 1;
                    }
                }
                EventType::PassIntercepted => {
                    stats.passes_intercepted += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.passes_attempted += 1;
                    }
                }
                EventType::Tackle => {
                    stats.tackles += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.tackles_won += 1;
                    }
                }
                EventType::Interception => {
                    stats.interceptions += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.interceptions += 1;
                    }
                }
                EventType::Foul => {
                    stats.fouls += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.fouls_committed += 1;
                    }
                }
                EventType::YellowCard => {
                    stats.yellow_cards += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.yellow_cards += 1;
                    }
                }
                // A second yellow is a booking and a sending-off: the red-card tally is how the
                // season stats and the post-match morale see a dismissal.
                EventType::SecondYellow => {
                    stats.yellow_cards += 1;
                    stats.red_cards += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.yellow_cards += 1;
                        ps.red_cards += 1;
                    }
                }
                EventType::RedCard => {
                    stats.red_cards += 1;
                    if !pid.is_empty() {
                        let ps = player_stats.entry(pid.to_string()).or_default();
                        ps.red_cards += 1;
                    }
                }
                EventType::Corner => {
                    stats.corners += 1;
                }
                EventType::FreeKick => {
                    stats.free_kicks += 1;
                }
                EventType::PenaltyAwarded => {
                    stats.penalties += 1;
                }
                // Shootout kicks are intentionally excluded from goals,
                // GoalDetails, and player stats — the shootout is scored
                // separately via home_penalties/away_penalties.
                EventType::ShootoutGoal | EventType::ShootoutMiss => {}
                _ => {}
            }
        }

        populate_minutes_played(
            &events,
            total_minutes,
            &tracked_player_ids,
            &mut player_stats,
        );

        let total_poss = home_possession_ticks + away_possession_ticks;
        let home_possession = if total_poss > 0 {
            home_possession_ticks as f64 / total_poss as f64 * 100.0
        } else {
            50.0
        };

        Self {
            home_goals: home_stats.goals,
            away_goals: away_stats.goals,
            home_stats,
            away_stats,
            events,
            goals,
            player_stats,
            home_possession,
            total_minutes,
            home_penalties: None,
            away_penalties: None,
        }
    }
}

fn populate_minutes_played(
    events: &[MatchEvent],
    total_minutes: u8,
    tracked_player_ids: &[String],
    player_stats: &mut HashMap<String, PlayerMatchStats>,
) {
    let mut intervals: HashMap<String, (u8, u8)> = tracked_player_ids
        .iter()
        .cloned()
        .map(|player_id| (player_id, (0, total_minutes)))
        .collect();

    for event in events {
        match event.event_type {
            EventType::Substitution => {
                if let Some(ref player_off_id) = event.secondary_player_id {
                    let interval = intervals
                        .entry(player_off_id.clone())
                        .or_insert((0, total_minutes));
                    interval.1 = interval.1.min(event.minute.min(total_minutes));
                }
                if let Some(ref player_on_id) = event.player_id {
                    intervals.insert(
                        player_on_id.clone(),
                        (event.minute.min(total_minutes), total_minutes),
                    );
                }
            }
            EventType::RedCard | EventType::SecondYellow => {
                if let Some(ref player_id) = event.player_id {
                    intervals
                        .entry(player_id.clone())
                        .and_modify(|interval| interval.1 = interval.1.min(event.minute))
                        .or_insert((0, event.minute.min(total_minutes)));
                }
            }
            _ => {}
        }
    }

    for (player_id, (entry, exit)) in intervals {
        player_stats.entry(player_id).or_default().minutes_played = exit.saturating_sub(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(minute: u8, event_type: EventType, side: Side, player: &str) -> MatchEvent {
        MatchEvent::new(minute, event_type, side, Zone::attacking_box(side)).with_player(player)
    }

    fn substitution(minute: u8, on: &str, off: &str) -> MatchEvent {
        event(minute, EventType::Substitution, Side::Home, on).with_secondary(off)
    }

    fn reported_minutes(events: Vec<MatchEvent>, player: &str) -> u8 {
        MatchReport::from_events_with_players(events, 0, 0, 90, vec![player.to_string()])
            .player_stats[player]
            .minutes_played
    }

    /// Given a substitute sent off five minutes after entering, when reported, then credit five minutes.
    #[test]
    fn substitute_sent_off_only_plays_until_dismissal() {
        assert_eq!(
            reported_minutes(
                vec![
                    substitution(60, "sub", "starter"),
                    event(65, EventType::RedCard, Side::Home, "sub")
                ],
                "sub"
            ),
            5
        );
    }

    /// Given a substitute later replaced, when reported, then credit only his time on the pitch.
    #[test]
    fn substitute_replaced_only_plays_until_exit() {
        assert_eq!(
            reported_minutes(
                vec![
                    substitution(60, "sub", "starter"),
                    substitution(80, "other", "sub")
                ],
                "sub"
            ),
            20
        );
    }

    /// Given a starter replaced at 60, when reported, then credit 60 minutes.
    #[test]
    fn starter_replaced_plays_until_exit() {
        assert_eq!(
            reported_minutes(vec![substitution(60, "sub", "starter")], "starter"),
            60
        );
    }

    /// Given a starter sent off at 65, when reported, then credit 65 minutes.
    #[test]
    fn starter_sent_off_plays_until_dismissal() {
        assert_eq!(
            reported_minutes(
                vec![event(65, EventType::SecondYellow, Side::Home, "starter")],
                "starter"
            ),
            65
        );
    }

    /// Given a starter still playing at full time, when reported, then credit the whole match.
    #[test]
    fn starter_remaining_plays_the_whole_match() {
        assert_eq!(reported_minutes(vec![], "starter"), 90);
    }

    // Regression: shootout kicks used to be counted as match goals, inflating
    // the scoreline (1-1 won 4-3 on pens was reported as 5-4) and the
    // scorers' goal tallies.
    #[test]
    fn shootout_kicks_are_not_goals() {
        let events = vec![
            event(20, EventType::Goal, Side::Home, "h1"),
            event(55, EventType::PenaltyGoal, Side::Away, "a1"),
            // Shootout after extra time
            event(121, EventType::ShootoutGoal, Side::Home, "h1"),
            event(121, EventType::ShootoutGoal, Side::Away, "a2"),
            event(122, EventType::ShootoutGoal, Side::Home, "h2"),
            event(122, EventType::ShootoutMiss, Side::Away, "a3"),
        ];
        let report = MatchReport::from_events(events, 50, 50, 120);

        assert_eq!(report.home_goals, 1);
        assert_eq!(report.away_goals, 1);
        assert_eq!(
            report.goals.len(),
            2,
            "GoalDetails must exclude shootout kicks"
        );
        assert_eq!(report.player_stats["h1"].goals, 1);
        assert_eq!(report.player_stats["a1"].goals, 1);
        assert!(
            report.player_stats.get("h2").is_none_or(|p| p.goals == 0),
            "shootout-only kicker must not be credited a goal"
        );
        // In-match penalties still count.
        assert_eq!(report.goals[1].goal_source, GoalSource::Penalty);
    }

    /// Given a player booked at 20' and shown a second yellow at 60',
    /// When the match report is built,
    /// Then he has two yellow cards and one red, and so has his side: a second yellow is a
    /// sending-off, and the red-card tally is how the season stats and morale see one.
    #[test]
    fn a_second_yellow_is_a_red_card_in_the_report() {
        let events = vec![
            event(20, EventType::YellowCard, Side::Home, "h1"),
            event(60, EventType::SecondYellow, Side::Home, "h1"),
        ];
        let report = MatchReport::from_events(events, 50, 50, 90);

        let sent_off = &report.player_stats["h1"];
        assert_eq!((sent_off.yellow_cards, sent_off.red_cards), (2, 1));
        assert_eq!(
            (report.home_stats.yellow_cards, report.home_stats.red_cards),
            (2, 1)
        );
    }
}
