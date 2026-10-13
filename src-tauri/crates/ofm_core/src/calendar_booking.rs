//! The world rest rule and the occupancy it is checked against.
//!
//! Two fixtures of one club must be at least [`MIN_GAP_DAYS`] dates apart, whatever competition
//! they belong to. Higher [`Layer`]s keep their dates; lower ones give way.

use chrono::{Days, NaiveDate};
use domain::league::FixtureCompetition;
use std::collections::BTreeMap;

/// Minimum rest: Saturday then Tuesday is legal, Saturday then Monday is not.
pub const MIN_GAP_DAYS: i64 = 3;

/// Whether two dates are too close for one club to play on both.
pub fn too_close(first: NaiveDate, second: NaiveDate) -> bool {
    (first - second).num_days().abs() < MIN_GAP_DAYS
}

/// Who gives way when two bookings clash: a lower layer yields to a higher one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Friendly,
    League,
    DomesticCup,
    Continental,
    International,
}

impl Layer {
    pub fn of(competition: &FixtureCompetition) -> Self {
        match competition {
            FixtureCompetition::Friendly
            | FixtureCompetition::FriendlyCup
            | FixtureCompetition::PreseasonTournament => Self::Friendly,
            FixtureCompetition::League => Self::League,
            FixtureCompetition::Cup => Self::DomesticCup,
            FixtureCompetition::ContinentalClub | FixtureCompetition::InternationalClub => {
                Self::Continental
            }
            FixtureCompetition::InternationalNation => Self::International,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Booking<'a> {
    pub competition_id: &'a str,
    pub fixture_id: &'a str,
    pub layer: Layer,
}

type DayBookings<'a> = BTreeMap<NaiveDate, Vec<Booking<'a>>>;

/// Every date each club plays or may still play. Firm bookings are known fixtures. Shadow
/// bookings are dates a club could still be drawn on: they keep a lower layer's fixture from
/// being moved onto a date a later draw will claim, and constrain nothing else.
#[derive(Debug, Default)]
pub struct Occupancy<'a> {
    firm: BTreeMap<&'a str, DayBookings<'a>>,
    shadow: BTreeMap<&'a str, DayBookings<'a>>,
}

fn nearby<'b, 'a>(
    days: &'b DayBookings<'a>,
    date: NaiveDate,
) -> impl Iterator<Item = (NaiveDate, &'b Booking<'a>)> + 'b {
    let reach = Days::new((MIN_GAP_DAYS - 1) as u64);
    let from = date.checked_sub_days(reach).unwrap_or(date);
    let to = date.checked_add_days(reach).unwrap_or(date);
    days.range(from..=to)
        .flat_map(|(booked_on, bookings)| bookings.iter().map(move |b| (*booked_on, b)))
}

impl<'a> Occupancy<'a> {
    pub fn book_firm(&mut self, club_id: &'a str, booking: Booking<'a>, date: NaiveDate) {
        self.firm
            .entry(club_id)
            .or_default()
            .entry(date)
            .or_default()
            .push(booking);
    }

    pub fn book_shadow(&mut self, club_id: &'a str, booking: Booking<'a>, date: NaiveDate) {
        self.shadow
            .entry(club_id)
            .or_default()
            .entry(date)
            .or_default()
            .push(booking);
    }

    /// True when `club_id` has no firm booking within the rest window of `date`.
    pub fn is_free(&self, club_id: &str, date: NaiveDate) -> bool {
        self.firm
            .get(club_id)
            .is_none_or(|days| nearby(days, date).next().is_none())
    }

    /// True when `club_id` has neither a firm nor a shadow booking within the rest window.
    pub fn is_free_of_shadows(&self, club_id: &str, date: NaiveDate) -> bool {
        self.is_free(club_id, date)
            && self
                .shadow
                .get(club_id)
                .is_none_or(|days| nearby(days, date).next().is_none())
    }

