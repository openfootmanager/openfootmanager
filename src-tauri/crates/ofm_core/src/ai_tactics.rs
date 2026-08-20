//! Tactical identity for clubs the player does not manage.
//!
//! `TacticsPhaseSettings` is nine dials, fully wired into both match engines,
//! and until now it had exactly one writer in the whole backend: the user's own
//! `set_tactics_phase` command. Every other club in every save ran
//! `TacticsPhaseSettings::default()`, which is neutral by construction — so the
//! nine-dial system was a single-team feature and every rival in the world
//! played the same way.
//!
//! This module gives each club a blueprint derived from its play style, applied
//! once when the club is built. A HighPress club now actually presses.
//!
//! # Why the dials are not spent evenly
//!
//! The engine does not price these dials symmetrically, and pretending
//! otherwise would hide a real balance problem behind a tidy-looking table.
//! Measured over 4000 matches per option against a neutral opponent
//! (`sim-bench --phase-sweep`), as goals for / goals against against a 1.98 /
//! 1.78 baseline:
//!
//! - **No measurable effect: `build_up_style`, `marking_style`, `tempo`.** Each
//!   is hooked on both match paths, so this is a small-coefficient result rather
//!   than dead code — but it makes them free identity space. They are what a
//!   club's style says about itself at no competitive cost, and they are spent
//!   here freely.
//! - **Honest trades: `pressing_intensity`, `break_speed`.** Fast breaks buy
//!   shots and concede them (2.12 / 1.84). Aggressive pressing buys possession
//!   and pays in fouls, plus a stamina cost that only the live path currently
//!   charges — so its price is real but under-collected until every match runs
//!   through that path.
//! - **One-sided in the club's favour: `defensive_line` low, `defensive_shape`
//!   compact, `width` narrow, `counter_press_duration` long.** A very low line
//!   concedes 1.54 against 1.78 and gives up nothing, because the territory it
//!   surrenders is not modelled. A long counter-press is the largest single
//!   effect on the board — 55.4% possession and 2.12 goals for, at no cost at
//!   all, because the energy it burns and the space it leaves behind are not
//!   modelled either.
//!
//! So blueprints are built for football sense with the one-sided dials
//! **rationed** — a style gets one where the football demands it and neutral
//! elsewhere — rather than budgeted to equalise net value under the current
//! numbers. Budgeting would be tuning against an accounting error: when the
//! engine learns to charge for a counter-press, every budget struck here would
//! be wrong again. Fixing that pricing is its own piece of work, and the
//! identity-vs-identity probe in `tests/tactical_identity_probe.rs` is what
//! says whether it has become urgent.

use domain::team::{
    BreakSpeed, BuildUpStyle, CounterPressDuration, DefensiveLine, DefensiveShape, MarkingStyle,
    PitchWidth, PlayStyle, PressingIntensity, TacticsPhaseSettings, Tempo,
};

