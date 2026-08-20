//! AI-only daily training policy.
//!
//! Applies automated training focus and intensity decisions to all non-user teams
//! each non-match training day, BEFORE `training::process_training` runs.
//!
//! Every reading here is taken across the eleven best available players rather
//! than the whole squad. A squad mean is the average of a group most of whom are
//! not going to play: a first eleven worn down to 50 sitting behind eleven
//! untouched reserves reads as a comfortable 75, and the planner answers by
//! putting that same eleven through a hard session. See
//! [`likely_starters_condition`].
//!
//! Algorithm:
//! 1. Skip the user-controlled team entirely.
//! 2. Skip if today is a rest day for that team's schedule.
//! 3. If the likely starters' condition < 10 → Recovery focus + Low intensity (no cycle advance).
//! 4. Otherwise compute intensity from that same reading's band (Low/Medium/High).
//! 5. Apply near-match / congestion downgrade where applicable.
//! 6. Pick focus from the style-biased 5-slot weekly cycle (indexed by weekday % 5).
//! 7. Force Recovery focus when final intensity is Low (fatigue band or downgraded).
//! 8. Force Tactical focus when congested but intensity is still Medium.
//! 9. V1 safety rule: Physical + High → downgrade intensity to Medium.

use crate::game::Game;
use domain::team::{PlayStyle, TrainingFocus, TrainingIntensity};

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

/// Below this avg condition: full recovery day (no cycle advance).
const RECOVERY_CRISIS_THRESHOLD: f64 = 10.0;
/// Below this avg condition: Low intensity band.
const LOW_INTENSITY_MAX: f64 = 40.0;
/// Above this avg condition: High intensity band (40–70 inclusive is Medium).
const HIGH_INTENSITY_MIN: f64 = 70.0;

// ---------------------------------------------------------------------------
// Style-biased weekly cycle
// ---------------------------------------------------------------------------

