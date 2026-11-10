//! National fixtures share nations and called-up players; club calendars reserve their dates too.

use super::INTERNATIONAL_FRIENDLY_COMPETITION_ID;
use crate::{
    calendar_booking::{MIN_GAP_DAYS, too_close},
    game::Game,
    knockout_dates::knockout_round_date,
};
use chrono::NaiveDate;
use domain::{
    league::{
        CompetitionType, Fixture, FixtureCompetition, FixtureStatus, Postponement, RescheduleReason,
    },
    national_team::NationalTeam,
};
use rand::Rng;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
std::thread_local! {
    static BOOKINGS_BUILT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static BOOKED_DATES: std::cell::RefCell<BTreeSet<NaiveDate>> = const { std::cell::RefCell::new(BTreeSet::new()) };
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Participant {
    Nation(String),
    Player(String),
}

struct Bookings<'a> {
    teams: &'a [NationalTeam],
    dates: BTreeMap<Participant, BTreeSet<NaiveDate>>,
}

impl<'a> Bookings<'a> {
    fn new(teams: &'a [NationalTeam]) -> Self {
        #[cfg(test)]
        BOOKINGS_BUILT.with(|count| count.set(count.get() + 1));
        Self {
            teams,
            dates: BTreeMap::new(),
        }
    }

    fn participants(&self, nation: &str) -> Vec<Participant> {
        std::iter::once(Participant::Nation(nation.to_string()))
            .chain(
                self.teams
                    .iter()
                    .filter(|team| team.id == nation)
                    .flat_map(|team| &team.squad_player_ids)
                    .cloned()
                    .map(Participant::Player),
            )
            .collect()
    }

    fn book(&mut self, home: &str, away: &str, date: NaiveDate) {
        #[cfg(test)]
        BOOKED_DATES.with(|dates| {
            dates.borrow_mut().insert(date);
        });
        for participant in self
            .participants(home)
            .into_iter()
            .chain(self.participants(away))
        {
            self.dates.entry(participant).or_default().insert(date);
        }
    }

    fn book_player(&mut self, player_id: &str, date: NaiveDate) {
        #[cfg(test)]
        BOOKED_DATES.with(|dates| {
            dates.borrow_mut().insert(date);
        });
        self.dates
            .entry(Participant::Player(player_id.to_string()))
            .or_default()
            .insert(date);
    }

    fn blocked(&self, home: &str, away: &str) -> BTreeSet<NaiveDate> {
        self.participants(home)
            .into_iter()
            .chain(self.participants(away))
            .filter_map(|participant| self.dates.get(&participant))
            .flat_map(|dates| dates.iter().copied())
            .collect()
    }

    fn is_free(&self, nation: &str, date: NaiveDate) -> bool {
        !self
            .blocked(nation, nation)
            .iter()
            .any(|other| too_close(date, *other))
    }
}

fn share_players(home: &NationalTeam, away: &NationalTeam) -> bool {
    home.squad_player_ids
        .iter()
        .any(|id| away.squad_player_ids.contains(id))
}

pub(super) fn draw_friendlies(
    teams: &[NationalTeam],
    window_dates: &[String],
    rng: &mut impl Rng,
) -> Vec<(usize, Fixture)> {
    let mut bookings = Bookings::new(teams);
    for fixture in teams.iter().flat_map(|team| &team.fixtures) {
        if let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") {
            bookings.book(&fixture.home_team_id, &fixture.away_team_id, date);
        }
        // A postponed match still fills its original window; it must never be drawn twice.
        if let Some(original) = fixture
            .postponement
            .as_ref()
            .and_then(|p| NaiveDate::parse_from_str(&p.original_date, "%Y-%m-%d").ok())
        {
            bookings.book(&fixture.home_team_id, &fixture.away_team_id, original);
        }
    }
    let mut drawn = Vec::new();
    for (window, date_text) in window_dates.iter().enumerate() {
        let Ok(date) = NaiveDate::parse_from_str(date_text, "%Y-%m-%d") else {
            continue;
        };
        let mut eligible: Vec<_> = teams
            .iter()
            .enumerate()
            .filter(|(_, team)| {
                !team.squad_player_ids.is_empty() && bookings.is_free(&team.id, date)
            })
            .map(|(index, _)| index)
            .collect();
        eligible.sort_by(|a, b| teams[*a].id.cmp(&teams[*b].id));
        super::shuffle(&mut eligible, rng);
        while let Some(home_index) = eligible.pop() {
            let home = &teams[home_index];
            if !bookings.is_free(&home.id, date) {
                continue;
            }
            let Some(opponent) = eligible.iter().position(|index| {
                bookings.is_free(&teams[*index].id, date) && !share_players(home, &teams[*index])
            }) else {
                continue;
            };
            let away = &teams[eligible.remove(opponent)];
            drawn.push((
                home_index,
                Fixture {
                    id: format!("ntf-{date_text}-{}-{}", home.id, away.id),
                    competition_id: INTERNATIONAL_FRIENDLY_COMPETITION_ID.into(),
                    matchday: window as u32 + 1,
                    date: date_text.clone(),
                    home_team_id: home.id.clone(),
                    away_team_id: away.id.clone(),
                    competition: FixtureCompetition::InternationalNation,
                    ..Default::default()
                },
            ));
            bookings.book(&home.id, &away.id, date);
        }
    }
    drawn
}

