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
//! - **Were one-sided in the club's favour: `defensive_line` low,
//!   `defensive_shape` compact, `width` narrow, `counter_press_duration` long.**
//!   When the blueprints were written, a very low line conceded 1.54 against
//!   1.78 and gave up nothing, and a long counter-press bought 55.4% possession
//!   and 2.12 goals for at no cost at all. The engine now charges the other half
//!   of each trade (`engine::shared`'s `tactics_defensive_line_recovery`,
//!   `tactics_break_distance`, `tactics_counter_press_exposure` and
//!   `tactics_width_versus_shape`), and the identity probe's spread between the
//!   best and worst style fell from 6.3 points to about 4.
//!
//! The blueprints still **ration** those four — a style gets one where the
//! football demands it and neutral elsewhere — rather than budgeting to
//! equalise net value. The pricing narrowed the gap; it has not been shown to
//! close it, and budgeting against numbers that move every time the engine is
//! re-priced would be wrong again at the next change. The ration is a pin to
//! revisit after a fresh `--phase-sweep`: if that run shows the four dials
//! trading evenly, it can go. `tests/tactical_identity_probe.rs` is what says
//! whether it has become urgent either way.

use crate::ai_math::mean_u8;
use crate::game::Game;
use crate::stable_hash::stable_hash;
use domain::league::FixtureStatus;
use domain::player::{Player, Position};
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

// ---------------------------------------------------------------------------
// The weekly review
// ---------------------------------------------------------------------------

/// The four dials `--phase-sweep` found the engine giving away, and the most any
/// one club may hold. The blueprints are rationed by hand; adaptation has to
/// obey the same limit at run time, because "we keep conceding" pushes a club
/// straight at the deep line and the compact block. The engine now charges for
/// all four, but not provably in full — see the module docs for when this
/// limit can be revisited.
const MAX_UNDER_PRICED_DIALS: usize = 2;

fn under_priced_dials(settings: &TacticsPhaseSettings) -> usize {
    [
        matches!(
            settings.defensive_line,
            DefensiveLine::VeryLow | DefensiveLine::Low
        ),
        settings.defensive_shape == DefensiveShape::Compact,
        settings.width == PitchWidth::Narrow,
        settings.counter_press_duration == CounterPressDuration::Long,
    ]
    .iter()
    .filter(|taken| **taken)
    .count()
}

/// A club sits down to look at itself once a week, on a day of its own.
///
/// Not every club on the same day: the review is a sweep over every squad in the
/// world, and spreading it over the week keeps that off any single day's
/// advance. Not on a fixed rota either — the day is derived from the club's id,
/// so it survives a save and a reload and does not depend on where the club sits
/// in the list.
///
/// A week is also the hysteresis. The plan is recomputed from scratch each time
/// rather than nudged from where it was, so a club cannot drift; what a weekly
/// cadence buys is that it cannot flip-flop on a Tuesday either.
const REVIEW_CYCLE_DAYS: u64 = 7;

/// Distinct from the lineup builder's stream, which hashes the same club ids for
/// a different question.
const REVIEW_SEED: u64 = 0x7461_6374_6963_7300; // "tactics\0"

fn review_weekday(team_id: &str) -> u32 {
    (stable_hash(team_id.as_bytes(), REVIEW_SEED) % REVIEW_CYCLE_DAYS) as u32
}

// --- What the manager looks at ---------------------------------------------

/// How many of the eleven best available players the legs reading is taken over.
const LIKELY_STARTERS: usize = 11;
/// How many defenders decide whether the line can be pushed up.
const BACK_LINE: usize = 4;

// Both caps are placed at roughly the twentieth percentile of a generated world
// (`tests/tactical_adaptation_probe.rs`, 440 clubs), so the rule they encode is
// one sentence: the least equipped fifth of the world cannot run the most
// demanding version of its style. A cap at the median is not a cap, it is a
// second blueprint; a cap below the tenth percentile never fires.

/// Below this mean stamina across the likely eleven, a squad cannot sustain a
/// pressing game for ninety minutes however much its manager would like to.
/// The world's likely-XI stamina runs 62.1 at p10 and 68.6 at the median.
const LEGS_FOR_A_PRESS: f64 = 64.0;
/// Below this mean pace across the back line, the space behind a high line is
/// not space this defence can cover. The world's back-four pace runs 55.8 at
/// p10 and 66.0 at the median — it is far more spread than stamina, which is
/// why the same percentile lands on a much lower number.
const PACE_FOR_A_HIGH_LINE: f64 = 59.0;

