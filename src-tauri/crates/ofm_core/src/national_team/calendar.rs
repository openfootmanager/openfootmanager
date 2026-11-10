//! National fixtures share nations and called-up players; club calendars reserve their dates too.

use super::INTERNATIONAL_FRIENDLY_COMPETITION_ID;
use crate::{calendar_booking::too_close, game::Game, knockout_dates::knockout_round_date};
use chrono::NaiveDate;
use domain::{
    league::{
        CompetitionType, Fixture, FixtureCompetition, FixtureStatus, Postponement, RescheduleReason,
    },
    national_team::NationalTeam,
};
use rand::Rng;
use std::collections::{BTreeMap, BTreeSet};

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
        for participant in self
            .participants(home)
            .into_iter()
            .chain(self.participants(away))
        {
            self.dates.entry(participant).or_default().insert(date);
        }
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