    /// The firm bookings of `club_id` within the rest window of `date`.
    pub fn firm_clashes<'b>(
        &'b self,
        club_id: &str,
        date: NaiveDate,
    ) -> impl Iterator<Item = (NaiveDate, &'b Booking<'a>)> + 'b {
        self.firm
            .get(club_id)
            .into_iter()
            .flat_map(move |days| nearby(days, date))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn booking(layer: Layer) -> Booking<'static> {
        Booking {
            competition_id: "c",
            fixture_id: "f",
            layer,
        }
    }

    fn booked(club: &'static str, date: NaiveDate) -> Occupancy<'static> {
        let mut occupancy = Occupancy::default();
        occupancy.book_firm(club, booking(Layer::League), date);
        occupancy
    }

    /// Given a match on Saturday, when Sunday to Monday are checked, then they are too close; Tuesday is legal.
    #[test]
    fn the_minimum_rest_is_three_days() {
        let saturday = day(2033, 3, 5);
        let occupancy = booked("madrid", saturday);
        for offset in 0..=2 {
            let date = saturday + Days::new(offset);
            assert!(!occupancy.is_free("madrid", date), "{date} is too close");
            assert!(too_close(saturday, date));
        }
        let tuesday = saturday + Days::new(3);
        assert!(occupancy.is_free("madrid", tuesday));
        assert!(!too_close(saturday, tuesday));
    }

    /// Given a match, when the days before it are checked, then the rule is symmetric.
    #[test]
    fn the_rule_looks_backwards_as_well_as_forwards() {
        let occupancy = booked("madrid", day(2033, 3, 8));
        assert!(!occupancy.is_free("madrid", day(2033, 3, 6)));
        assert!(occupancy.is_free("madrid", day(2033, 3, 5)));
    }

    /// Given a booking on December 31, when January 1 and 2 are checked, then both are too close and January 3 is legal.
    #[test]
    fn the_rule_crosses_the_new_year() {
        let occupancy = booked("ajax", day(2033, 12, 31));
        assert!(!occupancy.is_free("ajax", day(2034, 1, 1)));
        assert!(!occupancy.is_free("ajax", day(2034, 1, 2)));
        assert!(occupancy.is_free("ajax", day(2034, 1, 3)));
    }

    /// Given a booking of one club, when another club is checked on the same day, then it is free.
    #[test]
    fn another_club_is_unaffected() {
        assert!(booked("madrid", day(2033, 3, 5)).is_free("betis", day(2033, 3, 5)));
    }

    /// Given a shadow booking, when a lower layer looks for a target, then the date is not free of shadows
    /// but a cup's own firm check still passes.
    #[test]
    fn a_shadow_blocks_targets_but_not_firm_placement() {
        let mut occupancy = Occupancy::default();
        let date = day(2033, 3, 8);
        occupancy.book_shadow("madrid", booking(Layer::DomesticCup), date);
        assert!(occupancy.is_free("madrid", date));
        assert!(!occupancy.is_free_of_shadows("madrid", date));
    }

    /// Given each fixture kind, when its layer is taken, then the order is friendly < league < cup < continental < international.
    #[test]
    fn layers_follow_the_agreed_order() {
        use FixtureCompetition as K;
        let ordered = [
            K::Friendly,
            K::League,
            K::Cup,
            K::ContinentalClub,
            K::InternationalNation,
        ];
        assert!(
            ordered
                .windows(2)
                .all(|w| Layer::of(&w[0]) < Layer::of(&w[1]))
        );
        assert_eq!(Layer::of(&K::FriendlyCup), Layer::Friendly);
        assert_eq!(Layer::of(&K::PreseasonTournament), Layer::Friendly);
        assert_eq!(
            Layer::of(&K::InternationalClub),
            Layer::of(&K::ContinentalClub)
        );
    }

    /// Given firm bookings, when clashes are listed, then they name the booked layer and date.
    #[test]
    fn clashes_name_the_booking_and_its_date() {
        let occupancy = booked("madrid", day(2033, 3, 5));
        let clashes: Vec<_> = occupancy.firm_clashes("madrid", day(2033, 3, 6)).collect();
        assert_eq!(clashes.len(), 1);
        assert_eq!(clashes[0].0, day(2033, 3, 5));
        assert_eq!(clashes[0].1.layer, Layer::League);
    }
}
