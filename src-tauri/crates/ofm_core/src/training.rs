mod fitness_warnings;
mod focus_attributes;
pub use fitness_warnings::{
    ConditionStatus, FitnessStatus, SquadFitness, check_squad_fitness_warnings, condition_status,
    fitness_status, squad_fitness,
};

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
    date: chrono::NaiveDate,
    /// The game's seed, which a player's development traits are drawn from.
    world_seed: u64,
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
/// Every competition counts, whether or not the player follows it: one calendar rule for every club
/// (#869), ahead of the dormant tier's removal (every match will be played for real). Today a dormant
/// match is still settled by scoreline and costs no condition, so closing the ground for it is
/// the rule, not a recovery model. The taper before a match ([`tapering_teams`]) reads every
/// competition the same way.
pub(crate) fn teams_playing_on(game: &Game, date: &str) -> std::collections::HashSet<String> {
    game.competitions_in_play()
        .iter()
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
/// Every competition counts, as in [`teams_playing_on`]: one calendar rule for every club, whether or
/// not its fixtures are played out by the engine today.
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
    // Ages are reckoned on the game's own date.
    let current_date = game.clock.current_date.date_naive();

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
        date: current_date,
        world_seed: game.seed,
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
    let age = domain::age::playing_age(day.date, &player.date_of_birth);
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

    // Younger players grow faster and older ones slower, smoothly: by where the player is on his own
    // development curve (late bloomers and goalkeepers read later) and how far he is from his
    // ceiling. There is no step by age and no gate at the ceiling.
    let traits = crate::development::traits_of(day.world_seed, &player.id);
    let curve = crate::development::curve_age(player, age as i32, &traits);
    let gap = player.potential.saturating_sub(player.ovr);
    let age_factor = crate::development::training_factor(curve, gap);

    // Base gain per attribute per session, boosted by coaching staff
    let gain = 0.15
        * session.intensity_mult
        * age_factor
        * plan.bonus.coaching_mult
        * plan.bonus.specialization_mult;

    let is_goalkeeper = player.natural_position.to_group_position() == Position::Goalkeeper;
    apply_focus_gains(
        &mut player.attributes,
        &session.focus,
        is_goalkeeper,
        gain,
        rng,
    );
    apply_fitness_change(
        &mut player.fitness,
        &session.focus,
        session.intensity_mult,
        rng,
    );

    // Refresh position-weighted OVR and traits after attribute gains.
    refresh_player_derived(player, day.date);

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

    /// Every draw reads as the same point of the unit interval, so a test can set where a roll
    /// lands against a rate.
    struct Roll(u64);

    impl Roll {
        fn at(fraction: f64) -> Self {
            Self((fraction * u64::MAX as f64) as u64)
        }
    }

    impl rand::TryRng for Roll {
        type Error = std::convert::Infallible;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok((self.0 >> 32) as u32)
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok(self.0)
        }
        fn try_fill_bytes(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error> {
            for chunk in bytes.chunks_mut(8) {
                chunk.copy_from_slice(&self.0.to_le_bytes()[..chunk.len()]);
            }
            Ok(())
        }
    }

    /// Zero lies below every positive gain probability, so these scenarios have no lucky-roll dependency.
    fn lowest_roll() -> Roll {
        Roll(0)
    }

    /// One thirty-second succeeds at the primary rate (about 0.05 for a young player far from his
    /// ceiling, after training's share of development) and fails at the secondary (about 0.025).
    fn between_gain_rates() -> Roll {
        Roll(u64::MAX / 32)
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
        refresh_player_derived(&mut player, domain::age::season_midpoint(2026));
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
                date: domain::age::season_midpoint(2026),
                world_seed: 1,
            },
            &mut lowest_roll(),
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
                date: domain::age::season_midpoint(2026),
                world_seed: 1,
            },
            &mut between_gain_rates(),
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

    /// A midfielder born in `birth_year`, rated alike in every attribute, whose ceiling is his rating
    /// (his curve has turned if he is old enough).
    fn peaked_midfielder(birth_year: i32) -> Player {
        let mut player = player_at("peaked", "team1", "2029-06-30");
        player.date_of_birth = format!("{birth_year}-03-15");
        player.condition = 100;
        player.fitness = 80;
        player.morale = 80;
        refresh_player_derived(&mut player, domain::age::season_midpoint(2026));
        player.potential = player.ovr;
        player
    }

    /// Given players of 30 to 35 at their ceiling (the state a falling player is in after any
    ///       summer), with and without a season of training before the summer's decline,
    /// When the net change in rating over the year is averaged,
    /// Then a declining player never ends a year above where it began, trained or not (a rounding's
    ///      worth at 30 and 31, where the table takes nothing yet, a fall from 32): the ceiling is
    ///      his rating, so training has no room to lift him.
    #[test]
    fn a_trained_declining_player_never_ends_the_year_above_where_it_began() {
        use rand::SeedableRng;
        for age in 30..=35 {
            let birth_year = 2026 - age;
            let mut total = 0.0;
            let seeds = 40;
            for seed in 0..seeds {
                let mut player = peaked_midfielder(birth_year);
                let before = f64::from(player.ovr);
                let mut rng = rand_chacha::ChaCha12Rng::seed_from_u64(seed);
                for day in 0..250u32 {
                    train_player(
                        &mut player,
                        &plan(TrainingFocus::Technical),
                        &TrainingDay {
                            weekday_num: day % 5,
                            date: domain::age::season_midpoint(2026),
                            world_seed: 1,
                        },
                        &mut rng,
                    );
                }
                crate::development::develop_one_season(&mut player, age, 1, &mut rng);
                refresh_player_derived(&mut player, domain::age::season_midpoint(2026));
                total += f64::from(player.ovr) - before;
            }
            let net = total / f64::from(seeds as u32);
            // The decline table takes no rating until 32 (a rounding's worth either way), then
            // takes about a point a year; training adds nothing past the ceiling.
            let limit = if age <= 31 { 0.05 } else { -0.3 };
            assert!(net <= limit, "age {age}: net {net:+.2}");
        }
    }

    /// Given a 31-year-old whose ceiling is his rating, and a 24-year-old in the same position,
    /// When a technical session with a successful roll runs for each,
    /// Then neither gains: a peaked player has no room to train into, whatever his age.
    #[test]
    fn a_player_at_his_ceiling_gains_nothing_from_training_at_any_age() {
        let mut veteran = peaked_midfielder(1995);
        let mut young = peaked_midfielder(2002);
        let (veteran_before, young_before) = (attributes(&veteran), attributes(&young));

        session(&mut veteran, &plan(TrainingFocus::Technical), 0);
        session(&mut young, &plan(TrainingFocus::Technical), 0);

        assert_eq!(attributes(&veteran), veteran_before);
        assert_eq!(attributes(&young), young_before);
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
        refresh_player_derived(&mut player, domain::age::season_midpoint(2026));
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

    /// Given two injured players on 1 March 2026, one born 2004-12-15 (21) and one born 2004-01-15
    /// (22), when a rest day is processed, then the younger recovers more condition. The year of
    /// birth alone would call them both 22 and recover them the same.
    #[test]
    fn a_player_not_yet_twenty_two_recovers_better_than_one_who_is() {
        let strong_physio = TeamTrainingPlan {
            bonus: TeamCoachingBonus {
                physio_mult: 3.0,
                ..plan(TrainingFocus::Technical).bonus
            },
            ..plan(TrainingFocus::Technical)
        };
        let recovered_after_a_rest_day = |date_of_birth: &str| {
            let mut player = keeper("team1");
            player.date_of_birth = date_of_birth.to_string();
            player.condition = 40;
            player.injury = Some(Injury {
                name: "test injury".to_string(),
                days_remaining: 30,
            });
            train_player(
                &mut player,
                &strong_physio,
                &TrainingDay {
                    weekday_num: 2,
                    date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                    world_seed: 1,
                },
                &mut lowest_roll(),
            );
            player.condition - 40
        };

        let twenty_one = recovered_after_a_rest_day("2004-12-15");
        let twenty_two = recovered_after_a_rest_day("2004-01-15");

        assert!(
            twenty_one > twenty_two,
            "a 21-year-old recovered {twenty_one}, a 22-year-old {twenty_two}"
        );
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
        refresh_player_derived(&mut player, domain::age::season_midpoint(2026));
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

    /// Wednesday: a rest day on the Balanced schedule.
    const A_REST_DAY: u32 = 2;
    /// Monday: a training day on it.
    const A_TRAINING_DAY: u32 = 0;

    /// A 27-year-old at morale 50, condition 60 and fitness 60, with stamina 100: every recovery
    /// factor is 1.0, so a day off restores exactly the base.
    fn neutral_player() -> Player {
        let mut player = keeper("team1");
        player.date_of_birth = "1999-01-01".to_string();
        player.morale = 50;
        player.condition = 60;
        player.fitness = 60;
        player.attributes.stamina = 100;
        player
    }

    fn injured(player: &mut Player) {
        player.injury = Some(Injury {
            name: "test injury".to_string(),
            days_remaining: 30,
        });
    }

    fn staffed_plan(physio_mult: f64, medical_facility_mult: f64) -> TeamTrainingPlan {
        TeamTrainingPlan {
            bonus: TeamCoachingBonus {
                physio_mult,
                ..plan(TrainingFocus::Technical).bonus
            },
            medical_facility_mult,
            ..plan(TrainingFocus::Technical)
        }
    }

    fn condition_gained_on_a_day_off(player: &mut Player, plan: &TeamTrainingPlan) -> u8 {
        let before = player.condition;
        session(player, plan, A_REST_DAY);
        player.condition - before
    }

    /// Given a player whose recovery factors are all 1.0, with no physio or medical centre to
    ///       speak of
    /// When he has a day off
    /// Then he recovers exactly 10 condition
    #[test]
    fn a_day_off_restores_ten_condition_before_staff_and_facilities() {
        let mut player = neutral_player();

        assert_eq!(
            condition_gained_on_a_day_off(&mut player, &staffed_plan(1.0, 1.0)),
            10
        );
    }

    /// Given the best physio and medical centre (1.4 each, so a base of 19.6), and a tired but very
    ///       fit player: condition 40 (0.9) and fitness 90 (1.2)
    /// When he has a day off
    /// Then he recovers 21: the base scaled by his condition and his fitness
    #[test]
    fn a_tired_players_day_off_is_scaled_by_his_condition_and_fitness() {
        let mut player = neutral_player();
        player.condition = 40;
        player.fitness = 90;

        assert_eq!(
            condition_gained_on_a_day_off(&mut player, &staffed_plan(1.4, 1.4)),
            21
        );
    }

    /// Given an injured player in good spirits and very fit (morale 80: 1.1; fitness 95: 1.2), with
    ///       a base of 19.6
    /// When he has a day off
    /// Then he recovers 12: half the base, scaled by his morale and his fitness
    #[test]
    fn an_injured_players_recovery_is_half_the_base_scaled_by_morale_and_fitness() {
        let mut player = neutral_player();
        player.condition = 40;
        player.morale = 80;
        player.fitness = 95;
        injured(&mut player);

        assert_eq!(
            condition_gained_on_a_day_off(&mut player, &staffed_plan(1.4, 1.4)),
            12
        );
    }

    /// Given an injured player at fitness 80
    /// When a day passes
    /// Then his fitness is 79: an injury costs one point a day
    #[test]
    fn an_injured_player_loses_one_point_of_fitness_a_day() {
        let mut player = neutral_player();
        player.fitness = 80;
        injured(&mut player);

        session(&mut player, &plan(TrainingFocus::Technical), A_REST_DAY);

        assert_eq!(player.fitness, 79);
    }

    /// Given ages either side of each band: 21, 22, 25, 26, 29, 30, 33 and 34
    /// When the recovery factor for age is read
    /// Then it steps down from 1.10 to 1.05 at 22, to 1.00 at 26, to 0.85 at 30 and to 0.70 at 34
    #[test]
    fn recovery_slows_band_by_band_with_age() {
        let factors: Vec<f64> = [21, 22, 25, 26, 29, 30, 33, 34]
            .into_iter()
            .map(recovery_factor_from_age)
            .collect();

        assert_eq!(
            factors,
            vec![1.10, 1.05, 1.05, 1.00, 1.00, 0.85, 0.85, 0.70]
        );
    }

    /// Given fitness either side of each band: 29, 30, 49, 50, 69, 70, 89 and 90
    /// When the recovery factor for fitness is read
    /// Then it steps up from 0.75 to 0.88 at 30, to 1.00 at 50, to 1.12 at 70 and to 1.20 at 90
    #[test]
    fn fitter_players_recover_faster_band_by_band() {
        let factors: Vec<f64> = [29, 30, 49, 50, 69, 70, 89, 90]
            .into_iter()
            .map(recovery_factor_from_fitness)
            .collect();

        assert_eq!(
            factors,
            vec![0.75, 0.88, 0.88, 1.00, 1.00, 1.12, 1.12, 1.20]
        );
    }

    /// Given a club with no staff and a level-three medical centre (1.2), and a player whose
    ///       recovery factors are all 1.0
    /// When the club's rest day is processed
    /// Then he recovers 12: a fifth more than the bare 10
    #[test]
    fn a_level_three_medical_centre_adds_a_fifth_to_a_day_off() {
        let mut game = training_game("team1");
        game.players[0] = neutral_player();
        game.teams[0].facilities.medical = 3;

        process_training(&mut game, A_REST_DAY);

        assert_eq!(game.players[0].condition, 72);
    }

    fn staff(
        id: &str,
        team_id: Option<&str>,
        role: StaffRole,
        coaching: u8,
    ) -> domain::staff::Staff {
        let mut member = domain::staff::Staff::new(
            id.to_string(),
            "Sam".to_string(),
            id.to_string(),
            "1980-01-01".to_string(),
            role,
            domain::staff::StaffAttributes {
                coaching,
                judging_ability: 50,
                judging_potential: 50,
                physiotherapy: 50,
            },
        );
        member.team_id = team_id.map(str::to_string);
        member
    }

    /// Given a club with a coach rated 80 and an assistant manager rated 60, and both another
    ///       club's coach and an unattached coach rated 0
    /// When the club's coaching bonus is worked out
    /// Then it is 1.20: 0.85 plus half the average of its own coaches' ratings, 70
    #[test]
    fn the_coaching_bonus_is_the_clubs_average_coach_rating_on_a_half_point_scale() {
        let mut game = training_game("team1");
        game.staff = vec![
            staff("coach", Some("team1"), StaffRole::Coach, 80),
            staff("assistant", Some("team1"), StaffRole::AssistantManager, 60),
            staff("their-coach", Some("team2"), StaffRole::Coach, 0),
            staff("free-coach", None, StaffRole::Coach, 0),
        ];

        let bonus = compute_coaching_bonus(&game, "team1", &TrainingFocus::Technical);

        assert!(
            (bonus.coaching_mult - 1.20).abs() < 1e-9,
            "{}",
            bonus.coaching_mult
        );
    }

    /// The specialist bonus for `focus` at `team`, in a world where team1's only coach specialises in
    /// fitness and team2's only coach has no speciality.
    fn specialist_bonus(team: &str, focus: TrainingFocus) -> f64 {
        let mut fitness_coach = staff("fitness-coach", Some("team1"), StaffRole::Coach, 50);
        fitness_coach.specialization = Some(CoachingSpecialization::Fitness);
        let mut game = training_game("team1");
        game.staff = vec![
            fitness_coach,
            staff("plain-coach", Some("team2"), StaffRole::Coach, 50),
        ];
        compute_coaching_bonus(&game, team, &focus).specialization_mult
    }

    /// Given a club whose only coach specialises in fitness
    /// When its specialist bonus is read for physical training
    /// Then it is 1.25: a specialist in the session's focus adds a quarter
    #[test]
    fn a_fitness_specialist_adds_a_quarter_to_physical_training() {
        assert_eq!(specialist_bonus("team1", TrainingFocus::Physical), 1.25);
    }

    /// Given the same club
    /// When its specialist bonus is read for technical training
    /// Then it is 1.0: his speciality is not the session's focus
    #[test]
    fn a_fitness_specialist_adds_nothing_to_technical_training() {
        assert_eq!(specialist_bonus("team1", TrainingFocus::Technical), 1.0);
    }

    /// Given a club whose only coach has no speciality
    /// When its specialist bonus is read for physical training
    /// Then it is 1.0
    #[test]
    fn a_coach_without_a_speciality_adds_nothing() {
        assert_eq!(specialist_bonus("team2", TrainingFocus::Physical), 1.0);
    }

    fn coached_plan(coaching_mult: f64, specialization_mult: f64) -> TeamTrainingPlan {
        TeamTrainingPlan {
            bonus: TeamCoachingBonus {
                coaching_mult,
                specialization_mult,
                physio_mult: 1.0,
            },
            ..plan(TrainingFocus::Technical)
        }
    }

    fn attribute_points(player: &Player) -> u64 {
        attributes(player)
            .as_object()
            .expect("attributes serialize as an object")
            .values()
            .filter_map(serde_json::Value::as_u64)
            .sum()
    }

    /// The attribute points a fresh young keeper gains over `sessions` technical sessions on
    /// `plan`, a new keeper each session and one seeded stream of rolls. Each session draws the
    /// same number of rolls whatever it gains, so two plans see the same rolls.
    fn points_gained_over(sessions: usize, plan: &TeamTrainingPlan) -> u64 {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let day = TrainingDay {
            weekday_num: A_TRAINING_DAY,
            date: domain::age::season_midpoint(2026),
            world_seed: 1,
        };
        (0..sessions)
            .map(|_| {
                let mut player = keeper("team1");
                let before = attribute_points(&player);
                train_player(&mut player, plan, &day, &mut rng);
                attribute_points(&player) - before
            })
            .sum()
    }

    /// Given a young keeper trained 4,000 times by a staff worth 1.25 and by one worth 0.8, on the
    ///       same rolls
    /// When the attribute points gained are counted
    /// Then the better staff brings at least a third more: about 1.56 times as many
    #[test]
    fn a_better_coaching_staff_brings_more_attribute_gains() {
        let weak = points_gained_over(4_000, &coached_plan(0.8, 1.0));
        let strong = points_gained_over(4_000, &coached_plan(1.25, 1.0));

        assert!(
            strong as f64 > weak as f64 * 1.33,
            "{strong} against {weak}"
        );
    }

    /// Given a young keeper trained 4,000 times with a specialist coach (1.25) and without one, on
    ///       the same rolls
    /// When the attribute points gained are counted
    /// Then the specialist brings at least a tenth more: about 1.25 times as many
    #[test]
    fn a_specialist_coach_brings_more_attribute_gains() {
        let plain = points_gained_over(4_000, &coached_plan(1.0, 1.0));
        let specialist = points_gained_over(4_000, &coached_plan(1.0, 1.25));

        assert!(
            specialist as f64 > plain as f64 * 1.1,
            "{specialist} against {plain}"
        );
    }

    fn fitness_after(fitness: u8, focus: TrainingFocus, roll: Roll) -> u8 {
        fitness_after_at(1.0, fitness, focus, roll)
    }

    fn fitness_after_at(
        intensity_mult: f64,
        fitness: u8,
        focus: TrainingFocus,
        mut roll: Roll,
    ) -> u8 {
        let mut value = fitness;
        apply_fitness_change(&mut value, &focus, intensity_mult, &mut roll);
        value
    }

    /// Given a player at fitness 80 in a medium physical session, a 1.5% chance of a point
    /// When the roll is 0
    /// Then he gains a point
    #[test]
    fn physical_training_adds_a_fitness_point_on_a_roll_under_its_rate() {
        assert_eq!(
            fitness_after(80, TrainingFocus::Physical, Roll::at(0.0)),
            81
        );
    }

    /// Given the same session
    /// When the roll is 0.02, above the 1.5%
    /// Then he gains nothing
    #[test]
    fn physical_training_adds_nothing_on_a_roll_above_its_rate() {
        assert_eq!(
            fitness_after(80, TrainingFocus::Physical, Roll::at(0.02)),
            80
        );
    }

    /// Given a player at fitness 80 in a recovery session, a 5% chance of a point
    /// When the roll is 0
    /// Then he gains a point
    #[test]
    fn a_recovery_session_adds_a_fitness_point_on_a_roll_under_one_in_twenty() {
        assert_eq!(
            fitness_after(80, TrainingFocus::Recovery, Roll::at(0.0)),
            81
        );
    }

    /// Given the same session
    /// When the roll is 0.06, above the 5%
    /// Then he gains nothing
    #[test]
    fn a_recovery_session_adds_nothing_on_a_roll_above_one_in_twenty() {
        assert_eq!(
            fitness_after(80, TrainingFocus::Recovery, Roll::at(0.06)),
            80
        );
    }

    /// Given a player at fitness 100 and a roll of 0
    /// When he has a physical session
    /// Then he stays at 100
    #[test]
    fn physical_training_never_lifts_fitness_past_one_hundred() {
        assert_eq!(
            fitness_after(100, TrainingFocus::Physical, Roll::at(0.0)),
            100
        );
    }

    /// Given a player at fitness 100 and a roll of 0
    /// When he has a recovery session
    /// Then he stays at 100
    #[test]
    fn a_recovery_session_never_lifts_fitness_past_one_hundred() {
        assert_eq!(
            fitness_after(100, TrainingFocus::Recovery, Roll::at(0.0)),
            100
        );
    }

    /// Given a player at fitness 80 and a roll of 0.02, between the medium rate (1.5%) and the high
    ///       one (2.25%)
    /// When he has a high-intensity physical session
    /// Then he gains a point: intensity scales the chance up
    #[test]
    fn a_high_intensity_physical_session_raises_the_chance_of_a_fitness_point() {
        assert_eq!(
            fitness_after_at(1.5, 80, TrainingFocus::Physical, Roll::at(0.02)),
            81
        );
    }

    /// Given a player at fitness 90 in a technical session, a 5% chance of losing a point
    /// When the roll is 0
    /// Then he loses a point
    #[test]
    fn a_very_fit_player_loses_a_point_on_a_roll_under_one_in_twenty() {
        assert_eq!(
            fitness_after(90, TrainingFocus::Technical, Roll::at(0.0)),
            89
        );
    }

    /// Given the same session
    /// When the roll is 0.06, above the 5%
    /// Then he keeps all 90
    #[test]
    fn a_very_fit_player_keeps_his_fitness_on_a_roll_above_one_in_twenty() {
        assert_eq!(
            fitness_after(90, TrainingFocus::Technical, Roll::at(0.06)),
            90
        );
    }

    /// Given a player at fitness exactly 85 in a technical session and a roll of 0
    /// When the session ends
    /// Then he keeps all 85: only fitness above 85 needs maintaining
    #[test]
    fn fitness_of_eighty_five_needs_no_maintaining() {
        assert_eq!(
            fitness_after(85, TrainingFocus::Technical, Roll::at(0.0)),
            85
        );
    }

    /// Given a club with physios rated 80 and 40, and another club's physio rated 0
    /// When the club's physio bonus is worked out
    /// Then it is 1.24: 1.0 plus four tenths of its own physios' average, 60
    #[test]
    fn the_physio_bonus_is_the_clubs_average_physio_rating_on_a_four_tenths_scale() {
        let physio = |id: &str, team: &str, rating: u8| {
            let mut member = staff(id, Some(team), StaffRole::Physio, 50);
            member.attributes.physiotherapy = rating;
            member
        };
        let mut game = training_game("team1");
        game.staff = vec![
            physio("good", "team1", 80),
            physio("fair", "team1", 40),
            physio("theirs", "team2", 0),
        ];

        let bonus = compute_coaching_bonus(&game, "team1", &TrainingFocus::Technical);

        assert!(
            (bonus.physio_mult - 1.24).abs() < 1e-9,
            "{}",
            bonus.physio_mult
        );
    }

    /// Given a player with stamina 50 (recovery scaled by 0.75) whose other factors are all 1.0
    /// When he has a day off with no physio or medical centre to speak of
    /// Then he recovers 7: three quarters of the bare 10
    #[test]
    fn a_player_with_less_stamina_recovers_less_on_a_day_off() {
        let mut player = neutral_player();
        player.attributes.stamina = 50;

        assert_eq!(
            condition_gained_on_a_day_off(&mut player, &staffed_plan(1.0, 1.0)),
            7
        );
    }

    /// A 27-year-old at condition 60, morale 80 (1.1) and fitness 75 (1.12), with stamina 100, who
    /// trains on Monday with the best physio and medical centre (1.4 each): the change in his
    /// condition over the session.
    fn condition_change_over_a_session(focus: TrainingFocus) -> i16 {
        let mut player = neutral_player();
        player.morale = 80;
        player.fitness = 75;
        let mut plan = staffed_plan(1.4, 1.4);
        plan.default_focus = focus;

        session(&mut player, &plan, A_TRAINING_DAY);

        i16::from(player.condition) - 60
    }

    /// Given that player in a recovery session, which costs no condition
    /// When the session ends
    /// Then he is 21 better: nine times the staff's 1.96 (17.64), scaled by his morale and fitness
    #[test]
    fn a_recovery_session_restores_nine_times_the_staffs_base() {
        assert_eq!(condition_change_over_a_session(TrainingFocus::Recovery), 21);
    }

    /// Given that player in a medium technical session, which costs 6 condition
    /// When the session ends
    /// Then he is 1 better: it restores 7, three times the staff's 1.96, scaled by his morale and
    ///      fitness
    #[test]
    fn an_ordinary_session_restores_three_times_the_staffs_base() {
        assert_eq!(condition_change_over_a_session(TrainingFocus::Technical), 1);
    }

    fn planned_focus_at_condition(condition: u8) -> TrainingFocus {
        let mut player = neutral_player();
        player.condition = condition;
        player_training_session(&player, &plan(TrainingFocus::Technical), A_TRAINING_DAY).focus
    }

    /// Given a player at condition exactly 40 on a technical training day
    /// When his session is planned
    /// Then he trains as planned: the fatigue guard is below 40
    #[test]
    fn a_player_at_forty_condition_trains_as_planned() {
        assert_eq!(planned_focus_at_condition(40), TrainingFocus::Technical);
    }

    /// Given a player at condition 39 on the same day
    /// When his session is planned
    /// Then he is rested: his session becomes recovery work
    #[test]
    fn a_player_below_forty_condition_is_rested() {
        assert_eq!(planned_focus_at_condition(39), TrainingFocus::Recovery);
    }

    fn intensity_plan(intensity: TrainingIntensity) -> TeamTrainingPlan {
        TeamTrainingPlan {
            intensity,
            ..plan(TrainingFocus::Technical)
        }
    }

    /// Given a young keeper trained 4,000 times at high intensity (1.5) and at low (0.5), on the
    ///       same rolls
    /// When the attribute points gained are counted
    /// Then high intensity brings at least twice as many: about three times
    #[test]
    fn a_high_intensity_session_brings_more_attribute_gains_than_a_low_one() {
        let low = points_gained_over(4_000, &intensity_plan(TrainingIntensity::Low));
        let high = points_gained_over(4_000, &intensity_plan(TrainingIntensity::High));

        assert!(high as f64 > low as f64 * 2.0, "{high} against {low}");
    }
}