/// The nine-dial blueprint a club of this play style takes the field with.
///
/// Balanced is the neutral blueprint, deliberately: it is the style that has no
/// argument to make, and it is the control every other identity is measured
/// against.
pub fn blueprint_for(play_style: &PlayStyle) -> TacticsPhaseSettings {
    match play_style {
        // Nothing to say, said neutrally.
        PlayStyle::Balanced => TacticsPhaseSettings::default(),

        // Get the ball forward and get bodies wide. Gambles a high line to keep
        // the game in the opponent's half and accepts what that concedes.
        PlayStyle::Attacking => TacticsPhaseSettings {
            build_up_style: BuildUpStyle::Short,
            width: PitchWidth::Wide,
            tempo: Tempo::Direct,
            defensive_line: DefensiveLine::High,
            pressing_intensity: PressingIntensity::Medium,
            defensive_shape: DefensiveShape::Normal,
            marking_style: MarkingStyle::Zonal,
            counter_press_duration: CounterPressDuration::Short,
            break_speed: BreakSpeed::Fast,
        },

        // Deep, compact, man-oriented, and in no hurry. Concedes the ball and
        // the tempo on purpose; the low block is the whole idea.
        PlayStyle::Defensive => TacticsPhaseSettings {
            build_up_style: BuildUpStyle::Long,
            width: PitchWidth::Normal,
            tempo: Tempo::Patient,
            defensive_line: DefensiveLine::Low,
            pressing_intensity: PressingIntensity::Passive,
            defensive_shape: DefensiveShape::Compact,
            marking_style: MarkingStyle::ManToMan,
            counter_press_duration: CounterPressDuration::None,
            break_speed: BreakSpeed::Slow,
        },

        // Keep it, circulate it, and win it back immediately when it goes. The
        // long counter-press is the dial that makes possession football work,
        // and it is paid for with a high line and a slow break.
        PlayStyle::Possession => TacticsPhaseSettings {
            build_up_style: BuildUpStyle::Short,
            width: PitchWidth::Wide,
            tempo: Tempo::Patient,
            defensive_line: DefensiveLine::High,
            pressing_intensity: PressingIntensity::Medium,
            defensive_shape: DefensiveShape::Normal,
            marking_style: MarkingStyle::Zonal,
            counter_press_duration: CounterPressDuration::Long,
            break_speed: BreakSpeed::Slow,
        },

        // Sit off, let them come, and go the moment the ball turns over. No
        // counter-press at all — winning it back high is the opposite of the
        // plan.
        PlayStyle::Counter => TacticsPhaseSettings {
            build_up_style: BuildUpStyle::Long,
            width: PitchWidth::Normal,
            tempo: Tempo::Direct,
            defensive_line: DefensiveLine::Low,
            pressing_intensity: PressingIntensity::Passive,
            defensive_shape: DefensiveShape::Normal,
            marking_style: MarkingStyle::Zonal,
            counter_press_duration: CounterPressDuration::None,
            break_speed: BreakSpeed::Fast,
        },

        // Hunt the ball high up the pitch and squeeze the space behind. The
        // most expensive blueprint to run: a high line, aggressive pressing and
        // a long counter-press all at once.
        PlayStyle::HighPress => TacticsPhaseSettings {
            build_up_style: BuildUpStyle::Short,
            width: PitchWidth::Normal,
            tempo: Tempo::Direct,
            defensive_line: DefensiveLine::High,
            pressing_intensity: PressingIntensity::Aggressive,
            defensive_shape: DefensiveShape::Compact,
            marking_style: MarkingStyle::ManToMan,
            counter_press_duration: CounterPressDuration::Long,
            break_speed: BreakSpeed::Fast,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY_STYLE: [PlayStyle; 6] = [
        PlayStyle::Balanced,
        PlayStyle::Attacking,
        PlayStyle::Defensive,
        PlayStyle::Possession,
        PlayStyle::Counter,
        PlayStyle::HighPress,
    ];

    #[test]
    fn every_style_but_balanced_takes_the_field_with_something_to_say() {
        for style in EVERY_STYLE.iter().filter(|s| **s != PlayStyle::Balanced) {
            assert_ne!(
                blueprint_for(style),
                TacticsPhaseSettings::default(),
                "{style:?} is still running the neutral default blueprint"
            );
        }
    }

    #[test]
    fn balanced_is_the_neutral_blueprint_on_purpose() {
        // Not an oversight: it is the control the other five are read against,
        // and the style whose whole character is having no bias.
        assert_eq!(
            blueprint_for(&PlayStyle::Balanced),
            TacticsPhaseSettings::default()
        );
    }

    #[test]
    fn a_high_press_club_actually_presses() {
        let blueprint = blueprint_for(&PlayStyle::HighPress);
        assert_eq!(blueprint.pressing_intensity, PressingIntensity::Aggressive);
        assert_eq!(blueprint.defensive_line, DefensiveLine::High);
        assert_eq!(
            blueprint.counter_press_duration,
            CounterPressDuration::Long,
            "pressing without winning the ball straight back is not a high press"
        );
    }

    #[test]
    fn a_counter_club_sits_deep_and_breaks_fast() {
        let blueprint = blueprint_for(&PlayStyle::Counter);
        assert_eq!(blueprint.defensive_line, DefensiveLine::Low);
        assert_eq!(blueprint.break_speed, BreakSpeed::Fast);
        assert_eq!(
            blueprint.counter_press_duration,
            CounterPressDuration::None,
            "a counter-attacking side does not want the ball back high up the pitch"
        );
    }

    #[test]
    fn a_possession_club_keeps_the_ball_and_hunts_it_back() {
        let blueprint = blueprint_for(&PlayStyle::Possession);
        assert_eq!(blueprint.build_up_style, BuildUpStyle::Short);
        assert_eq!(blueprint.tempo, Tempo::Patient);
        assert_eq!(
            blueprint.counter_press_duration,
            CounterPressDuration::Long,
            "possession football is counter-pressing football"
        );
    }

    #[test]
    fn a_defensive_club_sits_deeper_than_an_attacking_one() {
        assert_eq!(
            blueprint_for(&PlayStyle::Defensive).defensive_line,
            DefensiveLine::Low
        );
        assert_eq!(
            blueprint_for(&PlayStyle::Attacking).defensive_line,
            DefensiveLine::High
        );
    }

    /// The one-sided dials are rationed, and this is what keeps that honest as
    /// the table gets edited. No style may stack every dial the engine
    /// currently under-prices: that is how an identity becomes a free win.
    #[test]
    fn no_identity_stacks_every_under_priced_dial() {
        for style in EVERY_STYLE {
            let blueprint = blueprint_for(&style);
            let one_sided = [
                blueprint.defensive_line == DefensiveLine::VeryLow
                    || blueprint.defensive_line == DefensiveLine::Low,
                blueprint.defensive_shape == DefensiveShape::Compact,
                blueprint.width == PitchWidth::Narrow,
                blueprint.counter_press_duration == CounterPressDuration::Long,
            ];
            let taken = one_sided.iter().filter(|t| **t).count();
            assert!(
                taken <= 2,
                "{style:?} takes {taken} of the four dials the engine under-prices; \
                 no identity may hold more than two until the engine charges for them"
            );
        }
    }

    #[test]
    fn a_blueprint_is_a_pure_function_of_the_style() {
        for style in EVERY_STYLE {
            assert_eq!(
                blueprint_for(&style),
                blueprint_for(&style),
                "{style:?} gave two different blueprints"
            );
        }
    }
}