fn tournament_bookings<'a>(game: &'a Game) -> Bookings<'a> {
    let mut bookings = Bookings::new(&game.national_teams);
    for competition in game
        .competitions
        .iter()
        .filter(|c| c.kind == CompetitionType::InternationalNation)
    {
        for fixture in &competition.fixtures {
            if let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") {
                bookings.book(&fixture.home_team_id, &fixture.away_team_id, date);
            }
        }
        // Reserve the finals' undrawn rounds too; a friendly must not claim their future dates.
        if let Some((first, last)) = crate::world_cup::finals_period(competition) {
            for offset in 0..=(last - first).num_days() {
                for nation in &competition.participant_ids {
                    bookings.book(nation, nation, first + chrono::Duration::days(offset));
                }
            }
        }
    }
    for fixture in game
        .national_teams
        .iter()
        .flat_map(|team| &team.fixtures)
        .filter(|fixture| fixture.status != FixtureStatus::Scheduled)
    {
        if let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") {
            bookings.book(&fixture.home_team_id, &fixture.away_team_id, date);
        }
    }
    bookings
}

/// Club fixtures are immutable once played. Without full XI snapshots here, reserve the
/// called-up clubmates conservatively, including clubs from recent recorded moves. Unplayed
/// club fixtures still yield to international duty.
fn book_completed_club_appearances(game: &Game, first_day: NaiveDate, bookings: &mut Bookings<'_>) {
    let horizon = first_day
        .checked_sub_days(chrono::Days::new(MIN_GAP_DAYS as u64))
        .unwrap_or(first_day);
    let since = horizon.to_string();
    let through = first_day.to_string();
    let called_up: BTreeSet<_> = game
        .national_teams
        .iter()
        .flat_map(|nation| nation.squad_player_ids.iter().map(String::as_str))
        .collect();
    let mut players_by_club: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for player in game
        .players
        .iter()
        .filter(|player| called_up.contains(player.id.as_str()))
    {
        let clubs: BTreeSet<_> = player
            .team_id
            .as_deref()
            .into_iter()
            .chain(
                player
                    .movement_history
                    .iter()
                    .filter(|movement| {
                        movement.date.as_str() >= since.as_str()
                            && movement.date.as_str() <= through.as_str()
                    })
                    .flat_map(|movement| {
                        [
                            movement.from_team_id.as_deref(),
                            movement.to_team_id.as_deref(),
                        ]
                        .into_iter()
                        .flatten()
                    }),
            )
            .collect();
        for club in clubs {
            players_by_club.entry(club).or_default().push(&player.id);
        }
    }
    for fixture in game
        .competitions
        .iter()
        .filter(|competition| competition.kind != CompetitionType::InternationalNation)
        .flat_map(|competition| &competition.fixtures)
        .filter(|fixture| fixture.status == FixtureStatus::Completed)
    {
        let Ok(date) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") else {
            continue;
        };
        if date < horizon {
            continue;
        }
        for club in [&fixture.home_team_id, &fixture.away_team_id] {
            for player in players_by_club.get(club.as_str()).into_iter().flatten() {
                bookings.book_player(player, date);
            }
        }
    }
}

/// A friendly gives way to a tournament or an earlier published friendly, retaining its identity.
/// Planning is read-only; applying it afterwards avoids invalidating the bookings while settling.
pub(crate) fn settle_friendlies(game: &mut Game, first_day: NaiveDate) {
    let moves = friendly_moves(game, first_day);
    for (team, fixture, date) in moves {
        let fixture = &mut game.national_teams[team].fixtures[fixture];
        if fixture.postponement.is_none() {
            fixture.postponement = Some(Postponement {
                original_date: fixture.date.clone(),
                reason: RescheduleReason::InternationalWindow,
            });
        }
        fixture.date = date.to_string();
    }
}

