use chrono::{DateTime, Duration, Utc};
use domain::league::{
    CompetitionFormat, CompetitionRules, CompetitionScope, CompetitionType, Fixture,
    FixtureCompetition, FixtureStatus, KnockoutRoundState, League,
};
use uuid::Uuid;

/// Generate a full double round-robin schedule (home & away) for the given teams.
/// Matchdays are spaced 7 days apart starting from `start_date`.
/// Uses a rotation-based algorithm for balanced scheduling.
pub fn generate_league(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
) -> League {
    let n = team_ids.len();
    assert!(n >= 2);

    let league_id = Uuid::new_v4().to_string();
    let mut league = League::new(league_id.clone(), name.to_string(), season, team_ids);

    // For round-robin with n teams (n must be even; if odd, add a "bye" — we assume even here)
    // Number of rounds in a single round-robin = n - 1
    // Each round has n / 2 matches
    let rounds = n - 1;
    let half = n / 2;

    // Build a mutable list of team indices (fix index 0, rotate the rest)
    let mut indices: Vec<usize> = (0..n).collect();

    let mut matchday: u32 = 1;

    // First leg (home)
    for _round in 0..rounds {
        let round_date = start_date + Duration::days((matchday as i64 - 1) * 7);
        let date_str = round_date.format("%Y-%m-%d").to_string();

        for i in 0..half {
            let home_idx = indices[i];
            let away_idx = indices[n - 1 - i];

            let fixture = Fixture {
                id: Uuid::new_v4().to_string(),
                competition_id: league_id.clone(),
                matchday,
                date: date_str.clone(),
                home_team_id: team_ids[home_idx].clone(),
                away_team_id: team_ids[away_idx].clone(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            };
            league.fixtures.push(fixture);
        }

        matchday += 1;

        // Rotate: keep index 0 fixed, rotate the rest
        let last = indices.pop().unwrap();
        indices.insert(1, last);
    }

    // Second leg (reverse home/away)
    let mut indices2: Vec<usize> = (0..n).collect();

    for _round in 0..rounds {
        let round_date = start_date + Duration::days((matchday as i64 - 1) * 7);
        let date_str = round_date.format("%Y-%m-%d").to_string();

        for i in 0..half {
            let home_idx = indices2[n - 1 - i]; // Reversed
            let away_idx = indices2[i];

            let fixture = Fixture {
                id: Uuid::new_v4().to_string(),
                competition_id: league_id.clone(),
                matchday,
                date: date_str.clone(),
                home_team_id: team_ids[home_idx].clone(),
                away_team_id: team_ids[away_idx].clone(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            };
            league.fixtures.push(fixture);
        }

        matchday += 1;

        let last = indices2.pop().unwrap();
        indices2.insert(1, last);
    }

    league
}

pub fn generate_knockout_cup(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
    kind: CompetitionType,
    scope: CompetitionScope,
) -> League {
    let competition_id = Uuid::new_v4().to_string();
    let mut cup = League::new(competition_id.clone(), name.to_string(), season, team_ids);
    cup.kind = kind.clone();
    cup.scope = scope;
    cup.rules = CompetitionRules {
        format: CompetitionFormat::Knockout,
        counts_in_season_flow: true,
    };
    cup.standings.clear();
    seed_knockout_round(
        &mut cup,
        team_ids,
        start_date,
        match kind {
            CompetitionType::ContinentalClub => FixtureCompetition::ContinentalClub,
            CompetitionType::InternationalClub => FixtureCompetition::InternationalClub,
            CompetitionType::InternationalNation => FixtureCompetition::InternationalNation,
            CompetitionType::FriendlyCup => FixtureCompetition::FriendlyCup,
            _ => FixtureCompetition::Cup,
        },
    );
    cup
}

fn seed_knockout_round(
    cup: &mut League,
    team_ids: &[String],
    start_date: DateTime<Utc>,
    fixture_competition: FixtureCompetition,
) {
    let round_index = cup.knockout_rounds.len() as u32 + 1;
    let round_name = match team_ids.len() {
        2 => "Final".to_string(),
        4 => "Semifinal".to_string(),
        8 => "Quarterfinal".to_string(),
        size => format!("Round of {size}"),
    };
    let mut round_fixture_ids = Vec::new();
    for (pair_index, pair) in team_ids.chunks(2).enumerate() {
        if pair.len() < 2 {
            continue;
        }
        let fixture_id = Uuid::new_v4().to_string();
        round_fixture_ids.push(fixture_id.clone());
        cup.fixtures.push(Fixture {
            id: fixture_id,
            competition_id: cup.id.clone(),
            matchday: round_index,
            date: (start_date + Duration::days(pair_index as i64))
                .format("%Y-%m-%d")
                .to_string(),
            home_team_id: pair[0].clone(),
            away_team_id: pair[1].clone(),
            competition: fixture_competition.clone(),
            status: FixtureStatus::Scheduled,
            result: None,
        });
    }
    cup.knockout_rounds.push(KnockoutRoundState {
        id: format!("{}-round-{}", cup.id, round_index),
        name: round_name,
        fixture_ids: round_fixture_ids,
        completed: false,
    });
}

pub fn advance_knockout_competition_round(cup: &mut League) {
    if cup.rules.format != CompetitionFormat::Knockout {
        return;
    }

    let Some(next_round) = cup.knockout_rounds.iter_mut().find(|round| !round.completed) else {
        return;
    };
    let round_fixtures: Vec<_> = next_round
        .fixture_ids
        .iter()
        .filter_map(|fixture_id| cup.fixtures.iter().find(|fixture| &fixture.id == fixture_id))
        .collect();
    if round_fixtures.is_empty() || round_fixtures.iter().any(|fixture| fixture.result.is_none()) {
        return;
    }

    next_round.completed = true;
    let winners: Vec<String> = round_fixtures
        .iter()
        .filter_map(|fixture| {
            let result = fixture.result.as_ref()?;
            if result.home_goals >= result.away_goals {
                Some(fixture.home_team_id.clone())
            } else {
                Some(fixture.away_team_id.clone())
            }
        })
        .collect();

    if winners.len() >= 2 {
        let last_round_date = round_fixtures
            .iter()
            .map(|fixture| fixture.date.as_str())
            .max()
            .unwrap_or("2026-01-01");
        let round_start = chrono::NaiveDate::parse_from_str(last_round_date, "%Y-%m-%d")
            .ok()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
            .unwrap_or_else(Utc::now)
            + Duration::days(14);
        let fixture_competition = cup
            .fixtures
            .iter()
            .find(|fixture| next_round.fixture_ids.contains(&fixture.id))
            .map(|fixture| fixture.competition.clone())
            .unwrap_or(FixtureCompetition::Cup);
        seed_knockout_round(cup, &winners, round_start, fixture_competition);
    }
}

pub fn generate_preseason_friendlies(
    team_ids: &[String],
    season_start: DateTime<Utc>,
    max_friendlies: usize,
) -> Vec<Fixture> {
    if team_ids.len() < 2 || max_friendlies == 0 {
        return Vec::new();
    }

    let mut rotation: Vec<Option<usize>> = (0..team_ids.len()).map(Some).collect();
    if rotation.len() % 2 != 0 {
        rotation.push(None);
    }

    let rounds_available = rotation.len().saturating_sub(1);
    let rounds_to_schedule = max_friendlies.min(rounds_available);
    if rounds_to_schedule == 0 {
        return Vec::new();
    }

    let half = rotation.len() / 2;
    let mut fixtures = Vec::with_capacity(rounds_to_schedule * half);

    for round in 0..rounds_to_schedule {
        let weeks_before_start = (rounds_to_schedule.saturating_sub(round)) as i64;
        let date = (season_start - Duration::days(weeks_before_start * 7))
            .format("%Y-%m-%d")
            .to_string();

        for i in 0..half {
            let Some(left_idx) = rotation[i] else {
                continue;
            };
            let Some(right_idx) = rotation[rotation.len() - 1 - i] else {
                continue;
            };

            let (home_idx, away_idx) = if (round + i) % 2 == 0 {
                (left_idx, right_idx)
            } else {
                (right_idx, left_idx)
            };

            fixtures.push(Fixture {
                id: Uuid::new_v4().to_string(),
                competition_id: String::new(),
                matchday: 0,
                date: date.clone(),
                home_team_id: team_ids[home_idx].clone(),
                away_team_id: team_ids[away_idx].clone(),
                competition: FixtureCompetition::Friendly,
                status: FixtureStatus::Scheduled,
                result: None,
            });
        }

        let last = rotation.pop().unwrap();
        rotation.insert(1, last);
    }

    fixtures
}

/// Push any fixture that lands on a reserved date (e.g. an international
/// window) forward to the next free day, so club matches never clash with
/// national-team call-ups. Order is preserved.
pub fn shift_fixtures_off_reserved_dates(league: &mut League, reserved_dates: &[String]) {
    if reserved_dates.is_empty() {
        return;
    }
    let reserved: std::collections::HashSet<&str> =
        reserved_dates.iter().map(|date| date.as_str()).collect();

    for fixture in &mut league.fixtures {
        let Some(mut date) = chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok()
        else {
            continue;
        };
        while reserved.contains(date.format("%Y-%m-%d").to_string().as_str()) {
            match date.succ_opt() {
                Some(next) => date = next,
                None => break,
            }
        }
        fixture.date = date.format("%Y-%m-%d").to_string();
    }
}

pub fn append_fixtures(league: &mut League, mut additional_fixtures: Vec<Fixture>) {
    league.fixtures.append(&mut additional_fixtures);
    league.fixtures.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then(left.matchday.cmp(&right.matchday))
            .then(left.id.cmp(&right.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn shift_fixtures_off_reserved_dates_moves_clashing_matches_forward() {
        let teams: Vec<String> = (0..2).map(|i| format!("team_{i}")).collect();
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let mut league = generate_league("Test League", 2026, &teams, start);

        // Force a clash, then reserve that exact day.
        league.fixtures[0].date = "2026-09-09".to_string();
        let reserved = vec!["2026-09-09".to_string()];

        shift_fixtures_off_reserved_dates(&mut league, &reserved);

        assert!(
            league.fixtures.iter().all(|f| f.date != "2026-09-09"),
            "no fixture should remain on a reserved date"
        );
        assert_eq!(
            league.fixtures[0].date, "2026-09-10",
            "clashing fixture should move to the next free day"
        );
    }

    #[test]
    fn shift_fixtures_off_reserved_dates_is_noop_without_reservations() {
        let teams: Vec<String> = (0..2).map(|i| format!("team_{i}")).collect();
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let mut league = generate_league("Test League", 2026, &teams, start);
        let before: Vec<String> = league.fixtures.iter().map(|f| f.date.clone()).collect();

        shift_fixtures_off_reserved_dates(&mut league, &[]);

        let after: Vec<String> = league.fixtures.iter().map(|f| f.date.clone()).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn test_generate_league_8_teams() {
        let teams: Vec<String> = (0..8).map(|i| format!("team_{}", i)).collect();
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let league = generate_league("Test League", 2026, &teams, start);

        // 8 teams: 7 rounds * 4 matches * 2 legs = 56 fixtures
        assert_eq!(league.fixtures.len(), 56);

        // 14 matchdays (7 per leg)
        let max_md = league.fixtures.iter().map(|f| f.matchday).max().unwrap();
        assert_eq!(max_md, 14);

        // Each team plays 14 matches total
        for team in &teams {
            let count = league
                .fixtures
                .iter()
                .filter(|f| f.home_team_id == *team || f.away_team_id == *team)
                .count();
            assert_eq!(count, 14, "Team {} plays {} matches", team, count);
        }

        // 8 standings entries
        assert_eq!(league.standings.len(), 8);
    }

    #[test]
    fn test_generate_league_16_teams() {
        let teams: Vec<String> = (0..16).map(|i| format!("team_{}", i)).collect();
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let league = generate_league("Premier Division", 2026, &teams, start);

        // 16 teams: 15 rounds * 8 matches * 2 legs = 240 fixtures
        assert_eq!(league.fixtures.len(), 240);

        // 30 matchdays (15 per leg)
        let max_md = league.fixtures.iter().map(|f| f.matchday).max().unwrap();
        assert_eq!(max_md, 30);

        // Each team plays 30 matches total (15 home + 15 away)
        for team in &teams {
            let count = league
                .fixtures
                .iter()
                .filter(|f| f.home_team_id == *team || f.away_team_id == *team)
                .count();
            assert_eq!(count, 30, "Team {} plays {} matches", team, count);
        }

        // 16 standings entries
        assert_eq!(league.standings.len(), 16);

        // No team plays itself
        for f in &league.fixtures {
            assert_ne!(f.home_team_id, f.away_team_id);
        }
    }

    #[test]
    fn generate_preseason_friendlies_marks_fixtures_as_friendlies() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let friendlies = generate_preseason_friendlies(
            &[
                "team_1".to_string(),
                "team_2".to_string(),
                "team_3".to_string(),
                "team_4".to_string(),
            ],
            start,
            3,
        );

        assert_eq!(friendlies.len(), 6);
        assert!(
            friendlies
                .iter()
                .all(|fixture| fixture.competition == FixtureCompetition::Friendly)
        );
        assert!(friendlies.iter().all(|fixture| fixture.matchday == 0));
        assert_eq!(friendlies[0].date, "2026-07-11");
        assert_eq!(friendlies[5].date, "2026-07-25");
    }

    #[test]
    fn generate_preseason_friendlies_gives_each_team_a_fixture_each_week() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let teams: Vec<String> = (1..=8).map(|n| format!("team_{}", n)).collect();
        let friendlies = generate_preseason_friendlies(&teams, start, 4);

        assert_eq!(friendlies.len(), 16);

        for team in &teams {
            let appearances = friendlies
                .iter()
                .filter(|fixture| fixture.home_team_id == *team || fixture.away_team_id == *team)
                .count();
            assert_eq!(
                appearances, 4,
                "{team} should get one fixture per preseason week"
            );
        }
    }

    #[test]
    fn generate_preseason_friendlies_does_not_double_book_teams_on_same_day() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let teams: Vec<String> = (1..=16).map(|n| format!("team_{}", n)).collect();
        let friendlies = generate_preseason_friendlies(&teams, start, 4);

        let unique_dates: std::collections::HashSet<_> = friendlies
            .iter()
            .map(|fixture| fixture.date.clone())
            .collect();
        assert_eq!(unique_dates.len(), 4);

        for date in unique_dates {
            for team in &teams {
                let appearances = friendlies
                    .iter()
                    .filter(|fixture| {
                        fixture.date == date
                            && (fixture.home_team_id == *team || fixture.away_team_id == *team)
                    })
                    .count();
                assert!(appearances <= 1, "{team} is double-booked on {date}");
            }
        }
    }
}