/// What the squad can be asked to do, as opposed to what the badge says.
struct SquadReading {
    /// Mean stamina of the eleven best available players.
    legs: f64,
    /// Mean pace of the four best defenders.
    defensive_pace: f64,
}

fn read_squad(squad: &[&Player]) -> Option<SquadReading> {
    if squad.is_empty() {
        return None;
    }

    // Ties broken on id so the reading does not depend on storage order.
    let by_standing = |candidates: &mut Vec<&&Player>| {
        candidates.sort_by(|a, b| b.ovr.cmp(&a.ovr).then_with(|| a.id.cmp(&b.id)));
    };

    let mut available: Vec<&&Player> = squad.iter().filter(|p| p.injury.is_none()).collect();
    if available.is_empty() {
        // An entire squad in the treatment room is still the squad the manager
        // has to plan around.
        available = squad.iter().collect();
    }
    by_standing(&mut available);
    let eleven = &available[..LIKELY_STARTERS.min(available.len())];
    let legs = mean_u8(eleven.iter().map(|p| p.attributes.stamina));

    let mut defenders: Vec<&&Player> = available
        .iter()
        .copied()
        .filter(|p| p.position.to_group_position() == Position::Defender)
        .collect();
    by_standing(&mut defenders);
    let defensive_pace = if defenders.is_empty() {
        // No recognised defenders is a broken squad, not a fast one. Read the
        // eleven rather than claim the back line can do anything.
        mean_u8(eleven.iter().map(|p| p.attributes.pace))
    } else {
        mean_u8(
            defenders[..BACK_LINE.min(defenders.len())]
                .iter()
                .map(|p| p.attributes.pace),
        )
    };

    Some(SquadReading {
        legs,
        defensive_pace,
    })
}

/// How many recent matches the manager judges the plan on.
const FORM_WINDOW: usize = 5;
/// Fewer results than this is not evidence, it is a new season.
const FORM_MINIMUM: usize = 3;

// Both triggers sit at the same tail of the same measured distribution: p85 of
// goals conceded and p15 of goals scored, over every five-match window a probe
// season produces (`tests/tactical_adaptation_probe.rs`). The two tails mirror
// each other because every match is counted from both ends. So these are the
// worst and best sixth of runs a club actually lives through.
//
// They are measurements, and they move when the football does. First set at 2.8
// and 1.2 when a club scored 2.06 a game; fielding a real, rested eleven instead
// of a whole tired squad lifted that to about 2.29, where 2.8 conceded sits near
// p70 and a third of clubs read as leaking. Six probe seasons put p85 at 3.2 in
// four of them and p15 at 1.4 in four. Anything that changes how many goals are
// scored should re-read that table before trusting these.
//
// The first pass used 2.2 and 0.8, which read like a matched pair and were
// nothing of the kind: 2.2 sat just above the median and fired for two clubs in
// five, while 0.8 sat below the fifth percentile and fired for almost nobody.
// Every club in the world was permanently "leaking" and none was ever blunt,
// and the reaction table came out as one column of compact blocks.

/// Conceding at this rate says the plan is not holding, whatever the badge says.
const LEAKY: f64 = 3.2;
/// Scoring at this rate says the same about the other end.
const BLUNT: f64 = 1.4;

struct FormReading {
    conceded_per_game: f64,
    scored_per_game: f64,
}

