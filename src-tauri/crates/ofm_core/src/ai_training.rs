//! AI-only daily training policy.
//!
//! Applies automated training focus and intensity decisions to all non-user teams
//! each non-match training day, BEFORE `training::process_training` runs.
//!
//! Algorithm:
//! 1. Skip the user-controlled team entirely.
//! 2. Skip if today is a rest day for that team's schedule.
//! 3. If avg available-player condition < 10 → Recovery focus + Low intensity (no cycle advance).
//! 4. Otherwise compute intensity from condition band (Low/Medium/High).
//! 5. Apply near-match / congestion downgrade where applicable.
//! 6. Pick focus from the style-biased 5-slot weekly cycle (indexed by weekday % 5).
//! 7. Force Recovery focus when final intensity is Low (fatigue band or downgraded).
//! 8. Force Tactical focus when congested but intensity is still Medium.
//! 9. V1 safety rule: Physical + High → downgrade intensity to Medium.

use crate::game::Game;
use chrono::NaiveDate;
use domain::league::FixtureStatus;
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
/// Fixture within this many days is considered "near match".
const NEAR_MATCH_DAYS: i64 = 2;
/// This many fixtures in the next 7 days counts as congested.
const CONGESTION_FIXTURE_THRESHOLD: usize = 2;

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

fn downgrade_intensity(intensity: TrainingIntensity) -> TrainingIntensity {
    match intensity {
        TrainingIntensity::High => TrainingIntensity::Medium,
        TrainingIntensity::Medium | TrainingIntensity::Low => TrainingIntensity::Low,
    }
}

// ---------------------------------------------------------------------------
// Per-team snapshot (immutable read, no borrows retained)
// ---------------------------------------------------------------------------

struct TeamSnapshot {
    avg_condition: f64,
    days_to_next_fixture: i64,
    fixtures_in_next_7: usize,
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

    let available_players: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.injury.is_none())
        .collect();

    let avg_condition = if available_players.is_empty() {
        100.0
    } else {
        available_players
            .iter()
            .map(|p| p.condition as f64)
            .sum::<f64>()
            / available_players.len() as f64
    };

    let today = game.clock.current_date.date_naive();
    let (days_to_next, fixtures_in_next_7) = match &game.league {
        None => (i64::MAX, 0),
        Some(league) => {
            let upcoming: Vec<i64> = league
                .fixtures
                .iter()
                .filter(|f| {
                    f.status == FixtureStatus::Scheduled
                        && (f.home_team_id == team_id || f.away_team_id == team_id)
                })
                .filter_map(|f| NaiveDate::parse_from_str(&f.date, "%Y-%m-%d").ok())
                .filter(|d| *d >= today)
                .map(|d| (d - today).num_days())
                .collect();

            let days_to_next = upcoming.iter().copied().min().unwrap_or(i64::MAX);
            let fixtures_in_next_7 = upcoming.iter().filter(|&&d| d <= 7).count();
            (days_to_next, fixtures_in_next_7)
        }
    };

    TeamSnapshot {
        avg_condition,
        days_to_next_fixture: days_to_next,
        fixtures_in_next_7,
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

    // Collect AI team IDs up front to avoid borrow conflicts.
    let team_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|t| Some(&t.id) != user_team_id.as_ref())
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
        if snap.avg_condition < RECOVERY_CRISIS_THRESHOLD {
            if let Some(team) = game.teams.iter_mut().find(|t| t.id == team_id) {
                team.training_focus = TrainingFocus::Recovery;
                team.training_intensity = TrainingIntensity::Low;
            }
            continue;
        }

        // Base intensity from condition band.
        let base_intensity = if snap.avg_condition < LOW_INTENSITY_MAX {
            TrainingIntensity::Low
        } else if snap.avg_condition <= HIGH_INTENSITY_MIN {
            TrainingIntensity::Medium
        } else {
            TrainingIntensity::High
        };

        let near_match = snap.days_to_next_fixture <= NEAR_MATCH_DAYS;
        let congested = snap.fixtures_in_next_7 >= CONGESTION_FIXTURE_THRESHOLD;
        let congestion_active = near_match || congested;

        let intensity = if congestion_active {
            downgrade_intensity(base_intensity)
        } else {
            base_intensity
        };

        // Style-biased weekly cycle; slot is based on weekday mod 5.
        let cycle = style_weekly_cycle(&snap.play_style);
        let slot = (weekday_num as usize) % 5;
        let rotation_focus = cycle[slot].clone();

        // Focus override based on effective intensity / congestion.
        let focus = match &intensity {
            // Low intensity (fatigue or congestion-downgraded) → recovery-first.
            TrainingIntensity::Low => TrainingFocus::Recovery,
            // Medium + congestion active → pre-match tactical work.
            TrainingIntensity::Medium if congestion_active => TrainingFocus::Tactical,
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
    // Congestion → Tactical focus + downgraded intensity
    // -----------------------------------------------------------------------

    #[test]
    fn congestion_downgrades_intensity_and_sets_tactical_focus() {
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
        // High base → downgraded to Medium due to congestion, Tactical focus
        assert_eq!(ai.training_focus, TrainingFocus::Tactical);
        assert_eq!(ai.training_intensity, TrainingIntensity::Medium);
    }
}
