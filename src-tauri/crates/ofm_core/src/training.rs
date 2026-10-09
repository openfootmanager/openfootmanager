mod fitness_warnings;
mod focus_attributes;
pub use fitness_warnings::check_squad_fitness_warnings;

use crate::game::Game;
use crate::player_rating::refresh_player_derived;
use domain::player::{Player, PlayerAttributes, Position};
use domain::staff::{CoachingSpecialization, StaffRole};
use domain::team::{TrainingFocus, TrainingIntensity, TrainingSchedule};
use rand::Rng;

use focus_attributes::focus_gains;
pub use focus_attributes::{TrainingFocusAttributes, training_focus_attributes};

/// Computed coaching quality for a team's staff.
pub struct TeamCoachingBonus {
    pub coaching_mult: f64, // Overall coaching quality multiplier (1.0 = no staff)
    pub specialization_mult: f64, // Extra bonus if a coach specializes in the current focus
    pub physio_mult: f64,   // Recovery bonus from physio staff
}

/// Compute coaching bonuses from a team's staff.
fn compute_coaching_bonus(game: &Game, team_id: &str, focus: &TrainingFocus) -> TeamCoachingBonus {
    let team_staff: Vec<_> = game
        .staff
        .iter()
        .filter(|s| s.team_id.as_deref() == Some(team_id))
        .collect();

    // Average coaching rating of coaches + assistant managers
    let coaching_staff: Vec<_> = team_staff
        .iter()
        .filter(|s| matches!(s.role, StaffRole::Coach | StaffRole::AssistantManager))
        .collect();

    let coaching_mult = if coaching_staff.is_empty() {
        0.8 // Penalty for having no coaching staff
    } else {
        let avg_coaching: f64 = coaching_staff
            .iter()
            .map(|s| s.attributes.coaching as f64)
            .sum::<f64>()
            / coaching_staff.len() as f64;
        // Range: 0.85 (coaching=0) to 1.35 (coaching=100)
        0.85 + (avg_coaching / 100.0) * 0.5
    };

    // Check if any coach specializes in the current training focus
    let focus_spec = match focus {
        TrainingFocus::Physical => Some(CoachingSpecialization::Fitness),
        TrainingFocus::Technical => Some(CoachingSpecialization::Technique),
        TrainingFocus::Tactical => Some(CoachingSpecialization::Tactics),
        TrainingFocus::Defending => Some(CoachingSpecialization::Defending),
        TrainingFocus::Attacking => Some(CoachingSpecialization::Attacking),
        TrainingFocus::Recovery => None,
    };

    let specialization_mult = if let Some(target_spec) = focus_spec {
        let has_specialist = coaching_staff
            .iter()
            .any(|s| s.specialization.as_ref() == Some(&target_spec));
        if has_specialist { 1.25 } else { 1.0 }
    } else {
        1.0
    };

    // Physio bonus for recovery
    let physio_staff: Vec<_> = team_staff
        .iter()
        .filter(|s| matches!(s.role, StaffRole::Physio))
        .collect();

    let physio_mult = if physio_staff.is_empty() {
        1.0
    } else {
        let avg_physio: f64 = physio_staff
            .iter()
            .map(|s| s.attributes.physiotherapy as f64)
            .sum::<f64>()
            / physio_staff.len() as f64;
        // Range: 1.0 (physio=0) to 1.4 (physio=100)
        1.0 + (avg_physio / 100.0) * 0.4
    };

    TeamCoachingBonus {
        coaching_mult,
        specialization_mult,
        physio_mult,
    }
}

struct TrainingDay {
    weekday_num: u32,
    year: u32,
}

/// Below this individual condition a player is automatically rested in training
/// (treated as Recovery focus) regardless of the team's plan. Team intensity is
/// one setting for the whole squad, but the per-player condition cost is flat —
/// so a player who is individually exhausted in an otherwise-okay squad keeps
/// net-losing condition (cost > their diminished recovery) and never climbs out.
/// This guard breaks that fatigue spiral so a rested bench can recover and the
/// condition-aware lineup picker has someone to rotate to.
///
/// It applies to every club. It used to exempt the user's, on the grounds that a
/// human manager has manual control — but the exemption was the difference
/// between a squad that stabilises and one that reaches condition 2 by week five
/// and stays there, which is not a choice anyone was making knowingly.
const FATIGUE_GUARD_CONDITION: u8 = 40;

/// A club within this many days of its next fixture trains at reduced load.
///
/// No squad does a full session two days before a game; the days before a match
/// are a taper, not a decision the manager re-litigates each week. Keeping this
/// as a property of training rather than of the AI's planner is what stops the
/// two sides of the game running on different physics.
const MATCH_TAPER_DAYS: i64 = 2;

