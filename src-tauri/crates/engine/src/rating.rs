//! Marks out of ten for a match, finished or as it stands.

use std::collections::HashMap;

use crate::report::{MatchReport, PlayerMatchStats};
use crate::types::{PlayerData, Position};

/// Below this a player has not been on long enough to be judged, and keeps the
/// unrated mark of `0.0` that downstream consumers skip.
pub const MIN_RATED_MINUTES: u8 = 10;

const BASE_RATING: f32 = 6.0;
const MIN_RATING: f32 = 1.0;
const MAX_RATING: f32 = 10.0;
const GOAL_BONUS: f32 = 0.6;
const ASSIST_BONUS: f32 = 0.5;
const YELLOW_PENALTY: f32 = 0.35;
const RED_PENALTY: f32 = 1.5;
const GOALKEEPER_CLEAN_SHEET_BONUS: f32 = 0.6;
const DEFENDER_CLEAN_SHEET_BONUS: f32 = 0.4;
/// A clean sheet is only his if he was there for most of it.
const MIN_CLEAN_SHEET_MINUTES: u8 = 60;

/// What a player's team did while he was on, which his own stats cannot say.
struct Appearance {
    result_bonus: f32,
    kept_clean_sheet: bool,
    position: Position,
}

impl MatchReport {
    /// Rates everyone who played. `home` and `away` must list every player of
    /// that side, including those substituted off, so each is judged by his
    /// own position and side.
    pub fn assign_ratings(&mut self, home: &[&PlayerData], away: &[&PlayerData]) {
        let home_result = match self.home_goals.cmp(&self.away_goals) {
            std::cmp::Ordering::Greater => 0.3,
            std::cmp::Ordering::Less => -0.3,
            std::cmp::Ordering::Equal => 0.0,
        };
        let sides = [
            (home, home_result, self.away_goals == 0),
            (away, -home_result, self.home_goals == 0),
        ];
        let appearances: HashMap<&str, Appearance> = sides
            .into_iter()
            .flat_map(|(players, result_bonus, kept_clean_sheet)| {
                players.iter().map(move |player| {
                    (
                        player.id.as_str(),
                        Appearance {
                            result_bonus,
                            kept_clean_sheet,
                            position: player.position,
                        },
                    )
                })
            })
            .collect();

        for (id, stats) in self.player_stats.iter_mut() {
            stats.rating = appearances
                .get(id.as_str())
                .map_or(0.0, |appearance| rate(stats, appearance));
        }
    }
}

/// Built from rates rather than raw totals, so the mark does not drift when the
/// engine's event volume changes.
fn rate(stats: &PlayerMatchStats, appearance: &Appearance) -> f32 {
    if stats.minutes_played < MIN_RATED_MINUTES {
        return 0.0;
    }
    let share = (stats.minutes_played as f32 / 90.0).clamp(0.15, 1.2);

    // A goal or a card counts the same whenever it happened, so these are not
    // scaled by minutes the way accumulating work is.
    let decisive = stats.goals as f32 * GOAL_BONUS + stats.assists as f32 * ASSIST_BONUS
        - card_penalty(stats)
        + clean_sheet_bonus(stats, appearance);

    let attacking = stats.shots_on_target as f32 * 0.12
        - stats.shots.saturating_sub(stats.shots_on_target) as f32 * 0.04;
    let defending = (stats.tackles_won + stats.interceptions) as f32 * 0.05;
    let work = attacking + defending + passing_score(stats, share)
        - stats.fouls_committed as f32 * 0.06
        - typical_work(appearance.position);

    (BASE_RATING + work * share.min(1.0) + decisive + appearance.result_bonus * share)
        .clamp(MIN_RATING, MAX_RATING)
}

/// What a player in this role does unprompted over a full match, measured with
/// the `rating_tests` distribution probe. The engine's actors are
/// position-specific (only forwards shoot, only midfielders assist), so without
/// this a forward averages a point above a defender for doing his job.
/// Re-measure when the engine's event mix changes.
fn typical_work(position: Position) -> f32 {
    match position {
        Position::Goalkeeper => 0.0,
        Position::Defender => 0.04,
        Position::Midfielder => 0.30,
        Position::Forward => 0.14,
    }
}

/// Accuracy against a competent baseline, weighted by how much he passed.
fn passing_score(stats: &PlayerMatchStats, share: f32) -> f32 {
    if stats.passes_attempted == 0 {
        return 0.0;
    }
    let accuracy = stats.passes_completed as f32 / stats.passes_attempted as f32;
    let involvement = (stats.passes_attempted as f32 / (18.0 * share)).clamp(0.0, 2.0);
    (accuracy - 0.78) * 3.0 * involvement
}

