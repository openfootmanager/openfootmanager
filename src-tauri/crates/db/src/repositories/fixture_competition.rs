//! How a `FixtureCompetition` is stored as text, shared by every table that holds one.
//!
//! `fixtures` and the match-stats tables used to carry their own copies of this mapping, and they
//! drifted: the `fixtures` one only knew League, Friendly and PreseasonTournament, so every cup,
//! continental and international fixture saved correctly and loaded back as a league fixture.

use domain::league::FixtureCompetition;

pub(crate) fn fixture_competition_to_string(competition: &FixtureCompetition) -> String {
    match competition {
        FixtureCompetition::League => "League".to_string(),
        FixtureCompetition::Cup => "Cup".to_string(),
        FixtureCompetition::ContinentalClub => "ContinentalClub".to_string(),
        FixtureCompetition::InternationalClub => "InternationalClub".to_string(),
        FixtureCompetition::InternationalNation => "InternationalNation".to_string(),
        FixtureCompetition::Friendly => "Friendly".to_string(),
        FixtureCompetition::FriendlyCup => "FriendlyCup".to_string(),
        FixtureCompetition::PreseasonTournament => "PreseasonTournament".to_string(),
    }
}

pub(crate) fn parse_fixture_competition(value: &str) -> FixtureCompetition {
    match value {
        "Cup" => FixtureCompetition::Cup,
        "ContinentalClub" => FixtureCompetition::ContinentalClub,
        "InternationalClub" => FixtureCompetition::InternationalClub,
        "InternationalNation" => FixtureCompetition::InternationalNation,
        "Friendly" => FixtureCompetition::Friendly,
        "FriendlyCup" => FixtureCompetition::FriendlyCup,
        "PreseasonTournament" => FixtureCompetition::PreseasonTournament,
        _ => FixtureCompetition::League,
    }
}

/// Every kind of fixture, once. The `match` has no wildcard on purpose: a new variant stops
/// this compiling until it is listed here, and so until its save/load is tested.
#[cfg(test)]
pub(crate) fn every_fixture_competition() -> [(&'static str, FixtureCompetition); 8] {
    use FixtureCompetition::*;
    match League {
        League | Cup | ContinentalClub | InternationalClub | InternationalNation | Friendly
        | FriendlyCup | PreseasonTournament => {}
    }
    [
        ("League", League),
        ("Cup", Cup),
        ("ContinentalClub", ContinentalClub),
        ("InternationalClub", InternationalClub),
        ("InternationalNation", InternationalNation),
        ("Friendly", Friendly),
        ("FriendlyCup", FriendlyCup),
        ("PreseasonTournament", PreseasonTournament),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_competitions_are_stored_by_name() {
        crate::stored_text::assert_stored_as(
            &every_fixture_competition(),
            fixture_competition_to_string,
            parse_fixture_competition,
        );
    }
}