/// This many fixtures inside a week is a congested run.
const CONGESTION_FIXTURE_THRESHOLD: usize = 2;

/// Why a club's sessions are lighter today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Taper {
    /// A fixture within `MATCH_TAPER_DAYS`: today's session runs one step
    /// lighter than the club's standing intensity.
    NearMatch,
    /// Two or more fixtures inside the coming week: every session is recovery
    /// work. A match costs a starter about 28 condition and a recovery day gives
    /// back about 10; two matches in a week leave no room for training load at
    /// all, and one step down from High is still Medium, which costs more than
    /// it restores. Clubs that trained through a two-match week reached its
    /// second match with squads averaging below 80.
    Congested,
}

/// One step down the intensity ladder.
fn downgrade_intensity(intensity: &TrainingIntensity) -> TrainingIntensity {
    match intensity {
        TrainingIntensity::High => TrainingIntensity::Medium,
        TrainingIntensity::Medium | TrainingIntensity::Low => TrainingIntensity::Low,
    }
}

/// Every club with a fixture today, in any competition it plays in.
///
/// Deliberately matched on date alone, with no filter on `FixtureStatus`. This is
/// asked *after* the day's matches have been simulated, by which point those
/// fixtures read `Completed`; a status filter here would report the clubs that
/// just played ninety minutes as free to train, and they would recover on the
/// very day they were emptied.
///
/// Only competitions in the active scope count. A dormant competition is
/// resolved by a scoreline-only model that charges nobody any condition, so its
/// clubs have not had a match in any physical sense — closing the training
/// ground on them would cost them the day's recovery for nothing.
pub(crate) fn teams_playing_on(game: &Game, date: &str) -> std::collections::HashSet<String> {
    game.competitions_in_play()
        .iter()
        .filter(|competition| game.competition_in_active_scope(competition))
        .flat_map(|competition| competition.fixtures.iter())
        .filter(|fixture| fixture.date == date)
        .flat_map(|fixture| [fixture.home_team_id.clone(), fixture.away_team_id.clone()])
        .collect()
}

/// Every club whose fixture list puts it inside a taper today.
///
/// Reads every competition a club plays in, not just `game.league`: a cup tie
/// tires a squad exactly as much as a league game. Built as one pass over the
/// fixture list rather than a per-club scan — this runs for every club, every
/// day, and a populated world holds tens of thousands of fixtures.
///
/// Only competitions in the active scope count, as in [`teams_playing_on`]: a
/// dormant match charges nobody any condition, so it is nothing to taper for.
fn tapering_teams(game: &Game) -> std::collections::HashMap<String, Taper> {
    use chrono::NaiveDate;
    use domain::league::FixtureStatus;

    let today = game.clock.current_date.date_naive();

    // team id → how many of its fixtures fall inside the next week.
    let mut fixtures_this_week: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    let mut tapering: std::collections::HashMap<String, Taper> = std::collections::HashMap::new();

    for fixture in game
        .competitions_in_play()
        .iter()
        .filter(|competition| game.competition_in_active_scope(competition))
        .flat_map(|competition| competition.fixtures.iter())
        .filter(|fixture| fixture.status == FixtureStatus::Scheduled)
    {
        let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") else {
            continue;
        };
        let days = (date - today).num_days();
        if !(0..=7).contains(&days) {
            continue;
        }
        for team_id in [&fixture.home_team_id, &fixture.away_team_id] {
            if days <= MATCH_TAPER_DAYS {
                tapering.entry(team_id.clone()).or_insert(Taper::NearMatch);
            }
            let seen = fixtures_this_week.entry(team_id.as_str()).or_default();
            *seen += 1;
            if *seen >= CONGESTION_FIXTURE_THRESHOLD {
                tapering.insert(team_id.clone(), Taper::Congested);
            }
        }
    }

    tapering
}

/// Per-team data collected before mutating players.
struct TeamTrainingPlan {
    default_focus: TrainingFocus,
    intensity: TrainingIntensity,
    schedule: TrainingSchedule,
    bonus: TeamCoachingBonus,
    medical_facility_mult: f64,
    /// player_id → group focus override (players not in any group use default_focus)
    group_overrides: std::collections::HashMap<String, TrainingFocus>,
    /// Why today's session runs at reduced load, if it does.
    taper: Option<Taper>,
}