/// Every club's recent results, most recent first, across every competition it
/// plays in — a cup exit says as much about a plan as a league defeat.
///
/// Built as one pass over the world's fixtures and then looked up, rather than
/// scanned once per club. `tapering_teams` learned this the same way: this runs
/// for a seventh of the clubs in the world every day, and a populated world
/// holds tens of thousands of fixtures.
///
/// Keyed by owned id rather than by borrow: the caller writes back through
/// `game.teams` while holding this, and a key borrowed out of the fixture list
/// would pin `game.competitions` for the whole loop.
fn read_form(game: &Game) -> std::collections::HashMap<String, FormReading> {
    let mut results: std::collections::HashMap<&str, Vec<(&str, u32, u32)>> = Default::default();
    for fixture in game
        .competitions_in_play()
        .iter()
        .flat_map(|competition| competition.fixtures.iter())
        .filter(|fixture| fixture.status == FixtureStatus::Completed)
    {
        let Some(result) = fixture.result.as_ref() else {
            continue;
        };
        let (home, away) = (u32::from(result.home_goals), u32::from(result.away_goals));
        results
            .entry(fixture.home_team_id.as_str())
            .or_default()
            .push((fixture.date.as_str(), home, away));
        results
            .entry(fixture.away_team_id.as_str())
            .or_default()
            .push((fixture.date.as_str(), away, home));
    }

    results
        .into_iter()
        .filter_map(|(team_id, mut played)| {
            if played.len() < FORM_MINIMUM {
                return None;
            }
            // ISO dates sort lexicographically, so ordering them needs no parsing.
            played.sort_by(|a, b| b.0.cmp(a.0));
            let window = &played[..FORM_WINDOW.min(played.len())];
            let games = window.len() as f64;
            Some((
                team_id.to_string(),
                FormReading {
                    scored_per_game: window.iter().map(|(_, s, _)| f64::from(*s)).sum::<f64>()
                        / games,
                    conceded_per_game: window.iter().map(|(_, _, c)| f64::from(*c)).sum::<f64>()
                        / games,
                },
            ))
        })
        .collect()
}

// --- What he does about it -------------------------------------------------

/// One step deeper, or `false` if the side is already on its own goal line.
fn sit_deeper(settings: &mut TacticsPhaseSettings) -> bool {
    settings.defensive_line = match settings.defensive_line {
        DefensiveLine::High => DefensiveLine::Medium,
        DefensiveLine::Medium => DefensiveLine::Low,
        DefensiveLine::Low => DefensiveLine::VeryLow,
        DefensiveLine::VeryLow => return false,
    };
    true
}

fn close_the_gaps(settings: &mut TacticsPhaseSettings) -> bool {
    if settings.defensive_shape == DefensiveShape::Compact {
        return false;
    }
    settings.defensive_shape = DefensiveShape::Compact;
    true
}

fn press_less(settings: &mut TacticsPhaseSettings) -> bool {
    settings.pressing_intensity = match settings.pressing_intensity {
        PressingIntensity::Aggressive => PressingIntensity::Medium,
        PressingIntensity::Medium => PressingIntensity::Passive,
        PressingIntensity::Passive => return false,
    };
    true
}

/// Win it back closer to their goal. The answer for a side that is already as
/// fast, as wide and as direct as it can get — which every attacking blueprint
/// is on the day it is written.
fn press_higher(settings: &mut TacticsPhaseSettings) -> bool {
    settings.pressing_intensity = match settings.pressing_intensity {
        PressingIntensity::Passive => PressingIntensity::Medium,
        PressingIntensity::Medium => PressingIntensity::Aggressive,
        PressingIntensity::Aggressive => return false,
    };
    true
}

fn break_faster(settings: &mut TacticsPhaseSettings) -> bool {
    settings.break_speed = match settings.break_speed {
        // The engine gives Slow and Medium the same zero counter chance. A
        // blunt side must reach Fast to change what happens on the pitch.
        BreakSpeed::Slow => BreakSpeed::Fast,
        BreakSpeed::Medium => BreakSpeed::Fast,
        BreakSpeed::Fast => return false,
    };
    true
}

fn stretch_the_pitch(settings: &mut TacticsPhaseSettings) -> bool {
    if settings.width == PitchWidth::Wide {
        return false;
    }
    settings.width = PitchWidth::Wide;
    true
}

fn go_more_direct(settings: &mut TacticsPhaseSettings) -> bool {
    if settings.tempo == Tempo::Direct {
        return false;
    }
    settings.tempo = Tempo::Direct;
    true
}

/// Apply the first change on the list that does something and does not push the
/// club past its ration of under-priced dials.
///
/// One change per problem, deliberately. A manager reacting to a bad run moves
/// one thing; a policy that moved all three would have rebuilt the blueprint,
/// and the club's identity would last exactly one bad month.
fn shade(settings: &mut TacticsPhaseSettings, moves: &[fn(&mut TacticsPhaseSettings) -> bool]) {
    for change in moves {
        let mut trial = settings.clone();
        if change(&mut trial) && under_priced_dials(&trial) <= MAX_UNDER_PRICED_DIALS {
            *settings = trial;
            return;
        }
    }
}

