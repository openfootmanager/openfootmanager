//! How good the world's players are, and how that moves from one season to the next.
//!
//! Measurement only (#869): nothing here judges the numbers. It reports, per season, what share of
//! senior players sit in the S and A tiers by current rating and by ceiling, so a change to how
//! players are generated, trained or retired can be read as a before and after.

use chrono::NaiveDate;
use domain::age::age_on_or_unknown;
use domain::player::Player;
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use ofm_core::player_rating::qualifies_for_wonderkid;
use ofm_core::player_tier::{PlayerTier, player_tier};

use crate::driver::SeasonObserver;

/// A player is senior from this age; the tier shares count seniors only, because a youth's
/// rating says little about the player he will be.
pub const SENIOR_MIN_AGE: i32 = 21;

/// The world's players on one day. Shares are fractions of senior players, except
/// `youth_share` and `wonderkid_share`, which are fractions of all players.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistributionReport {
    pub players: usize,
    pub seniors: usize,
    pub mean_senior_ovr: f64,
    pub senior_ovr_s_share: f64,
    pub senior_ovr_a_share: f64,
    pub senior_ceiling_s_share: f64,
    pub senior_ceiling_a_share: f64,
    pub youth_share: f64,
    /// Under the rule the game applies today (`player_rating::qualifies_for_wonderkid`).
    pub wonderkid_share: f64,
}

pub fn distribution_report(game: &Game) -> DistributionReport {
    distribution_of(&game.players, game.clock.current_date.date_naive())
}

fn distribution_of(players: &[Player], on: NaiveDate) -> DistributionReport {
    let mut seniors = 0usize;
    let mut ovr_sum = 0u64;
    let (mut ovr_s, mut ovr_a, mut ceiling_s, mut ceiling_a) = (0usize, 0usize, 0usize, 0usize);
    let mut wonderkids = 0usize;
    for player in players {
        let age = age_on_or_unknown(on, &player.date_of_birth);
        if qualifies_for_wonderkid(age.max(0) as u32, player.potential, player.ovr) {
            wonderkids += 1;
        }
        if age < SENIOR_MIN_AGE {
            continue;
        }
        seniors += 1;
        ovr_sum += u64::from(player.ovr);
        match player_tier(player.ovr) {
            PlayerTier::S => ovr_s += 1,
            PlayerTier::A => ovr_a += 1,
            PlayerTier::Rest => {}
        }
        match player_tier(player.potential) {
            PlayerTier::S => ceiling_s += 1,
            PlayerTier::A => ceiling_a += 1,
            PlayerTier::Rest => {}
        }
    }
    let share = |count: usize, of: usize| {
        if of == 0 {
            0.0
        } else {
            count as f64 / of as f64
        }
    };
    DistributionReport {
        players: players.len(),
        seniors,
        mean_senior_ovr: if seniors == 0 {
            0.0
        } else {
            ovr_sum as f64 / seniors as f64
        },
        senior_ovr_s_share: share(ovr_s, seniors),
        senior_ovr_a_share: share(ovr_a, seniors),
        senior_ceiling_s_share: share(ceiling_s, seniors),
        senior_ceiling_a_share: share(ceiling_a, seniors),
        youth_share: share(players.len() - seniors, players.len()),
        wonderkid_share: share(wonderkids, players.len()),
    }
}

/// The report after each season's rollover, in order.
#[derive(Default)]
pub struct DistributionLog {
    pub after_each_season: Vec<DistributionReport>,
}

impl SeasonObserver for DistributionLog {
    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {
        self.after_each_season.push(distribution_report(game));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{Player, PlayerAttributes, Position};

    fn on() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()
    }

    fn attributes() -> PlayerAttributes {
        PlayerAttributes {
            pace: 50,
            stamina: 50,
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
        }
    }

    fn player(born: &str, ovr: u8, potential: u8) -> Player {
        let mut player = Player::new(
            format!("p-{born}-{ovr}-{potential}"),
            "P".to_string(),
            "P P".to_string(),
            born.to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attributes(),
        );
        player.ovr = ovr;
        player.potential = potential;
        player
    }

    /// Given four seniors rated S, A, A and 60,
    /// When the world is reported,
    /// Then a quarter are S and half are A by rating.
    #[test]
    fn senior_ratings_are_counted_by_tier() {
        let players = [
            player("1995-01-01", 83, 83),
            player("1995-01-01", 78, 78),
            player("1995-01-01", 77, 77),
            player("1995-01-01", 60, 60),
        ];

        let report = distribution_of(&players, on());

        assert_eq!(report.seniors, 4);
        assert_eq!(report.senior_ovr_s_share, 0.25);
        assert_eq!(report.senior_ovr_a_share, 0.5);
    }

    /// Given a senior with a 60 rating and an S ceiling, and a senior with an A rating and an A ceiling,
    /// When the world is reported,
    /// Then the ceiling shares are read from potential, not from the rating.
    #[test]
    fn ceilings_are_tiered_apart_from_ratings() {
        let players = [player("1995-01-01", 60, 85), player("1995-01-01", 78, 78)];

        let report = distribution_of(&players, on());

        assert_eq!(report.senior_ovr_s_share, 0.0);
        assert_eq!(report.senior_ceiling_s_share, 0.5);
        assert_eq!(report.senior_ceiling_a_share, 0.5);
    }

    /// Given a youth of 20 with an S rating next to a senior of 21 with a 60,
    /// When the world is reported,
    /// Then only the senior is counted in the tier shares and the youth in `youth_share`.
    #[test]
    fn a_youth_is_not_counted_among_the_seniors() {
        let players = [player("2006-07-01", 85, 85), player("2005-07-01", 60, 60)];

        let report = distribution_of(&players, on());

        assert_eq!(report.seniors, 1);
        assert_eq!(report.senior_ovr_s_share, 0.0);
        assert_eq!(report.youth_share, 0.5);
    }

    /// Given seniors rated 60 and 70 and a youth rated 90,
    /// When the world is reported,
    /// Then the mean rating is that of the seniors alone.
    #[test]
    fn the_mean_rating_is_the_seniors() {
        let players = [
            player("1995-01-01", 60, 60),
            player("1995-01-01", 70, 70),
            player("2007-01-01", 90, 90),
        ];

        let report = distribution_of(&players, on());

        assert_eq!(report.mean_senior_ovr, 65.0);
    }

    /// Given a 19-year-old with a 95 ceiling and 30 points of room, and a 30-year-old,
    /// When the world is reported,
    /// Then half the players are wonderkids under today's rule.
    #[test]
    fn wonderkids_follow_the_rule_the_game_applies_today() {
        let players = [player("2007-01-01", 65, 95), player("1996-01-01", 65, 95)];

        let report = distribution_of(&players, on());

        assert_eq!(report.wonderkid_share, 0.5);
    }

    /// Given no players, when the world is reported, then every figure is zero rather than NaN.
    #[test]
    fn an_empty_world_reports_zeros() {
        let report = distribution_of(&[], on());

        assert_eq!(report.seniors, 0);
        assert_eq!(report.mean_senior_ovr, 0.0);
        assert_eq!(report.youth_share, 0.0);
    }
}