/// Process daily training for every club that is not playing today.
///
/// A club's players train according to its current focus, intensity and
/// schedule. Rest days (determined by the weekly schedule) give full condition
/// recovery with no training cost. Players assigned to a training group use that
/// group's focus instead of the team default.
///
/// Clubs with a fixture today are skipped, and only those clubs: whether the
/// training ground opens is a question about *this* club's calendar, not the
/// world's. It used to be asked globally, so one cup tie anywhere shut every
/// training ground in the game and nobody recovered.
///
/// `weekday_num` is 0=Mon .. 6=Sun (chrono Weekday::num_days_from_monday()).
pub fn process_training(game: &mut Game, weekday_num: u32) {
    // Derive the current year from the game clock for accurate age calculations.
    let current_year = game
        .clock
        .current_date
        .format("%Y")
        .to_string()
        .parse::<u32>()
        .unwrap_or(2026);

    // Index each team's plan by id so players are visited once (O(teams + players))
    // instead of rescanning every player for every team (O(teams * players)).
    let tapering = tapering_teams(game);
    let playing_today = teams_playing_on(
        game,
        &game.clock.current_date.format("%Y-%m-%d").to_string(),
    );
    let plans: std::collections::HashMap<String, TeamTrainingPlan> = game
        .teams
        .iter()
        .filter(|t| !playing_today.contains(&t.id))
        .map(|t| {
            let bonus = compute_coaching_bonus(game, &t.id, &t.training_focus);
            let medical_facility_mult =
                1.0 + f64::from(t.facilities.medical.saturating_sub(1)) * 0.1;
            let mut group_overrides = std::collections::HashMap::new();
            for group in &t.training_groups {
                for pid in &group.player_ids {
                    group_overrides.insert(pid.clone(), group.focus.clone());
                }
            }
            (
                t.id.clone(),
                TeamTrainingPlan {
                    default_focus: t.training_focus.clone(),
                    intensity: t.training_intensity.clone(),
                    schedule: t.training_schedule.clone(),
                    bonus,
                    medical_facility_mult,
                    group_overrides,
                    taper: tapering.get(&t.id).copied(),
                },
            )
        })
        .collect();

    let day = TrainingDay {
        weekday_num,
        year: current_year,
    };
    // One stream for the day's session: players are visited in the order of `game.players`,
    // which is stable, and `process_training` runs once a day.
    let mut rng = game.rng_today("training");
    for player in game.players.iter_mut() {
        let Some(plan) = player.team_id.as_deref().and_then(|id| plans.get(id)) else {
            continue;
        };
        train_player(player, plan, &day, &mut rng);
    }
}

struct PlayerTrainingSession {
    is_training_day: bool,
    focus: TrainingFocus,
    intensity_mult: f64,
    condition_cost: u8,
    recovery_base: f64,
}

fn player_training_session(
    player: &Player,
    plan: &TeamTrainingPlan,
    weekday_num: u32,
) -> PlayerTrainingSession {
    let is_training_day = plan.schedule.is_training_day(weekday_num);

    // The taper: with a fixture close, today's session runs one step lighter than
    // the manager's standing setting. The setting itself is untouched — the club's
    // stored plan is the manager's, and a taper is not a change of plan.
    let intensity = match plan.taper {
        Some(Taper::Congested) if is_training_day => TrainingIntensity::Low,
        Some(Taper::NearMatch) if is_training_day => downgrade_intensity(&plan.intensity),
        _ => plan.intensity.clone(),
    };
    let intensity_mult = match &intensity {
        TrainingIntensity::Low => 0.5,
        TrainingIntensity::Medium => 1.0,
        TrainingIntensity::High => 1.5,
    };

    // Two ways a session becomes recovery work rather than a load:
    //
    // 1. The fatigue guard — a player this tired physically cannot take a hard
    //    session. See `FATIGUE_GUARD_CONDITION`.
    // 2. A tapered session that has already come down to Low. At that point the
    //    squad is ticking over before a game, which is what Recovery models.
    //
    // Injured players are exempt from both: they don't train regardless, and
    // routing them through Recovery focus here would inflate the injured-recovery
    // base below (9.0 instead of 3.0), giving them ~3x recovery.
    let recovery_focus = TrainingFocus::Recovery;
    let is_resting = is_training_day
        && player.injury.is_none()
        && (player.condition < FATIGUE_GUARD_CONDITION
            || (plan.taper.is_some() && intensity == TrainingIntensity::Low));
    let player_focus = if is_resting {
        &recovery_focus
    } else {
        // Determine this player's effective focus:
        // player override > group override > team default
        player
            .training_focus
            .as_ref()
            .or_else(|| plan.group_overrides.get(&player.id))
            .unwrap_or(&plan.default_focus)
    };

    // On rest days or Recovery focus: no training cost
    let condition_cost: u8 = if !is_training_day {
        0
    } else {
        match (player_focus, &intensity) {
            (TrainingFocus::Recovery, _) => 0,
            (_, TrainingIntensity::Low) => 3,
            (_, TrainingIntensity::Medium) => 6,
            (_, TrainingIntensity::High) => 10,
        }
    };

    // A scheduled day off restores more than any session, including a Recovery
    // one. It has to: restoring condition is the *only* thing it does, while a
    // Recovery session does that and nudges match fitness besides. At the old
    // 7.0 against a session's 9.0 a day off was strictly dominated, so the
    // schedule with the fewest of them was simply the best schedule — a club
    // training six days a week finished it fresher than one resting five.
    // The trade is now honest: days off buy condition, sessions buy sharpness
    // and development.
    let recovery_base: f64 = if !is_training_day {
        10.0 * plan.bonus.physio_mult * plan.medical_facility_mult
    } else {
        match player_focus {
            TrainingFocus::Recovery => 9.0 * plan.bonus.physio_mult * plan.medical_facility_mult,
            _ => 3.0 * plan.bonus.physio_mult * plan.medical_facility_mult,
        }
    };

    PlayerTrainingSession {
        is_training_day,
        focus: player_focus.clone(),
        intensity_mult,
        condition_cost,
        recovery_base,
    }
}

