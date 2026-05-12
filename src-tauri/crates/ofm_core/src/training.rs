mod fitness_warnings;
pub use fitness_warnings::check_squad_fitness_warnings;

use crate::game::Game;
use crate::player_rating::refresh_player_derived;
use domain::staff::{CoachingSpecialization, StaffRole};
use domain::team::{TrainingFocus, TrainingIntensity, TrainingSchedule};

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

/// Per-team data collected before mutating players.
struct TeamTrainingPlan {
    team_id: String,
    default_focus: TrainingFocus,
    intensity: TrainingIntensity,
    schedule: TrainingSchedule,
    bonus: TeamCoachingBonus,
    medical_facility_mult: f64,
    /// player_id → group focus override (players not in any group use default_focus)
    group_overrides: std::collections::HashMap<String, TrainingFocus>,
}

/// Process daily training for all teams.
/// On non-match days each team's players train according to the team's
/// current focus, intensity, and schedule. Rest days (determined by the
/// weekly schedule) give full condition recovery with no training cost.
/// Players assigned to a training group use that group's focus instead of
/// the team default.
/// `weekday_num` is 0=Mon .. 6=Sun (chrono Weekday::num_days_from_monday()).
pub fn process_training(game: &mut Game, weekday_num: u32) {
    // Derive the current year from the game clock for accurate age calculations.
    let current_year = game.clock.current_date.format("%Y").to_string()
        .parse::<u32>()
        .unwrap_or(2026);

    // Collect plans for all teams (immutable borrow)
    let team_plans: Vec<TeamTrainingPlan> = game
        .teams
        .iter()
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
            TeamTrainingPlan {
                team_id: t.id.clone(),
                default_focus: t.training_focus.clone(),
                intensity: t.training_intensity.clone(),
                schedule: t.training_schedule.clone(),
                bonus,
                medical_facility_mult,
                group_overrides,
            }
        })
        .collect();

    for plan in &team_plans {
        let is_training_day = plan.schedule.is_training_day(weekday_num);

        let intensity_mult = match &plan.intensity {
            TrainingIntensity::Low => 0.5,
            TrainingIntensity::Medium => 1.0,
            TrainingIntensity::High => 1.5,
        };

        for player in game.players.iter_mut() {
            if player.team_id.as_deref() != Some(&plan.team_id) {
                continue;
            }

            // Determine this player's effective focus:
            // player override > group override > team default
            let player_focus = player
                .training_focus
                .as_ref()
                .or_else(|| plan.group_overrides.get(&player.id))
                .unwrap_or(&plan.default_focus);

            // On rest days or Recovery focus: no training cost
            let condition_cost: u8 = if !is_training_day {
                0
            } else {
                match (player_focus, &plan.intensity) {
                    (TrainingFocus::Recovery, _) => 0,
                    (_, TrainingIntensity::Low) => 3,
                    (_, TrainingIntensity::Medium) => 6,
                    (_, TrainingIntensity::High) => 10,
                }
            };

            // Recovery amount: rest days get boosted recovery (like Recovery focus)
            let recovery_base: f64 = if !is_training_day {
                7.0 * plan.bonus.physio_mult * plan.medical_facility_mult
            } else {
                match player_focus {
                    TrainingFocus::Recovery => {
                        9.0 * plan.bonus.physio_mult * plan.medical_facility_mult
                    }
                    _ => 3.0 * plan.bonus.physio_mult * plan.medical_facility_mult,
                }
            };

            // Age, morale, and current condition all affect recovery rate.
            // Older players recover more slowly; high morale aids recovery;
            // severely fatigued players have a harder time bouncing back.
            let age = estimate_age(&player.date_of_birth, current_year);
            let age_rec = recovery_factor_from_age(age);
            let morale_rec = recovery_factor_from_morale(player.morale);
            let condition_rec = recovery_factor_from_condition(player.condition);
            let fitness_rec = recovery_factor_from_fitness(player.fitness);

            // Injured players: half base recovery, scaled by age and morale.
            // Fitness decays slowly during injury (inactive = losing sharpness).
            if player.injury.is_some() {
                let recovery = (recovery_base * 0.5 * age_rec * morale_rec * fitness_rec) as u8;
                player.condition = (player.condition + recovery).min(100);
                player.fitness = clamp_fitness(player.fitness as i16 - 1);
                continue;
            }

            // On rest days: only recovery, no attribute gains
            if !is_training_day {
                let stamina_factor = player.attributes.stamina as f64 / 100.0;
                let recovery = (recovery_base
                    * (0.5 + stamina_factor * 0.5)
                    * age_rec
                    * morale_rec
                    * condition_rec
                    * fitness_rec) as u8;
                player.condition = (player.condition + recovery).min(100);
                continue;
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
                * intensity_mult
                * age_factor
                * plan.bonus.coaching_mult
                * plan.bonus.specialization_mult;

            // Apply attribute gains based on player's effective focus
            apply_focus_gains(&mut player.attributes, player_focus, gain);

            // Apply fitness changes based on training focus.
            // Physical training builds fitness; non-physical days slowly decay it if peak.
            // Recovery focus gives a tiny fitness boost.
            apply_fitness_change(&mut player.fitness, player_focus, intensity_mult);

            // Refresh position-weighted OVR and traits after attribute gains.
            refresh_player_derived(player, current_year);

            // Apply condition: deplete from training, then recover
            player.condition = player.condition.saturating_sub(condition_cost);
            let stamina_factor = player.attributes.stamina as f64 / 100.0;
            let recovery = (recovery_base
                * (0.5 + stamina_factor * 0.5)
                * age_rec
                * morale_rec
                * condition_rec
                * fitness_rec) as u8;
            player.condition = (player.condition + recovery).min(100);
        }
    }
}

