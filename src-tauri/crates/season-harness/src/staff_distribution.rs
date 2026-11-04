//! How good the world's staff are, role by role, and how that moves from one season to the next.
//!
//! Measurement only: nothing here judges the numbers. Each role's ratings
//! (`ofm_core::staff_rating`) are kept as a histogram, at clubs and on the market apart, so the
//! per-role tier cut-offs (the top 2.5%, 11.5% and 51.5% of a role) can be read off any season.
//! The retired players the game turns into scouts are counted too: they join the scouting pool
//! every summer, with judging taken from their own attributes rather than the staff bell.

use domain::staff::{Staff, StaffRole};
use ofm_core::end_of_season::{EndOfSeasonSummary, RETIRED_PLAYER_SCOUT_ID_PREFIX};
use ofm_core::game::Game;
use ofm_core::staff_rating::staff_rating;

use crate::driver::SeasonObserver;

/// How many staff hold each rating from 0 to 100.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatingHistogram {
    counts: [usize; 101],
}

impl Default for RatingHistogram {
    fn default() -> Self {
        Self { counts: [0; 101] }
    }
}

impl RatingHistogram {
    fn add(&mut self, rating: u8) {
        self.counts[usize::from(rating.min(100))] += 1;
    }

    pub fn count(&self) -> usize {
        self.counts.iter().sum()
    }

    pub fn mean(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            return 0.0;
        }
        let total: usize = self
            .counts
            .iter()
            .enumerate()
            .map(|(rating, many)| rating * many)
            .sum();
        total as f64 / count as f64
    }

    pub fn sd(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            return 0.0;
        }
        let mean = self.mean();
        let squares: f64 = self
            .counts
            .iter()
            .enumerate()
            .map(|(rating, many)| (rating as f64 - mean).powi(2) * *many as f64)
            .sum();
        (squares / count as f64).sqrt()
    }

    /// The lowest rating among the top `share` of staff, by nearest rank: the rating of the
    /// `ceil(count × share)`-th best. `None` for no staff.
    pub fn lowest_rating_of_the_top(&self, share: f64) -> Option<u8> {
        let count = self.count();
        if count == 0 {
            return None;
        }
        let rank = ((count as f64 * share).ceil() as usize).max(1);
        let mut seen = 0usize;
        for rating in (0..=100u8).rev() {
            seen += self.counts[usize::from(rating)];
            if seen >= rank {
                return Some(rating);
            }
        }
        Some(0)
    }

    /// The share of staff rated `rating` or more.
    pub fn share_at_or_above(&self, rating: u8) -> f64 {
        let count = self.count();
        if count == 0 {
            return 0.0;
        }
        let above: usize = self.counts[usize::from(rating.min(100))..].iter().sum();
        above as f64 / count as f64
    }
}

/// One role's staff on one day.
#[derive(Debug, Clone, PartialEq)]
pub struct RoleRatings {
    pub role: StaffRole,
    pub at_clubs: RatingHistogram,
    pub free_agents: RatingHistogram,
    /// Retired players turned scouts who are at a club (always zero for any role but Scout).
    pub retired_player_scouts_at_clubs: usize,
}

/// The world's staff on one day, one entry per role in `StaffRole::ALL`'s order.
#[derive(Debug, Clone, PartialEq)]
pub struct StaffDistributionReport {
    pub roles: Vec<RoleRatings>,
}

impl StaffDistributionReport {
    pub fn role(&self, role: &StaffRole) -> Option<&RoleRatings> {
        self.roles.iter().find(|ratings| &ratings.role == role)
    }
}

/// The report for every staff member in `game`.
pub fn staff_distribution_report(game: &Game) -> StaffDistributionReport {
    staff_distribution_of(&game.staff)
}

fn staff_distribution_of(staff: &[Staff]) -> StaffDistributionReport {
    StaffDistributionReport {
        roles: StaffRole::ALL
            .into_iter()
            .map(|role| {
                let mut ratings = RoleRatings {
                    role: role.clone(),
                    at_clubs: RatingHistogram::default(),
                    free_agents: RatingHistogram::default(),
                    retired_player_scouts_at_clubs: 0,
                };
                for member in staff.iter().filter(|member| member.role == role) {
                    if member.team_id.is_some() {
                        ratings.at_clubs.add(staff_rating(member));
                        if member.role == StaffRole::Scout
                            && member.id.starts_with(RETIRED_PLAYER_SCOUT_ID_PREFIX)
                        {
                            ratings.retired_player_scouts_at_clubs += 1;
                        }
                    } else {
                        ratings.free_agents.add(staff_rating(member));
                    }
                }
                ratings
            })
            .collect(),
    }
}

/// The report after each season's rollover, in order.
#[derive(Default)]
pub struct StaffDistributionLog {
    pub after_each_season: Vec<StaffDistributionReport>,
}