fn train_player(
    player: &mut Player,
    plan: &TeamTrainingPlan,
    day: &TrainingDay,
    rng: &mut impl Rng,
) {
    let session = player_training_session(player, plan, day.weekday_num);

    // Age, morale, and current condition all affect recovery rate.
    // Older players recover more slowly; high morale aids recovery;
    // severely fatigued players have a harder time bouncing back.
    let age = estimate_age(&player.date_of_birth, day.year);
    let age_rec = recovery_factor_from_age(age);
    let morale_rec = recovery_factor_from_morale(player.morale);
    let condition_rec = recovery_factor_from_condition(player.condition);
    let fitness_rec = recovery_factor_from_fitness(player.fitness);

    // Injured players: half base recovery, scaled by age and morale.
    // Fitness decays slowly during injury (inactive = losing sharpness).
    if player.injury.is_some() {
        let recovery = (session.recovery_base * 0.5 * age_rec * morale_rec * fitness_rec) as u8;
        player.condition = (player.condition + recovery).min(100);
        player.fitness = clamp_fitness(player.fitness as i16 - 1);
        return;
    }

    // On rest days: only recovery, no attribute gains
    if !session.is_training_day {
        let stamina_factor = player.attributes.stamina as f64 / 100.0;
        let recovery = (session.recovery_base
            * (0.5 + stamina_factor * 0.5)
            * age_rec
            * morale_rec
            * condition_rec
            * fitness_rec) as u8;
        player.condition = (player.condition + recovery).min(100);
        return;
    }

    // Age factor for attribute gains: younger players grow faster, older players slower
    let age_factor = if age <= 21 {
        1.5
    } else if age <= 25 {
        1.2
    } else if age <= 29 {
        1.0
    } else if age <= 33 {
        0.6
    } else {
        0.3
    };

    // Base gain per attribute per session, boosted by coaching staff
    let gain = 0.15
        * session.intensity_mult
        * age_factor
        * plan.bonus.coaching_mult
        * plan.bonus.specialization_mult;

    // Peaked players (ovr == potential) get no attribute gains. Without this
    // gate, attribute drift lifts ovr, and `refresh_player_derived`'s
    // `potential = max(potential, ovr)` invariant silently raises the career
    // ceiling in lockstep — the ceiling stops being a ceiling.
    if player.potential > player.ovr {
        let is_goalkeeper = player.natural_position.to_group_position() == Position::Goalkeeper;
        apply_focus_gains(
            &mut player.attributes,
            &session.focus,
            is_goalkeeper,
            gain,
            rng,
        );
    }
    apply_fitness_change(
        &mut player.fitness,
        &session.focus,
        session.intensity_mult,
        rng,
    );

    // Refresh position-weighted OVR and traits after attribute gains.
    refresh_player_derived(player, day.year);

    // Apply condition: deplete from training, then recover
    player.condition = player.condition.saturating_sub(session.condition_cost);
    let stamina_factor = player.attributes.stamina as f64 / 100.0;
    let recovery = (session.recovery_base
        * (0.5 + stamina_factor * 0.5)
        * age_rec
        * morale_rec
        * condition_rec
        * fitness_rec) as u8;
    player.condition = (player.condition + recovery).min(100);
}