/// Apply fitness changes based on training focus.
/// Physical training builds fitness (probabilistic small gains).
/// Recovery focus gives a tiny boost. Non-physical training slowly decays high fitness.
fn apply_fitness_change(fitness: &mut u8, focus: &TrainingFocus, intensity_mult: f64) {
    use rand::RngExt;
    let mut rng = rand::rng();
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

fn try_gain(current: &mut u8, gain: f64) {
    use rand::RngExt;
    if *current >= 99 {
        return;
    }
    let mut rng = rand::rng();
    let roll: f64 = rng.random_range(0.0..1.0);
    if roll < gain {
        *current = (*current + 1).min(99);
    }
}

/// Apply attribute gains based on training focus.
fn apply_focus_gains(
    attrs: &mut domain::player::PlayerAttributes,
    focus: &TrainingFocus,
    gain: f64,
) {
    match focus {
        TrainingFocus::Physical => {
            try_gain(&mut attrs.pace, gain);
            try_gain(&mut attrs.stamina, gain);
            try_gain(&mut attrs.strength, gain);
            try_gain(&mut attrs.agility, gain);
        }
        TrainingFocus::Technical => {
            try_gain(&mut attrs.passing, gain);
            try_gain(&mut attrs.shooting, gain);
            try_gain(&mut attrs.dribbling, gain);
        }
        TrainingFocus::Tactical => {
            try_gain(&mut attrs.positioning, gain);
            try_gain(&mut attrs.vision, gain);
            try_gain(&mut attrs.decisions, gain);
            try_gain(&mut attrs.composure, gain);
        }
        TrainingFocus::Defending => {
            try_gain(&mut attrs.tackling, gain);
            try_gain(&mut attrs.defending, gain);
            try_gain(&mut attrs.strength, gain * 0.5);
            try_gain(&mut attrs.positioning, gain * 0.5);
        }
        TrainingFocus::Attacking => {
            try_gain(&mut attrs.shooting, gain);
            try_gain(&mut attrs.dribbling, gain);
            try_gain(&mut attrs.pace, gain * 0.5);
        }
        TrainingFocus::Recovery => {
            // No attribute gains on recovery days
        }
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