/// The plan this club takes into its next match.
///
/// Recomputed from the style blueprint every time rather than nudged from
/// whatever is stored, which is what keeps it honest: the answer is a pure
/// function of style, squad and form, so a club cannot ratchet its way to
/// something no manager ever chose, and running the review twice in a day
/// changes nothing.
fn match_plan(
    play_style: &PlayStyle,
    squad: Option<&SquadReading>,
    form: Option<&FormReading>,
) -> TacticsPhaseSettings {
    let mut settings = blueprint_for(play_style);

    if let Some(form) = form {
        if form.conceded_per_game >= LEAKY {
            shade(&mut settings, &[close_the_gaps, sit_deeper, press_less]);
        }
        if form.scored_per_game <= BLUNT {
            shade(
                &mut settings,
                &[
                    break_faster,
                    stretch_the_pitch,
                    press_higher,
                    go_more_direct,
                ],
            );
        }
    }

    // What the players can actually do has the last word. A bad run is a reason
    // to try something; it is not a reason to ask a squad for something it does
    // not have, so the caps close over the reaction as well as the blueprint.
    // They only ever ask for less, so nothing here can breach the ration.
    if let Some(squad) = squad {
        if squad.legs < LEGS_FOR_A_PRESS {
            if settings.pressing_intensity == PressingIntensity::Aggressive {
                settings.pressing_intensity = PressingIntensity::Medium;
            }
            if settings.counter_press_duration == CounterPressDuration::Long {
                settings.counter_press_duration = CounterPressDuration::Short;
            }
        }
        if squad.defensive_pace < PACE_FOR_A_HIGH_LINE
            && settings.defensive_line == DefensiveLine::High
        {
            settings.defensive_line = DefensiveLine::Medium;
        }
    }

    settings
}