/// Returns a 5-slot weekly focus cycle for the given play style.
///
/// Indexed by `weekday_num % 5`:
///   0 = Mon slot, 1 = Tue slot, 2 = Wed slot, 3 = Thu slot, 4 = Fri slot.
///   With modulo-5 indexing, Saturday (`weekday_num = 5`) wraps to slot 0
///   (the Monday slot), so the 5-slot cycle repeats Monday-Friday.
///
/// Balanced uses one of each focus.
/// Every other style has 3 slots for its biased focus (2 extras vs the base).
fn style_weekly_cycle(play_style: &PlayStyle) -> [TrainingFocus; 5] {
    match play_style {
        PlayStyle::Balanced => [
            TrainingFocus::Physical,
            TrainingFocus::Technical,
            TrainingFocus::Tactical,
            TrainingFocus::Defending,
            TrainingFocus::Attacking,
        ],
        PlayStyle::Attacking => [
            TrainingFocus::Physical,
            TrainingFocus::Technical,
            TrainingFocus::Attacking,
            TrainingFocus::Attacking,
            TrainingFocus::Attacking,
        ],
        PlayStyle::Defensive => [
            TrainingFocus::Physical,
            TrainingFocus::Technical,
            TrainingFocus::Defending,
            TrainingFocus::Defending,
            TrainingFocus::Defending,
        ],
        PlayStyle::Possession => [
            TrainingFocus::Physical,
            TrainingFocus::Technical,
            TrainingFocus::Tactical,
            TrainingFocus::Tactical,
            TrainingFocus::Tactical,
        ],
        PlayStyle::HighPress => [
            TrainingFocus::Physical,
            TrainingFocus::Physical,
            TrainingFocus::Physical,
            TrainingFocus::Technical,
            TrainingFocus::Tactical,
        ],
        PlayStyle::Counter => [
            TrainingFocus::Physical,
            TrainingFocus::Technical,
            TrainingFocus::Technical,
            TrainingFocus::Technical,
            TrainingFocus::Tactical,
        ],
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// How many players the readiness reading is taken across: a starting eleven.
const LIKELY_STARTERS: usize = 11;

/// Average condition of the eleven best players this club has available.
///
/// This is the number the intensity bands are computed from, and it is
/// deliberately *not* the squad average. A squad average is dominated by
/// whoever is not playing: a first eleven ground down to 50 sitting behind
/// eleven untouched reserves reads as a comfortable 75, so the planner sees
/// room to work and puts the same exhausted eleven through a High session. The
/// gap between the two numbers is not small — over a simulated season an AI
/// squad averages around 71 while its best eleven arrive at matches in the
/// high 40s.
///
/// It is also deliberately not the side the lineup picker would actually name.
/// That side is rotation-adjusted: the tired stars have been left out of it, so
/// reading it would launder the fatigue back out of the signal exactly the way
/// a fresh bench launders it out of the squad mean. The question worth asking
/// is "what condition are this club's best players in", and the answer has to
/// stay uncomfortable while they are tired.
///
/// Injured players are excluded — they are not candidates for anything, and
/// their condition is being managed by the treatment room rather than by the
/// training ground. `ovr` is position-weighted, so a goalkeeper is comparable
/// with an outfielder and the eleven cannot degenerate into one shape. Ties
/// break on player id so a squad of equals gives the same reading every day.
/// A club with nothing to read is reported fully fit rather than in crisis:
/// there is nobody for a lighter session to protect.
fn likely_starters_condition(game: &Game, team_id: &str) -> f64 {
    let mut available: Vec<(u8, &str, u8)> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.injury.is_none())
        .map(|p| (p.ovr, p.id.as_str(), p.condition))
        .collect();

    if available.is_empty() {
        return 100.0;
    }

    available.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let eleven = &available[..LIKELY_STARTERS.min(available.len())];

    eleven.iter().map(|(_, _, c)| *c as f64).sum::<f64>() / eleven.len() as f64
}

// ---------------------------------------------------------------------------
// Per-team snapshot (immutable read, no borrows retained)
// ---------------------------------------------------------------------------

struct TeamSnapshot {
    /// Average condition of the eleven best available players — see
    /// [`likely_starters_condition`] for why it is not the squad average.
    starters_condition: f64,
    play_style: PlayStyle,
    is_training_day: bool,
}

fn snapshot_team(game: &Game, team_id: &str, weekday_num: u32) -> TeamSnapshot {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (play_style, schedule) = team
        .map(|t| (t.play_style.clone(), t.training_schedule.clone()))
        .unwrap_or((
            PlayStyle::Balanced,
            domain::team::TrainingSchedule::Balanced,
        ));

    let is_training_day = schedule.is_training_day(weekday_num);

    TeamSnapshot {
        starters_condition: likely_starters_condition(game, team_id),
        play_style,
        is_training_day,
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Compute and apply AI training decisions to every non-user team.
///
/// Must be called BEFORE `training::process_training` so that the planner's
/// chosen focus and intensity are in effect when training effects are applied.
pub fn apply_ai_training_policies(game: &mut Game, weekday_num: u32) {
    let user_team_id = game.manager.team_id.clone();
    // A club playing today is not training today, so there is no session for a
    // plan to describe. Skipping it also keeps the daily write off the record for
    // clubs that would only have it overwritten unread.
    let playing_today = crate::training::teams_playing_on(
        game,
        &game.clock.current_date.format("%Y-%m-%d").to_string(),
    );

    // Collect AI team IDs up front to avoid borrow conflicts.
    let team_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|t| Some(&t.id) != user_team_id.as_ref())
        .filter(|t| !playing_today.contains(&t.id))
        .map(|t| t.id.clone())
        .collect();

    for team_id in team_ids {
        // Snapshot reads immutably and returns owned data — no borrow retained.
        let snap = snapshot_team(game, &team_id, weekday_num);

        // Rest day: no-op.
        if !snap.is_training_day {
            continue;
        }

        // Recovery crisis: full recovery day, cycle does NOT advance.
        if snap.starters_condition < RECOVERY_CRISIS_THRESHOLD {
            if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
                team.training_focus = TrainingFocus::Recovery;
                team.training_intensity = TrainingIntensity::Low;
            }
            continue;
        }

        // Base intensity from condition band.
        let base_intensity = if snap.starters_condition < LOW_INTENSITY_MAX {
            TrainingIntensity::Low
        } else if snap.starters_condition <= HIGH_INTENSITY_MIN {
            TrainingIntensity::Medium
        } else {
            TrainingIntensity::High
        };

        // The near-match / congestion taper used to live here. It now lives in
        // `training::is_tapering`, which applies it to every club — the human's
        // included — because reducing load before a game is a property of
        // training, not a manager's insight. What stays here is what a manager
        // genuinely decides: how hard to work when there is room to, and on what.
        let intensity = base_intensity;

        // Style-biased weekly cycle; slot is based on weekday mod 5.
        let cycle = style_weekly_cycle(&snap.play_style);
        let slot = (weekday_num as usize) % 5;
        let rotation_focus = cycle[slot].clone();

        // Focus override based on effective intensity.
        let focus = match &intensity {
            // Low intensity band (a tired squad) → recovery-first.
            TrainingIntensity::Low => TrainingFocus::Recovery,
            // Healthy band → follow style-biased rotation.
            _ => rotation_focus,
        };

        // V1 safety rule: Physical + High is the most punishing combination.
        let intensity = if focus == TrainingFocus::Physical && intensity == TrainingIntensity::High
        {
            TrainingIntensity::Medium
        } else {
            intensity
        };

        if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
            team.training_focus = focus;
            team.training_intensity = intensity;
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::{Team, TrainingFocus, TrainingIntensity, TrainingSchedule};

    fn default_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: 65,
            tackling: 65,
            dribbling: 65,
            defending: 65,
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: 20,
            reflexes: 30,
            aerial: 60,
        }
    }

    fn make_player(id: &str, team_id: &str, condition: u8) -> Player {
        let mut p = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1995-01-01".to_string(),
            "ENG".to_string(),
            Position::Midfielder,
            default_attrs(),
        );
        p.team_id = Some(team_id.to_string());
        p.condition = condition;
        p
    }

    fn make_team(id: &str, play_style: PlayStyle) -> Team {
        let mut t = Team::new(
            id.to_string(),
            id.to_string(),
            id[..3.min(id.len())].to_string(),
            "England".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            40_000,
        );
        t.play_style = play_style;
        t.training_schedule = TrainingSchedule::Balanced;
        t.training_focus = TrainingFocus::Physical;
        t.training_intensity = TrainingIntensity::Medium;
        t
    }

    fn make_manager(team_id: Option<&str>) -> Manager {
        let mut m = Manager::new(
            "mgr1".to_string(),
            "User".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "ENG".to_string(),
        );
        if let Some(tid) = team_id {
            m.hire(tid.to_string());
        }
        m
    }

    fn make_game_with_two_teams(
        user_team_id: &str,
        ai_team_id: &str,
        ai_play_style: PlayStyle,
        ai_condition: u8,
    ) -> Game {
        // Monday 2026-06-15
        let date = Utc.with_ymd_and_hms(2026, 6, 15, 12, 0, 0).unwrap();
        let clock = GameClock::new(date);
        let manager = make_manager(Some(user_team_id));

        let user_team = make_team(user_team_id, PlayStyle::Balanced);
        let mut ai_team = make_team(ai_team_id, ai_play_style);
        ai_team.training_focus = TrainingFocus::Physical;
        ai_team.training_intensity = TrainingIntensity::Medium;

        let players: Vec<Player> = (0..5)
            .flat_map(|i| {
                vec![
                    make_player(&format!("u{}", i), user_team_id, 80),
                    make_player(&format!("a{}", i), ai_team_id, ai_condition),
                ]
            })
            .collect();

        Game::new(
            clock,
            manager,
            vec![user_team, ai_team],
            players,
            vec![],
            vec![],
        )
    }

    // -----------------------------------------------------------------------
    // Rest-day guard
    // -----------------------------------------------------------------------

    #[test]
    fn rest_day_does_not_change_ai_settings() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 80);
        game.teams
            .iter_mut()
            .find(|t| t.id == "ai")
            .unwrap()
            .training_focus = TrainingFocus::Defending;

        // Wednesday (2) is a rest day for Balanced schedule
        apply_ai_training_policies(&mut game, 2);

        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(
            ai.training_focus,
            TrainingFocus::Defending,
            "rest day must not change focus"
        );
    }

    // -----------------------------------------------------------------------
    // User team guard
    // -----------------------------------------------------------------------

    #[test]
    fn user_team_is_never_mutated() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 80);
        game.teams
            .iter_mut()
            .find(|t| t.id == "user")
            .unwrap()
            .training_focus = TrainingFocus::Defending;
        game.teams
            .iter_mut()
            .find(|t| t.id == "user")
            .unwrap()
            .training_intensity = TrainingIntensity::High;

        // Monday (0) is a training day
        apply_ai_training_policies(&mut game, 0);

        let user = game.teams.iter().find(|t| t.id == "user").unwrap();
        assert_eq!(user.training_focus, TrainingFocus::Defending);
        assert_eq!(user.training_intensity, TrainingIntensity::High);
    }

    // -----------------------------------------------------------------------
    // Recovery crisis (< 10)
    // -----------------------------------------------------------------------

    #[test]
    fn recovery_crisis_sets_recovery_focus_and_low_intensity() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 5);

        apply_ai_training_policies(&mut game, 0); // Monday

        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(ai.training_focus, TrainingFocus::Recovery);
        assert_eq!(ai.training_intensity, TrainingIntensity::Low);
    }

    // -----------------------------------------------------------------------
    // Condition band → intensity
    // -----------------------------------------------------------------------

    #[test]
    fn avg_condition_39_gives_low_intensity() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 39);
        apply_ai_training_policies(&mut game, 0);
        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(ai.training_intensity, TrainingIntensity::Low);
    }

    #[test]
    fn avg_condition_40_gives_medium_intensity() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 40);
        apply_ai_training_policies(&mut game, 0);
        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(ai.training_intensity, TrainingIntensity::Medium);
    }

    #[test]
    fn avg_condition_71_gives_high_intensity() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 71);
        // Use Tuesday (weekday 1, Balanced schedule trains Tue) → slot 1 = Technical.
        // Technical + High does not trigger the safety rule, so intensity stays High.
        apply_ai_training_policies(&mut game, 1);
        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(ai.training_intensity, TrainingIntensity::High);
    }

    // -----------------------------------------------------------------------
    // Style-biased rotation on healthy squad
    // -----------------------------------------------------------------------

    #[test]
    fn balanced_style_rotates_through_all_five_focuses() {
        let focuses: Vec<TrainingFocus> = (0..5)
            .map(|slot| style_weekly_cycle(&PlayStyle::Balanced)[slot].clone())
            .collect();
        assert!(focuses.contains(&TrainingFocus::Physical));
        assert!(focuses.contains(&TrainingFocus::Technical));
        assert!(focuses.contains(&TrainingFocus::Tactical));
        assert!(focuses.contains(&TrainingFocus::Defending));
        assert!(focuses.contains(&TrainingFocus::Attacking));
    }

    #[test]
    fn attacking_style_has_three_attacking_slots() {
        let cycle = style_weekly_cycle(&PlayStyle::Attacking);
        let attacking_count = cycle
            .iter()
            .filter(|f| **f == TrainingFocus::Attacking)
            .count();
        assert_eq!(attacking_count, 3);
    }

    #[test]
    fn high_press_style_has_three_physical_slots() {
        let cycle = style_weekly_cycle(&PlayStyle::HighPress);
        let physical_count = cycle
            .iter()
            .filter(|f| **f == TrainingFocus::Physical)
            .count();
        assert_eq!(physical_count, 3);
    }

    #[test]
    fn counter_style_has_three_technical_slots() {
        let cycle = style_weekly_cycle(&PlayStyle::Counter);
        let technical_count = cycle
            .iter()
            .filter(|f| **f == TrainingFocus::Technical)
            .count();
        assert_eq!(technical_count, 3);
    }

    #[test]
    fn possession_style_has_three_tactical_slots() {
        let cycle = style_weekly_cycle(&PlayStyle::Possession);
        let tactical_count = cycle
            .iter()
            .filter(|f| **f == TrainingFocus::Tactical)
            .count();
        assert_eq!(tactical_count, 3);
    }

    #[test]
    fn defensive_style_has_three_defending_slots() {
        let cycle = style_weekly_cycle(&PlayStyle::Defensive);
        let defending_count = cycle
            .iter()
            .filter(|f| **f == TrainingFocus::Defending)
            .count();
        assert_eq!(defending_count, 3);
    }

    #[test]
    fn healthy_squad_attacking_style_uses_attacking_focus_on_slot2() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Attacking, 80);
        // Slot 2 (weekday % 5 == 2) is Attacking for Attacking style
        // weekday 2 = Wednesday; for Balanced schedule that's a rest day.
        // Use weekday 7 % 5 = 2 to test slot logic without a rest-day veto.
        // Actually we need a schedule that trains on Wed.
        game.teams
            .iter_mut()
            .find(|t| t.id == "ai")
            .unwrap()
            .training_schedule = TrainingSchedule::Intense; // trains Mon-Sat

        apply_ai_training_policies(&mut game, 2); // Wed slot = index 2

        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(ai.training_focus, TrainingFocus::Attacking);
    }

    // -----------------------------------------------------------------------
    // V1 safety rule
    // -----------------------------------------------------------------------

    #[test]
    fn physical_focus_with_high_intensity_is_downgraded_to_medium() {
        // Condition 80 → High base intensity; slot 0 = Physical for any style
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 80);

        apply_ai_training_policies(&mut game, 0); // Mon slot 0 = Physical

        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        // Physical would normally get High, but safety rule caps it at Medium
        assert_eq!(ai.training_focus, TrainingFocus::Physical);
        assert_ne!(
            ai.training_intensity,
            TrainingIntensity::High,
            "Physical + High must be blocked by safety rule"
        );
        assert_eq!(ai.training_intensity, TrainingIntensity::Medium);
    }

    // -----------------------------------------------------------------------
    // Low intensity → Recovery focus
    // -----------------------------------------------------------------------

    #[test]
    fn low_intensity_band_forces_recovery_focus() {
        // Condition 25 → Low band (no fixture congestion)
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 25);
        apply_ai_training_policies(&mut game, 1); // Tue
        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(ai.training_focus, TrainingFocus::Recovery);
        assert_eq!(ai.training_intensity, TrainingIntensity::Low);
    }

    // -----------------------------------------------------------------------
    // What the controller steers on
    //
    // These two cases pull in opposite directions on purpose. A statistic that
    // gets one of them right by accident — the squad mean, the squad minimum, a
    // low percentile — gets the other one wrong.
    // -----------------------------------------------------------------------

    /// A club of 22: eleven better players and eleven reserves, each half given
    /// its own overall rating and condition.
    fn make_game_with_a_split_squad(
        starter_ovr: u8,
        starter_condition: u8,
        reserve_ovr: u8,
        reserve_condition: u8,
    ) -> Game {
        let date = Utc.with_ymd_and_hms(2026, 6, 15, 12, 0, 0).unwrap();
        let clock = GameClock::new(date);
        let manager = make_manager(Some("user"));

        let mut players: Vec<Player> = Vec::new();
        for i in 0..11 {
            let mut starter = make_player(&format!("first{}", i), "ai", starter_condition);
            starter.ovr = starter_ovr;
            players.push(starter);
            let mut reserve = make_player(&format!("reserve{}", i), "ai", reserve_condition);
            reserve.ovr = reserve_ovr;
            players.push(reserve);
        }

        Game::new(
            clock,
            manager,
            vec![
                make_team("user", PlayStyle::Balanced),
                make_team("ai", PlayStyle::Balanced),
            ],
            players,
            vec![],
            vec![],
        )
    }

    #[test]
    fn a_fresh_bench_does_not_authorise_working_a_tired_first_eleven() {
        // Eleven at 50 behind eleven at 100: the squad averages 75, which reads
        // as a squad with room to work. The eleven who play do not have it.
        let mut game = make_game_with_a_split_squad(75, 50, 50, 100);

        // Tuesday: a Balanced training day whose cycle slot is Technical, so the
        // Physical-and-High safety rule cannot stand in for the result.
        apply_ai_training_policies(&mut game, 1);

        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(
            ai.training_intensity,
            TrainingIntensity::Medium,
            "a first eleven at 50 must not be worked at High just because the \
             reserves are fresh"
        );
    }

    #[test]
    fn a_shattered_reserve_squad_does_not_wrap_the_first_eleven_in_cotton_wool() {
        // The mirror image: the eleven who play are fresh at 90 and the reserves
        // are wrecked at 20. The squad averages 55. Nothing about the players who
        // take the field says this club should be training lightly.
        let mut game = make_game_with_a_split_squad(75, 90, 50, 20);

        apply_ai_training_policies(&mut game, 1);

        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        assert_eq!(
            ai.training_intensity,
            TrainingIntensity::High,
            "the players who play are at 90; the squad mean is being dragged \
             down by people who are not going to be picked"
        );
    }

    #[test]
    fn the_reading_does_not_depend_on_the_order_players_happen_to_be_stored_in() {
        // Every player rated the same, so rating cannot separate them and the
        // tiebreak decides which eleven the signal reads. It must decide the
        // same way whichever end of the squad list it starts from.
        let mut game = make_game_with_a_split_squad(60, 100, 60, 20);
        let first = likely_starters_condition(&game, "ai");
        game.players.reverse();
        let second = likely_starters_condition(&game, "ai");

        assert!(
            (first - second).abs() < f64::EPSILON,
            "the same squad gave two different readings ({first} then {second}) \
             once its players were stored in a different order"
        );
    }

    // -----------------------------------------------------------------------
    // Fixture proximity is no longer this module's business
    //
    // The near-match / congestion taper moved to `training::is_tapering`, which
    // applies it to every club. What this planner writes is the club's standing
    // plan; the taper is applied on top of it, per day, without rewriting it.
    // -----------------------------------------------------------------------

    #[test]
    fn a_congested_fixture_list_does_not_change_the_standing_plan() {
        let mut game = make_game_with_two_teams("user", "ai", PlayStyle::Balanced, 80);

        // Add 2 fixtures in the next 7 days for the AI team
        // today is 2026-06-15 (Mon); fixtures on +3 and +5 days
        let fix1 = Fixture {
            id: "f1".to_string(),
            competition_id: "league1".to_string(),
            matchday: 1,
            date: "2026-06-18".to_string(),
            home_team_id: "ai".to_string(),
            away_team_id: "other".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        };
        let fix2 = Fixture {
            id: "f2".to_string(),
            competition_id: "league1".to_string(),
            matchday: 2,
            date: "2026-06-20".to_string(),
            home_team_id: "ai".to_string(),
            away_team_id: "other2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        };
        let team_ids = vec!["ai".to_string()];
        let mut league = League::new("league1".to_string(), "Test".to_string(), 1, &team_ids);
        league.fixtures = vec![fix1, fix2];
        game.league = Some(league);

        apply_ai_training_policies(&mut game, 0); // Mon, healthy squad, but congested

        let ai = game.teams.iter().find(|t| t.id == "ai").unwrap();
        // The fixture list no longer reaches into the standing plan: a condition-80
        // squad follows its style's Monday slot, which for Balanced is Physical, and
        // lands on Medium only because of the Physical-and-High safety rule.
        // `training_tests` covers what those fixtures actually do — taper the session.
        assert_eq!(ai.training_focus, TrainingFocus::Physical);
        assert_eq!(ai.training_intensity, TrainingIntensity::Medium);
        assert_eq!(
            style_weekly_cycle(&PlayStyle::Balanced)[0],
            TrainingFocus::Physical,
            "this test reads the Monday slot; if the cycle changes, so must it"
        );
    }
}