/// The engine counts a second yellow as a second `yellow_cards`, never as a
/// `red_cards`, and only a dismissal can produce two in one match.
fn card_penalty(stats: &PlayerMatchStats) -> f32 {
    let sent_off_on_a_second_yellow = stats.red_cards == 0 && stats.yellow_cards >= 2;
    if sent_off_on_a_second_yellow {
        RED_PENALTY
    } else {
        stats.yellow_cards as f32 * YELLOW_PENALTY + stats.red_cards as f32 * RED_PENALTY
    }
}

fn clean_sheet_bonus(stats: &PlayerMatchStats, appearance: &Appearance) -> f32 {
    if !appearance.kept_clean_sheet || stats.minutes_played < MIN_CLEAN_SHEET_MINUTES {
        return 0.0;
    }
    match appearance.position {
        Position::Goalkeeper => GOALKEEPER_CLEAN_SHEET_BONUS,
        Position::Defender => DEFENDER_CLEAN_SHEET_BONUS,
        Position::Midfielder | Position::Forward => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(minutes: u8) -> PlayerMatchStats {
        PlayerMatchStats {
            minutes_played: minutes,
            ..Default::default()
        }
    }

    fn appearance(position: Position) -> Appearance {
        Appearance {
            result_bonus: 0.0,
            kept_clean_sheet: false,
            position,
        }
    }

    fn mid(stats: &PlayerMatchStats) -> f32 {
        rate(stats, &appearance(Position::Midfielder))
    }

    /// Given a full match with nothing notable, When rated, Then it sits at the base.
    #[test]
    fn an_anonymous_full_match_rates_around_the_base() {
        let quiet_keeper = rate(&stats(90), &appearance(Position::Goalkeeper));
        assert!((5.9..=6.1).contains(&quiet_keeper));
    }

    /// Given a player below the minutes threshold, When rated, Then he stays unrated.
    #[test]
    fn a_player_below_the_minutes_threshold_is_unrated() {
        let mut cameo = stats(MIN_RATED_MINUTES - 1);
        cameo.goals = 1;
        assert_eq!(mid(&stats(0)), 0.0);
        assert_eq!(mid(&cameo), 0.0);
        assert!(mid(&stats(MIN_RATED_MINUTES)) > 0.0);
    }

    /// Given two goals, When rated, Then the mark is clearly above a quiet game.
    #[test]
    fn scoring_raises_the_mark() {
        let mut scorer = stats(90);
        scorer.goals = 2;
        assert!(mid(&scorer) > mid(&stats(90)) + 1.0);
    }

    /// Given an assist, When rated, Then it is worth less than a goal but more than nothing.
    #[test]
    fn an_assist_raises_the_mark_less_than_a_goal() {
        let mut assister = stats(90);
        assister.assists = 1;
        let mut scorer = stats(90);
        scorer.goals = 1;
        assert!(mid(&assister) > mid(&stats(90)));
        assert!(mid(&scorer) > mid(&assister));
    }

    /// Given a red card, When rated, Then the mark falls below five.
    #[test]
    fn a_sending_off_ruins_it() {
        let mut sent_off = stats(60);
        sent_off.red_cards = 1;
        sent_off.fouls_committed = 3;
        assert!(mid(&sent_off) < 5.0);
    }

    /// Given tidy and sloppy passers, When rated, Then accuracy decides.
    #[test]
    fn accurate_passing_beats_sloppy_passing() {
        let mut tidy = stats(90);
        tidy.passes_attempted = 60;
        tidy.passes_completed = 56;
        let mut wasteful = stats(90);
        wasteful.passes_attempted = 60;
        wasteful.passes_completed = 38;
        assert!(mid(&tidy) > mid(&wasteful));
    }

    /// Given the real events of a second-yellow dismissal and of a straight red, When both
    /// players are rated from the report, Then neither is rated above five.
    #[test]
    fn a_second_yellow_dismissal_costs_as_much_as_a_straight_red() {
        use crate::event::{EventType, MatchEvent};
        use crate::types::{Side, Zone};

        let event = |minute, event_type, player: &str| {
            MatchEvent::new(minute, event_type, Side::Home, Zone::Midfield).with_player(player)
        };
        let report = MatchReport::from_events(
            vec![
                event(20, EventType::YellowCard, "second-yellow"),
                event(55, EventType::SecondYellow, "second-yellow"),
                event(60, EventType::RedCard, "straight-red"),
            ],
            0,
            0,
            90,
        );
        let rate_player = |id: &str| {
            let mut stats = report.player_stats[id].clone();
            stats.minutes_played = 60;
            rate(&stats, &appearance(Position::Midfielder))
        };

        assert!(
            rate_player("second-yellow") < 5.0,
            "{}",
            rate_player("second-yellow")
        );
        assert!(rate_player("straight-red") < 5.0);
        assert!((rate_player("second-yellow") - rate_player("straight-red")).abs() < 0.4);
    }

    /// Given a single yellow card, When rated, Then it costs the yellow penalty only.
    #[test]
    fn one_yellow_costs_only_the_yellow_penalty() {
        let mut booked = stats(90);
        booked.yellow_cards = 1;
        let quiet = stats(90);
        assert!((mid(&quiet) - mid(&booked) - YELLOW_PENALTY).abs() < 1e-4);
    }

    /// The threshold is a game rule, so its value is pinned once here.
    #[test]
    fn a_player_needs_ten_minutes_to_be_rated() {
        assert_eq!(MIN_RATED_MINUTES, 10);
        assert_eq!(mid(&stats(9)), 0.0);
        assert!(mid(&stats(10)) > 0.0);
    }

    /// Given a short appearance, When rated, Then it stays near the base.
    #[test]
    fn a_short_appearance_stays_close_to_the_base() {
        let mut cameo = stats(MIN_RATED_MINUTES);
        cameo.passes_attempted = 4;
        cameo.passes_completed = 4;
        let rating = rate(
            &cameo,
            &Appearance {
                result_bonus: 0.3,
                ..appearance(Position::Midfielder)
            },
        );
        assert!((5.0..=7.5).contains(&rating), "{rating}");
    }

    /// Given a substitute and a starter scoring one goal each, When rated, Then the goal is worth the same.
    #[test]
    fn a_goal_is_worth_the_same_late_as_early() {
        let mut early = stats(90);
        early.goals = 1;
        let mut late = stats(MIN_RATED_MINUTES);
        late.goals = 1;
        let full_gap = mid(&early) - mid(&stats(90));
        let cameo_gap = mid(&late) - mid(&stats(MIN_RATED_MINUTES));
        assert!(
            (full_gap - cameo_gap).abs() < 0.01,
            "{full_gap} vs {cameo_gap}"
        );
    }

    /// Given a substitute sent off, When rated, Then he is punished as a starter would be.
    #[test]
    fn a_substitute_sent_off_is_punished_for_it() {
        let mut disgrace = stats(MIN_RATED_MINUTES);
        disgrace.red_cards = 1;
        assert!(mid(&disgrace) < mid(&stats(MIN_RATED_MINUTES)) - 1.0);
    }

    /// Given absurd extremes, When rated, Then the mark stays on the 1-10 scale.
    #[test]
    fn ratings_stay_inside_the_scale() {
        let mut heroic = stats(90);
        heroic.goals = 5;
        heroic.assists = 4;
        let mut dreadful = stats(90);
        dreadful.red_cards = 1;
        dreadful.yellow_cards = 2;
        dreadful.fouls_committed = 12;
        let won = Appearance {
            result_bonus: 0.3,
            ..appearance(Position::Forward)
        };
        let lost = Appearance {
            result_bonus: -0.3,
            ..appearance(Position::Defender)
        };
        assert_eq!(rate(&heroic, &won), MAX_RATING);
        let dreadful_rating = rate(&dreadful, &lost);
        assert!(
            (MIN_RATING..3.0).contains(&dreadful_rating),
            "{dreadful_rating}"
        );
    }

    /// Given a clean sheet, When rated, Then keepers and defenders gain, forwards do not.
    #[test]
    fn a_clean_sheet_lifts_keepers_and_defenders_only() {
        let kept = |position| {
            rate(
                &stats(90),
                &Appearance {
                    kept_clean_sheet: true,
                    ..appearance(position)
                },
            )
        };
        let open = |position| rate(&stats(90), &appearance(position));
        assert!(kept(Position::Goalkeeper) > open(Position::Goalkeeper) + 0.3);
        assert!(kept(Position::Defender) > open(Position::Defender) + 0.2);
        assert_eq!(kept(Position::Midfielder), open(Position::Midfielder));
        assert_eq!(kept(Position::Forward), open(Position::Forward));
    }

    /// Given a clean sheet shared for a few minutes, When rated, Then it is not credited.
    #[test]
    fn a_clean_sheet_needs_a_real_shift() {
        let rated = |minutes| {
            rate(
                &stats(minutes),
                &Appearance {
                    kept_clean_sheet: true,
                    ..appearance(Position::Goalkeeper)
                },
            ) - rate(&stats(minutes), &appearance(Position::Goalkeeper))
        };
        assert_eq!(rated(MIN_RATED_MINUTES), 0.0);
        assert!(rated(90) > 0.0);
    }
}