fn friendly_moves(game: &Game, first_day: NaiveDate) -> Vec<(usize, usize, NaiveDate)> {
    let mut bookings = tournament_bookings(game);
    book_completed_club_appearances(game, first_day, &mut bookings);
    let mut scheduled: Vec<_> = game
        .national_teams
        .iter()
        .enumerate()
        .flat_map(|(team, nation)| {
            nation
                .fixtures
                .iter()
                .enumerate()
                .filter(|(_, fixture)| fixture.status == FixtureStatus::Scheduled)
                .map(move |(index, fixture)| (team, index, fixture))
        })
        .collect();
    scheduled.sort_by_key(|(_, _, fixture)| (&fixture.date, &fixture.id));
    let mut moves = Vec::new();
    for (team, index, fixture) in scheduled {
        let Ok(original) = NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") else {
            continue;
        };
        let blocked = bookings.blocked(&fixture.home_team_id, &fixture.away_team_id);
        let Some(date) =
            knockout_round_date(original.max(first_day), None, 0, &BTreeSet::new(), &blocked)
        else {
            log::error!(
                "national friendly {} has no legal date inside the calendar search",
                fixture.id
            );
            continue;
        };
        bookings.book(&fixture.home_team_id, &fixture.away_team_id, date);
        if date != original {
            moves.push((team, index, date));
        }
    }
    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::league::{CompetitionScope, League, MatchResult};
    use domain::manager::Manager;

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2031, 6, day).unwrap()
    }

    fn fixture(id: &str, date: &str, status: FixtureStatus) -> Fixture {
        let result = (status == FixtureStatus::Completed).then(MatchResult::default);
        Fixture {
            id: id.into(),
            date: date.into(),
            home_team_id: "nt-a".into(),
            away_team_id: "nt-b".into(),
            competition_id: INTERNATIONAL_FRIENDLY_COMPETITION_ID.into(),
            competition: FixtureCompetition::InternationalNation,
            status,
            result,
            ..Default::default()
        }
    }

    fn game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 6, 10, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "manager".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        for id in ["nt-a", "nt-b"] {
            let mut nation = NationalTeam::new(id.into(), id.into(), id.into(), None);
            nation.squad_player_ids.push(format!("player-{id}"));
            game.national_teams.push(nation);
        }
        game.national_teams[0].fixtures.push(fixture(
            "historical",
            "2020-01-01",
            FixtureStatus::Completed,
        ));
        game
    }

    /// Given retained historical friendlies and no Scheduled friendly,
    /// When a live match day finishes, then no national booking map is constructed.
    #[test]
    fn a_day_without_scheduled_friendlies_builds_no_bookings() {
        let mut game = game();
        BOOKINGS_BUILT.with(|count| count.set(0));
        drop(Bookings::new(&game.national_teams));
        assert_eq!(
            BOOKINGS_BUILT.with(|count| count.get()),
            1,
            "the counter observes a real booking construction"
        );
        BOOKINGS_BUILT.with(|count| count.set(0));
        crate::turn::finish_live_match_day(&mut game);
        assert_eq!(
            BOOKINGS_BUILT.with(|count| count.get()),
            0,
            "no Scheduled friendly needs bookings"
        );
        assert_eq!(game.national_teams[0].fixtures.len(), 1);
    }

    /// Given an upcoming friendly and historical national fixtures before and at the rest horizon,
    /// When it is settled, then only horizon-or-newer dates are booked and all history is retained.
    #[test]
    fn settlement_books_only_history_that_can_still_affect_rest() {
        let mut game = game();
        game.national_teams[0].fixtures.extend([
            fixture("horizon", "2031-06-08", FixtureStatus::Completed),
            fixture("recent", "2031-06-10", FixtureStatus::Completed),
            fixture("future", "2031-06-20", FixtureStatus::Scheduled),
        ]);
        let mut cup = League::new("continental".into(), "Continental".into(), 2019, &[]);
        cup.kind = CompetitionType::InternationalNation;
        cup.scope = CompetitionScope::Continental;
        cup.fixtures.push(fixture(
            "old-tournament",
            "2019-06-01",
            FixtureStatus::Completed,
        ));
        game.competitions.push(cup);
        BOOKED_DATES.with(|dates| dates.borrow_mut().clear());
        settle_friendlies(&mut game, day(11));
        let dates = BOOKED_DATES.with(|dates| dates.borrow().clone());
        assert!(dates.contains(&day(8)), "the horizon boundary is retained");
        assert!(
            dates.contains(&day(10)),
            "recent history still reserves rest"
        );
        assert!(
            dates.contains(&day(20)),
            "the upcoming friendly is actually settled"
        );
        assert!(
            !dates.contains(&NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            "old friendly history must not build bookings"
        );
        assert!(
            !dates.contains(&NaiveDate::from_ymd_opt(2019, 6, 1).unwrap()),
            "old tournament history must not build bookings"
        );
        assert_eq!(game.national_teams[0].fixtures.len(), 4);
        assert_eq!(game.competitions[0].fixtures.len(), 1);
    }
}