/// Apply fitness changes based on training focus.
/// Physical training builds fitness (probabilistic small gains).
/// Recovery focus gives a tiny boost. Non-physical training slowly decays high fitness.
fn apply_fitness_change(
    fitness: &mut u8,
    focus: &TrainingFocus,
    intensity_mult: f64,
    rng: &mut impl Rng,
) {
    use rand::RngExt;
    match focus {
        TrainingFocus::Physical => {
            // Physical training is the primary way to build fitness.
            // Higher intensity → higher gain probability.
            let gain_prob = 0.015 * intensity_mult; // 0.0075–0.0225 per session
            let roll: f64 = rng.random_range(0.0..1.0);
            if roll < gain_prob && *fitness < 100 {
                *fitness = fitness.saturating_add(1);
            }
        }
        TrainingFocus::Recovery => {
            // Recovery days give a tiny fitness nudge.
            let roll: f64 = rng.random_range(0.0..1.0);
            if roll < 0.05 && *fitness < 100 {
                *fitness = fitness.saturating_add(1);
            }
        }
        _ => {
            // Non-physical training: very slight decay if player is already very fit
            // (fitness above 85 needs active maintenance).
            if *fitness > 85 {
                let roll: f64 = rng.random_range(0.0..1.0);
                if roll < 0.05 {
                    *fitness = fitness.saturating_sub(1);
                }
            }
        }
    }
}

fn try_gain(current: &mut u8, gain: f64, rng: &mut impl Rng) {
    use rand::RngExt;
    if *current >= 99 {
        return;
    }
    let roll: f64 = rng.random_range(0.0..1.0);
    if roll < gain {
        *current = (*current + 1).min(99);
    }
}

fn apply_focus_gains(
    attrs: &mut PlayerAttributes,
    focus: &TrainingFocus,
    is_goalkeeper: bool,
    gain: f64,
    rng: &mut impl Rng,
) {
    for focus_gain in focus_gains(focus, is_goalkeeper) {
        try_gain(
            focus_gain.attribute.of_mut(attrs),
            gain * focus_gain.rate,
            rng,
        );
    }
}

/// Estimate player age from date_of_birth string ("YYYY-MM-DD").
fn estimate_age(dob: &str, as_of_year: u32) -> u32 {
    let parts: Vec<&str> = dob.split('-').collect();
    if parts.is_empty() {
        return 25; // fallback
    }
    let birth_year: u32 = parts[0].parse().unwrap_or(2000);
    as_of_year.saturating_sub(birth_year)
}

/// Recovery multiplier from age: younger players bounce back faster.
fn recovery_factor_from_age(age: u32) -> f64 {
    if age <= 21 {
        1.10
    } else if age <= 25 {
        1.05
    } else if age <= 29 {
        1.00
    } else if age <= 33 {
        0.85
    } else {
        0.70
    }
}

/// Recovery multiplier from morale: players in good spirits recover better.
fn recovery_factor_from_morale(morale: u8) -> f64 {
    if morale >= 70 {
        1.10
    } else if morale >= 40 {
        1.00
    } else {
        0.90
    }
}

/// Recovery multiplier from current condition: severely fatigued players recover more slowly.
fn recovery_factor_from_condition(condition: u8) -> f64 {
    if condition < 30 {
        0.80
    } else if condition < 50 {
        0.90
    } else {
        1.00
    }
}

/// Recovery multiplier from fitness: fitter players recover condition faster.
fn recovery_factor_from_fitness(fitness: u8) -> f64 {
    if fitness < 30 {
        0.75
    } else if fitness < 50 {
        0.88
    } else if fitness < 70 {
        1.00
    } else if fitness < 90 {
        1.12
    } else {
        1.20
    }
}