/// Every club the player does not manage looks at how it plays, once a week.
///
/// This is what pays for slices 6 and 7 shipping without a backfill. A save made
/// before them carries the neutral blueprint and an empty role map on every AI
/// club; nothing here treats that as a special case, so those clubs simply get
/// their identity at their first review like everyone else.
///
/// Deliberate, and worth saying out loud: a club the player leaves becomes an AI
/// club, and its next review overwrites the tactics and the roles the player set
/// there. That is a new manager taking over and doing it his way.
pub fn apply_ai_tactical_reviews(game: &mut Game, weekday_num: u32) {
    let user_team_id = game.manager.team_id.clone();
    let due: Vec<String> = game
        .teams
        .iter()
        .filter(|team| Some(&team.id) != user_team_id.as_ref())
        .filter(|team| review_weekday(&team.id) == weekday_num)
        .map(|team| team.id.clone())
        .collect();

    // One pass over the world's fixtures for everybody, not one per club.
    let form = read_form(game);

    for team_id in due {
        let Some(team_index) = game.teams.iter().position(|team| team.id == team_id) else {
            continue;
        };

        // Disjoint field borrows: the squad is read out of `game.players` while
        // the club is written through `game.teams`.
        let squad: Vec<&Player> = game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            .collect();
        let reading = read_squad(&squad);

        let team = &mut game.teams[team_index];
        team.tactics_phase = match_plan(&team.play_style, reading.as_ref(), form.get(&team_id));
        crate::ai_roles::assign_squad_roles(team, squad.iter().copied());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, League, MatchResult};
    use domain::manager::Manager;
    use domain::player::PlayerAttributes;
    use domain::team::Team;

    fn form_game() -> Game {
        let clock =
            crate::clock::GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager".into(),
            "Test".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        manager.hire("user".into());
        let mut teams: Vec<Team> = ["home", "away", "user"]
            .into_iter()
            .map(|id| {
                Team::new(
                    id.into(),
                    id.into(),
                    id.into(),
                    "England".into(),
                    "London".into(),
                    "Ground".into(),
                    20_000,
                )
            })
            .collect();
        teams[0].play_style = PlayStyle::Attacking;
        let participants = teams.iter().map(|team| team.id.clone()).collect::<Vec<_>>();
        let mut league = League::new("league".into(), "League".into(), 2026, &participants);
        for (index, (home, away, home_goals, away_goals)) in [
            ("home", "away", 1, 4),
            ("away", "home", 5, 0),
            ("home", "away", 0, 3),
        ]
        .into_iter()
        .enumerate()
        {
            league.fixtures.push(Fixture {
                id: format!("fixture-{index}"),
                competition_id: "league".into(),
                matchday: index as u32 + 1,
                date: format!("2026-08-{:02}", index + 1),
                home_team_id: home.into(),
                away_team_id: away.into(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals,
                    away_goals,
                    ..Default::default()
                }),
            });
        }
        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);
        game.competitions = vec![league];
        game
    }

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

    // -----------------------------------------------------------------------
    // The weekly review
    // -----------------------------------------------------------------------

    fn a_squad_that_can_run() -> SquadReading {
        SquadReading {
            legs: 75.0,
            defensive_pace: 75.0,
        }
    }

    fn a_squad_with_no_legs() -> SquadReading {
        SquadReading {
            legs: 40.0,
            defensive_pace: 75.0,
        }
    }

    fn a_slow_back_line() -> SquadReading {
        SquadReading {
            legs: 75.0,
            defensive_pace: 40.0,
        }
    }

    /// Conceding at exactly the rate that says the plan is not holding, and
    /// scoring well enough that the other trigger stays quiet.
    fn leaking() -> FormReading {
        FormReading {
            conceded_per_game: LEAKY,
            scored_per_game: 2.0,
        }
    }

    /// Scoring at exactly the blunt rate, and keeping them out at the other end.
    fn toothless() -> FormReading {
        FormReading {
            conceded_per_game: 1.0,
            scored_per_game: BLUNT,
        }
    }

    #[test]
    fn a_squad_good_enough_to_run_the_plan_and_no_bad_run_behind_it_just_plays_the_plan() {
        for style in EVERY_STYLE {
            assert_eq!(
                match_plan(&style, Some(&a_squad_that_can_run()), None),
                blueprint_for(&style),
                "{style:?} was talked out of its own identity for no reason"
            );
        }
    }

    #[test]
    fn a_squad_without_the_legs_is_not_asked_to_press_for_ninety_minutes() {
        let plan = match_plan(&PlayStyle::HighPress, Some(&a_squad_with_no_legs()), None);
        assert_eq!(
            plan.pressing_intensity,
            PressingIntensity::Medium,
            "a squad that cannot run was still sent out to press aggressively"
        );
        assert_eq!(plan.counter_press_duration, CounterPressDuration::Short);
    }

    #[test]
    fn a_slow_back_line_is_not_asked_to_hold_a_high_line() {
        for style in [
            PlayStyle::Attacking,
            PlayStyle::Possession,
            PlayStyle::HighPress,
        ] {
            let plan = match_plan(&style, Some(&a_slow_back_line()), None);
            assert_ne!(
                plan.defensive_line,
                DefensiveLine::High,
                "{style:?} pushed a defence that cannot turn up the pitch"
            );
        }
    }

    #[test]
    fn a_side_that_keeps_conceding_changes_something_at_the_back() {
        for style in EVERY_STYLE {
            let settled = match_plan(&style, Some(&a_squad_that_can_run()), None);
            let reacting = match_plan(&style, Some(&a_squad_that_can_run()), Some(&leaking()));
            assert_ne!(
                settled, reacting,
                "{style:?} conceded three a game for a month and changed nothing"
            );
        }
    }

    #[test]
    fn a_side_that_cannot_score_changes_something_going_forward() {
        for style in EVERY_STYLE {
            let settled = match_plan(&style, Some(&a_squad_that_can_run()), None);
            let reacting = match_plan(&style, Some(&a_squad_that_can_run()), Some(&toothless()));
            assert_ne!(
                settled, reacting,
                "{style:?} has not scored in a month and changed nothing"
            );
        }
    }

    #[test]
    fn blunt_slow_blueprints_gain_an_effective_fast_break() {
        for style in [PlayStyle::Defensive, PlayStyle::Possession] {
            let plan = match_plan(&style, None, Some(&toothless()));
            assert_eq!(
                plan.break_speed,
                BreakSpeed::Fast,
                "{style:?} still cannot break fast"
            );
            assert_ne!(plan.break_speed, blueprint_for(&style).break_speed);
        }
    }

    #[test]
    fn completed_home_and_away_results_read_from_each_clubs_perspective() {
        let game = form_game();
        let form = read_form(&game);
        let home = &form["home"];
        let away = &form["away"];
        assert_eq!(home.scored_per_game, 1.0 / 3.0);
        assert_eq!(home.conceded_per_game, 4.0);
        assert_eq!(away.scored_per_game, 4.0);
        assert_eq!(away.conceded_per_game, 1.0 / 3.0);
    }

    #[test]
    fn a_leaky_club_reacts_to_real_results_at_its_review() {
        let mut game = form_game();
        apply_ai_tactical_reviews(&mut game, review_weekday("home"));
        let home = game.teams.iter().find(|team| team.id == "home").unwrap();
        assert_eq!(home.tactics_phase.defensive_shape, DefensiveShape::Compact);
    }

    #[test]
    fn a_second_review_of_the_same_game_changes_nothing() {
        let mut game = form_game();
        let weekday = review_weekday("home");
        apply_ai_tactical_reviews(&mut game, weekday);
        let first = game.teams.clone();
        assert_ne!(first[0].tactics_phase, blueprint_for(&PlayStyle::Attacking));
        apply_ai_tactical_reviews(&mut game, weekday);
        for (first, second) in first.iter().zip(&game.teams) {
            assert_eq!(first.tactics_phase, second.tactics_phase);
            assert_eq!(first.player_roles, second.player_roles);
        }
    }

    #[test]
    fn squad_reading_excludes_injured_stars_and_reads_the_defenders_pace() {
        let make_player = |id: &str, position: Position, stamina: u8, pace: u8, injured: bool| {
            let attributes = PlayerAttributes {
                pace,
                stamina,
                strength: 50,
                agility: 50,
                passing: 50,
                shooting: 50,
                tackling: 50,
                dribbling: 50,
                defending: 50,
                positioning: 50,
                vision: 50,
                decisions: 50,
                composure: 50,
                aggression: 50,
                teamwork: 50,
                leadership: 50,
                handling: 50,
                reflexes: 50,
                aerial: 50,
            };
            let mut player = Player::new(
                id.into(),
                id.into(),
                id.into(),
                "1998-01-01".into(),
                "England".into(),
                position,
                attributes,
            );
            player.ovr = if injured { 99 } else { 60 };
            if injured {
                player.injury = Some(domain::player::Injury {
                    name: "Test".into(),
                    days_remaining: 3,
                });
            }
            player
        };
        let players = [
            make_player("defender", Position::Defender, 50, 45, false),
            make_player("forward", Position::Forward, 70, 90, false),
            make_player("injured", Position::Defender, 99, 99, true),
        ];
        let squad = players.iter().collect::<Vec<_>>();
        let reading = read_squad(&squad).unwrap();
        assert_eq!(reading.legs, 60.0);
        assert_eq!(reading.defensive_pace, 45.0);
    }

    /// The reason this exists: the natural answer to conceding is a deeper line
    /// and a compact block, two of the four dials the engine priced one-sidedly.
    /// Left alone, adaptation would quietly undo the ration on the blueprints,
    /// and every struggling club in the world would converge on the same shape.
    #[test]
    fn no_adaptation_stacks_every_under_priced_dial() {
        let squads = [
            None,
            Some(a_squad_that_can_run()),
            Some(a_squad_with_no_legs()),
            Some(a_slow_back_line()),
        ];
        let forms = [
            None,
            Some(leaking()),
            Some(toothless()),
            Some(FormReading {
                conceded_per_game: 4.0,
                scored_per_game: 0.0,
            }),
        ];
        for style in EVERY_STYLE {
            for squad in &squads {
                for form in &forms {
                    let plan = match_plan(&style, squad.as_ref(), form.as_ref());
                    let taken = under_priced_dials(&plan);
                    assert!(
                        taken <= MAX_UNDER_PRICED_DIALS,
                        "{style:?} ended up holding {taken} of the four under-priced \
                         dials after adapting: {plan:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn reviewing_twice_says_the_same_thing_twice() {
        // The plan is rebuilt from the blueprint each time rather than nudged
        // from what is stored, so a club cannot ratchet itself somewhere no
        // manager chose. A second review on the same evidence is a no-op.
        let first = match_plan(
            &PlayStyle::HighPress,
            Some(&a_squad_with_no_legs()),
            Some(&leaking()),
        );
        let second = match_plan(
            &PlayStyle::HighPress,
            Some(&a_squad_with_no_legs()),
            Some(&leaking()),
        );
        assert_eq!(first, second);
    }

    #[test]
    fn a_clubs_review_day_is_its_own_and_does_not_move() {
        assert_eq!(review_weekday("club-42"), review_weekday("club-42"));

        let mut days = std::collections::HashSet::new();
        for index in 0..500 {
            days.insert(review_weekday(&format!("club-{index}")));
        }
        assert_eq!(
            days.len(),
            REVIEW_CYCLE_DAYS as usize,
            "the review lands on {} of the seven days, so it is not spread across \
             the week at all",
            days.len()
        );
    }
}