impl SeasonObserver for StaffDistributionLog {
    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {
        self.after_each_season.push(staff_distribution_report(game));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::staff::StaffAttributes;

    /// A member of `role` whose rating is `rating`: every attribute at it, so each role's rule
    /// (one attribute, a mean or a weighted blend) reads the same number.
    fn member(id: &str, role: StaffRole, rating: u8, club: Option<&str>) -> Staff {
        let mut staff = Staff::new(
            id.to_string(),
            "First".to_string(),
            "Last".to_string(),
            "1980-01-01".to_string(),
            role,
            StaffAttributes {
                coaching: rating,
                judging_ability: rating,
                judging_potential: rating,
                physiotherapy: rating,
            },
        );
        staff.team_id = club.map(str::to_string);
        staff
    }

    /// Given two coaches at clubs rated 70 and 80, a coach on the market rated 50, and a physio
    /// When the staff are reported
    /// Then the coaches at clubs average 75, the market's coach is counted apart, and the physio
    ///      is in his own role's histogram, not the coaches'.
    #[test]
    fn staff_are_counted_by_role_at_clubs_and_on_the_market_apart() {
        let staff = [
            member("a", StaffRole::Coach, 70, Some("club")),
            member("b", StaffRole::Coach, 80, Some("club")),
            member("c", StaffRole::Coach, 50, None),
            member("d", StaffRole::Physio, 90, Some("club")),
        ];

        let report = staff_distribution_of(&staff);

        let coaches = report
            .role(&StaffRole::Coach)
            .expect("coaches are reported");
        assert_eq!(coaches.at_clubs.count(), 2);
        assert_eq!(coaches.at_clubs.mean(), 75.0);
        assert_eq!(coaches.at_clubs.sd(), 5.0);
        assert_eq!(coaches.free_agents.count(), 1);
        assert_eq!(coaches.free_agents.mean(), 50.0);
        let physios = report
            .role(&StaffRole::Physio)
            .expect("physios are reported");
        assert_eq!(physios.at_clubs.count(), 1);
        assert_eq!(physios.at_clubs.mean(), 90.0);
    }

    /// Given forty scouts at clubs rated 61 to 100, one each
    /// When the lowest rating of the top 2.5%, 11.5% and 51.5% is read
    /// Then it is the 1st best (100), the 5th best (96, as 11.5% of 40 is 4.6) and the 21st best
    ///      (80, as 51.5% of 40 is 20.6): nearest rank, rounding the count up.
    #[test]
    fn the_cut_off_of_a_top_share_is_its_nearest_rank() {
        let staff: Vec<Staff> = (61..=100u8)
            .map(|rating| {
                member(
                    &format!("s{rating}"),
                    StaffRole::Scout,
                    rating,
                    Some("club"),
                )
            })
            .collect();

        let scouts = staff_distribution_of(&staff)
            .role(&StaffRole::Scout)
            .expect("scouts are reported")
            .at_clubs
            .clone();

        assert_eq!(scouts.lowest_rating_of_the_top(0.025), Some(100));
        assert_eq!(scouts.lowest_rating_of_the_top(0.115), Some(96));
        assert_eq!(scouts.lowest_rating_of_the_top(0.515), Some(80));
        assert_eq!(scouts.share_at_or_above(96), 0.125);
    }

    /// Given a scout the end of season made from a retired player, at a club, another on the
    /// market, and a scout generated for a club
    /// When the staff are reported
    /// Then one retired-player scout is counted at clubs, and all three are in the scouts' ratings.
    #[test]
    fn retired_players_turned_scouts_are_counted_at_clubs() {
        let staff = [
            member("staff_retired_scout_p1", StaffRole::Scout, 70, Some("club")),
            member("staff_retired_scout_p2", StaffRole::Scout, 60, None),
            member("generated", StaffRole::Scout, 80, Some("club")),
        ];

        let scouts = staff_distribution_of(&staff)
            .role(&StaffRole::Scout)
            .expect("scouts are reported")
            .clone();

        assert_eq!(scouts.retired_player_scouts_at_clubs, 1);
        assert_eq!(scouts.at_clubs.count(), 2);
        assert_eq!(scouts.free_agents.count(), 1);
    }

    /// Given a coach at a club whose id carries the retired-player scout prefix
    /// When the staff are reported
    /// Then he is not counted as a retired-player scout, among the coaches or anywhere: only a
    ///      scout is one, whatever an id says.
    #[test]
    fn only_a_scout_counts_as_a_retired_player_scout() {
        let staff = [member(
            "staff_retired_scout_p9",
            StaffRole::Coach,
            70,
            Some("club"),
        )];

        let report = staff_distribution_of(&staff);

        for role in &report.roles {
            assert_eq!(role.retired_player_scouts_at_clubs, 0, "{:?}", role.role);
        }
    }

    /// Given no staff at all
    /// When the staff are reported
    /// Then every role is there, empty, with no cut-off and zero shares.
    #[test]
    fn an_empty_world_reports_every_role_empty() {
        let report = staff_distribution_of(&[]);

        assert_eq!(report.roles.len(), 4);
        for role in &report.roles {
            assert_eq!(role.at_clubs.count(), 0);
            assert_eq!(role.at_clubs.lowest_rating_of_the_top(0.025), None);
            assert_eq!(role.at_clubs.share_at_or_above(50), 0.0);
        }
    }
}