/// Clamp a fitness value to 0–100.
fn clamp_fitness(val: i16) -> u8 {
    val.clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::test_fixtures::{manager_for, nation_team, player_at};
    use chrono::{Datelike, TimeZone, Utc};
    use domain::player::{Injury, PlayerTrait, Position};

    /// Zero lies below every positive gain probability, so these scenarios have no lucky-roll dependency.
    struct LowestRoll;

    impl rand::TryRng for LowestRoll {
        type Error = std::convert::Infallible;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok(0)
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok(0)
        }
        fn try_fill_bytes(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error> {
            bytes.fill(0);
            Ok(())
        }
    }

    /// One sixth succeeds at the primary 0.225 rate and fails at the secondary 0.1125 rate.
    struct BetweenGainRates;

    impl rand::TryRng for BetweenGainRates {
        type Error = std::convert::Infallible;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok(u32::MAX / 6)
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok(u64::MAX / 6)
        }
        fn try_fill_bytes(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error> {
            for chunk in bytes.chunks_mut(8) {
                chunk.copy_from_slice(&(u64::MAX / 6).to_le_bytes()[..chunk.len()]);
            }
            Ok(())
        }
    }

    fn keeper(team_id: &str) -> Player {
        let mut player = player_at("keeper", team_id, "2029-06-30");
        player.date_of_birth = "2006-03-15".to_string();
        player.natural_position = Position::Goalkeeper;
        player.position = Position::Goalkeeper;
        player.attributes.handling = 49;
        player.attributes.reflexes = 53;
        player.potential = 99;
        player.condition = 100;
        player.fitness = 80;
        player.morale = 80;
        refresh_player_derived(&mut player, 2026);
        player
    }

    fn plan(focus: TrainingFocus) -> TeamTrainingPlan {
        TeamTrainingPlan {
            default_focus: focus,
            intensity: TrainingIntensity::Medium,
            schedule: TrainingSchedule::Balanced,
            bonus: TeamCoachingBonus {
                coaching_mult: 1.0,
                specialization_mult: 1.0,
                physio_mult: 1.0,
            },
            medical_facility_mult: 1.0,
            group_overrides: std::collections::HashMap::new(),
            taper: None,
        }
    }

    fn session(player: &mut Player, plan: &TeamTrainingPlan, weekday_num: u32) {
        train_player(
            player,
            plan,
            &TrainingDay {
                weekday_num,
                year: 2026,
            },
            &mut LowestRoll,
        );
    }

    fn attributes(player: &Player) -> serde_json::Value {
        serde_json::to_value(&player.attributes).unwrap()
    }

    fn training_game(team_id: &str) -> Game {
        let mut game = Game::new(
            crate::clock::GameClock::new(Utc.with_ymd_and_hms(2026, 1, 12, 12, 0, 0).unwrap()),
            manager_for("team1"),
            vec![
                nation_team("team1", "England", 500),
                nation_team("team2", "England", 400),
            ],
            vec![keeper(team_id)],
            vec![],
            vec![],
        );
        game.seed = 20261006;
        for team in &mut game.teams {
            team.training_focus = TrainingFocus::Technical;
            team.training_intensity = TrainingIntensity::Medium;
            team.training_schedule = TrainingSchedule::Balanced;
        }
        game
    }

    fn training_days(game: &mut Game) {
        for _ in 0..120 {
            let weekday_num = game.clock.current_date.weekday().num_days_from_monday();
            process_training(game, weekday_num);
            game.clock.current_date += chrono::Duration::days(1);
        }
    }

    /// Given a goalkeeper with growth room, when technical training gains succeed, then handling and reflexes improve with passing instead of shooting and dribbling.
    #[test]
    fn technical_training_develops_the_goalkeepers_specialist_skills() {
        let mut player = keeper("team1");
        let mut expected = player.attributes.clone();
        expected.passing += 1;
        expected.handling += 1;
        expected.reflexes += 1;
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert_eq!(attributes(&player), serde_json::to_value(expected).unwrap());
    }

    /// Given a goalkeeper with growth room, when defending gains succeed, then keeper skills replace outfield defending while strength and positioning still improve.
    #[test]
    fn defending_training_develops_the_goalkeepers_specialist_skills() {
        let mut player = keeper("team1");
        let mut expected = player.attributes.clone();
        expected.handling += 1;
        expected.reflexes += 1;
        expected.strength += 1;
        expected.positioning += 1;
        session(&mut player, &plan(TrainingFocus::Defending), 0);
        assert_eq!(attributes(&player), serde_json::to_value(expected).unwrap());
    }

    /// Given a keeper and a roll between primary and secondary defending gain rates, when training runs, then keeper skills grow while half-strength secondary attributes do not.
    #[test]
    fn goalkeeper_defending_preserves_the_secondary_gain_rate() {
        let mut player = keeper("team1");
        let mut expected = player.attributes.clone();
        expected.handling += 1;
        expected.reflexes += 1;
        train_player(
            &mut player,
            &plan(TrainingFocus::Defending),
            &TrainingDay {
                weekday_num: 0,
                year: 2026,
            },
            &mut BetweenGainRates,
        );
        assert_eq!(attributes(&player), serde_json::to_value(expected).unwrap());
    }

    /// Given an outfield player, when technical gains succeed, then its existing passing, shooting and dribbling mapping remains unchanged.
    #[test]
    fn technical_training_preserves_the_outfield_attribute_mapping() {
        let mut player = keeper("team1");
        player.natural_position = Position::Striker;
        player.position = Position::Striker;
        let mut expected = player.attributes.clone();
        expected.passing += 1;
        expected.shooting += 1;
        expected.dribbling += 1;
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert_eq!(attributes(&player), serde_json::to_value(expected).unwrap());
    }

    /// Given an outfield defender, when defending gains succeed, then its existing tackling, defending and secondary gains remain unchanged.
    #[test]
    fn defending_training_preserves_the_outfield_attribute_mapping() {
        let mut player = keeper("team1");
        player.natural_position = Position::CenterBack;
        player.position = Position::CenterBack;
        let mut expected = player.attributes.clone();
        expected.tackling += 1;
        expected.defending += 1;
        expected.strength += 1;
        expected.positioning += 1;
        session(&mut player, &plan(TrainingFocus::Defending), 0);
        assert_eq!(attributes(&player), serde_json::to_value(expected).unwrap());
    }

    /// Given a natural goalkeeper whose legacy primary position is stale, when technical training runs, then its natural position selects goalkeeper gains.
    #[test]
    fn a_goalkeepers_natural_position_selects_its_training_attributes() {
        let mut player = keeper("team1");
        player.position = Position::Forward;
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert_eq!(player.attributes.handling, 50);
        assert_eq!(player.attributes.reflexes, 54);
        assert_eq!(player.attributes.shooting, 60);
    }

    /// Given a natural outfield player whose legacy primary position says goalkeeper, when technical training runs, then goalkeeper attributes remain untouched.
    #[test]
    fn an_outfield_players_natural_position_keeps_its_training_mapping() {
        let mut player = keeper("team1");
        player.natural_position = Position::Striker;
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert_eq!(player.attributes.handling, 49);
        assert_eq!(player.attributes.reflexes, 53);
        assert_eq!(player.attributes.shooting, 61);
    }

    /// Given an individual technical focus overriding defending group and physical team work, when the keeper trains, then the individual focus controls gains.
    #[test]
    fn an_individual_focus_takes_precedence_for_a_goalkeeper() {
        let mut player = keeper("team1");
        player.training_focus = Some(TrainingFocus::Technical);
        let mut plan = plan(TrainingFocus::Physical);
        plan.group_overrides
            .insert(player.id.clone(), TrainingFocus::Defending);
        session(&mut player, &plan, 0);
        assert_eq!(player.attributes.handling, 50);
        assert_eq!(player.attributes.reflexes, 54);
        assert_eq!(player.attributes.passing, 61);
        assert_eq!(player.attributes.strength, 60);
    }

    /// Given a defending group overriding physical team work, when the keeper trains without an individual override, then group focus controls keeper gains.
    #[test]
    fn a_group_focus_takes_precedence_for_a_goalkeeper() {
        let mut player = keeper("team1");
        let mut plan = plan(TrainingFocus::Physical);
        plan.group_overrides
            .insert(player.id.clone(), TrainingFocus::Defending);
        session(&mut player, &plan, 0);
        assert_eq!(player.attributes.handling, 50);
        assert_eq!(player.attributes.reflexes, 54);
        assert_eq!(player.attributes.positioning, 61);
        assert_eq!(player.attributes.passing, 60);
    }

    /// Given a keeper already at its potential, when a technical session has successful rolls, then attributes and its career ceiling cannot drift upward.
    #[test]
    fn a_peaked_goalkeeper_cannot_gain_attributes() {
        let mut player = keeper("team1");
        player.potential = player.ovr;
        let before = attributes(&player);
        let potential = player.potential;
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert_eq!(attributes(&player), before);
        assert_eq!(player.potential, potential);
    }

    /// Given keeper skills one below the cap and growth room, when two successful sessions run, then both stop at 99 without overflow.
    #[test]
    fn goalkeeper_skill_gains_stop_at_the_existing_attribute_cap() {
        let mut player = keeper("team1");
        player.attributes.handling = 98;
        player.attributes.reflexes = 98;
        refresh_player_derived(&mut player, 2026);
        let plan = plan(TrainingFocus::Technical);
        session(&mut player, &plan, 0);
        session(&mut player, &plan, 0);
        assert_eq!(player.attributes.handling, 99);
        assert_eq!(player.attributes.reflexes, 99);
    }

    /// Given an injured keeper, when a scheduled technical day runs, then rehabilitation does not grant attribute gains.
    #[test]
    fn an_injured_goalkeeper_cannot_gain_training_attributes() {
        let mut player = keeper("team1");
        player.injury = Some(Injury {
            name: "test injury".to_string(),
            days_remaining: 7,
        });
        let before = attributes(&player);
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert_eq!(attributes(&player), before);
    }

    /// Given a keeper on a scheduled rest day, when training is processed, then rest restores condition without improving keeper skills.
    #[test]
    fn a_goalkeeper_rest_day_cannot_grant_attribute_gains() {
        let mut player = keeper("team1");
        player.condition = 70;
        let before = attributes(&player);
        session(&mut player, &plan(TrainingFocus::Technical), 2);
        assert_eq!(attributes(&player), before);
        assert!(player.condition > 70);
    }

    /// Given a keeper assigned Recovery, when a training day runs, then goalkeeper gains remain disabled.
    #[test]
    fn recovery_focus_cannot_grant_goalkeeper_attribute_gains() {
        let mut player = keeper("team1");
        let before = attributes(&player);
        session(&mut player, &plan(TrainingFocus::Recovery), 0);
        assert_eq!(attributes(&player), before);
    }

    /// Given a fatigued keeper with technical focus, when its training day runs, then the shared fatigue guard rests it without keeper gains.
    #[test]
    fn the_fatigue_guard_restores_a_goalkeeper_without_attribute_gains() {
        let mut player = keeper("team1");
        player.condition = 39;
        let before = attributes(&player);
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert_eq!(attributes(&player), before);
        assert!(player.condition > 39);
    }

    /// Given fixture congestion, when a technical keeper session runs, then its recovery taper still prevents gains.
    #[test]
    fn a_congested_goalkeeper_session_remains_recovery_work() {
        let mut player = keeper("team1");
        let mut plan = plan(TrainingFocus::Technical);
        plan.taper = Some(Taper::Congested);
        let before = attributes(&player);
        session(&mut player, &plan, 0);
        assert_eq!(attributes(&player), before);
    }

    /// Given a medium session immediately before a fixture, when its load drops to Low, then the keeper still receives recovery work rather than gains.
    #[test]
    fn a_goalkeeper_near_match_taper_preserves_recovery_work() {
        let mut player = keeper("team1");
        let mut plan = plan(TrainingFocus::Technical);
        plan.taper = Some(Taper::NearMatch);
        let before = attributes(&player);
        session(&mut player, &plan, 0);
        assert_eq!(attributes(&player), before);
    }

    /// Given keeper skills below the specialist trait thresholds, when training raises both through them, then position-weighted OVR and traits refresh from the improved attributes.
    #[test]
    fn goalkeeper_gains_refresh_its_rating_and_specialist_traits() {
        let mut player = keeper("team1");
        player.attributes.handling = 84;
        player.attributes.reflexes = 84;
        refresh_player_derived(&mut player, 2026);
        let before_ovr = player.ovr;
        assert!(!player.traits.contains(&PlayerTrait::SafeHands));
        assert!(!player.traits.contains(&PlayerTrait::CatReflexes));
        session(&mut player, &plan(TrainingFocus::Technical), 0);
        assert!(player.ovr > before_ovr);
        assert!(player.traits.contains(&PlayerTrait::SafeHands));
        assert!(player.traits.contains(&PlayerTrait::CatReflexes));
    }

    /// Given a user keeper with growth room, when the actual daily training route advances a seeded run, then both keeper attributes grow.
    #[test]
    fn the_daily_training_route_develops_a_user_goalkeeper() {
        let mut game = training_game("team1");
        training_days(&mut game);
        assert!(game.players[0].attributes.handling > 49);
        assert!(game.players[0].attributes.reflexes > 53);
    }

    /// Given an AI keeper under the same training rules, when the actual daily route advances a seeded run, then both keeper attributes grow too.
    #[test]
    fn the_daily_training_route_develops_an_ai_goalkeeper() {
        let mut game = training_game("team2");
        training_days(&mut game);
        assert!(game.players[0].attributes.handling > 49);
        assert!(game.players[0].attributes.reflexes > 53);
    }

    /// Given a career with improved keeper skills and a nondefault seed, when a fresh reader loads it and continues daily training, then the saved gains survive and future growth matches the original run.
    #[test]
    fn goalkeeper_training_replays_from_the_saved_seed_and_attributes() {
        let mut game = training_game("team1");
        training_days(&mut game);
        assert!(game.players[0].attributes.handling > 49);
        assert!(game.players[0].attributes.reflexes > 53);
        let checkpoint = attributes(&game.players[0]);
        let mut restored: Game =
            serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        assert_eq!(restored.seed, 20261006);
        assert_eq!(attributes(&restored.players[0]), checkpoint);
        assert_eq!(restored.clock.current_date, game.clock.current_date);
        training_days(&mut game);
        training_days(&mut restored);
        assert!(restored.players[0].attributes.handling > 49);
        assert!(restored.players[0].attributes.reflexes > 53);
        assert_eq!(
            serde_json::to_value(&restored.players).unwrap(),
            serde_json::to_value(&game.players).unwrap()
        );
    }
}
