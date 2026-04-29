use chrono::{DateTime, Duration, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
use uuid::Uuid;

/// Generate a full double round-robin schedule (home & away) for the given teams.
/// Matchdays are spaced 7 days apart starting from `start_date`.
/// Uses the "circle method" for balanced scheduling.
pub fn generate_league(
    name: &str,
    season: u32,
    team_ids: &[String],
    start_date: DateTime<Utc>,
) -> League {
    let n = team_ids.len();
    assert!(n >= 2, "Need at least 2 teams for a league");

    let league_id = Uuid::new_v4().to_string();
    let mut league = League::new(league_id, name.to_string(), season, team_ids);

    // For round-robin with n teams (n must be even; if odd, add a "bye" — we assume even here)
    // Number of rounds in a single round-robin = n - 1
    // Each round has n / 2 matches
    let rounds = n - 1;
    let half = n / 2;

    // Build a mutable list of team indices (circle method: fix index 0, rotate the rest)
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

pub fn generate_preseason_friendlies(
    user_team_id: &str,
    opponent_ids: &[String],
    season_start: DateTime<Utc>,
    max_friendlies: usize,
) -> Vec<Fixture> {
    opponent_ids
        .iter()
        .filter(|opponent_id| opponent_id.as_str() != user_team_id)
        .take(max_friendlies)
        .enumerate()
        .map(|(index, opponent_id)| {
            let weeks_before_start = (max_friendlies.saturating_sub(index)) as i64;
            let date = (season_start - Duration::days(weeks_before_start * 7))
                .format("%Y-%m-%d")
                .to_string();
            let (home_team_id, away_team_id) = if index % 2 == 0 {
                (user_team_id.to_string(), opponent_id.clone())
            } else {
                (opponent_id.clone(), user_team_id.to_string())
            };

            Fixture {
                id: Uuid::new_v4().to_string(),
                matchday: 0,
                date,
                home_team_id,
                away_team_id,
                competition: FixtureCompetition::Friendly,
                status: FixtureStatus::Scheduled,
                result: None,
            }
        })
        .collect()
}

pub fn generate_domestic_cup_round(
    team_ids: &[String],
    date: DateTime<Utc>,
    round_name_matchday: u32,
) -> Vec<Fixture> {
    let mut fixtures = Vec::new();
    if team_ids.len() < 2 {
        return fixtures;
    }
    for pair in team_ids.chunks(2) {
        if pair.len() < 2 {
            continue;
        }
        fixtures.push(Fixture {
            id: Uuid::new_v4().to_string(),
            matchday: round_name_matchday,
            date: date.format("%Y-%m-%d").to_string(),
            home_team_id: pair[0].clone(),
            away_team_id: pair[1].clone(),
            competition: FixtureCompetition::PreseasonTournament,
            status: FixtureStatus::Scheduled,
            result: None,
        });
    }
    fixtures
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
            "team_1",
            &[
                "team_2".to_string(),
                "team_3".to_string(),
                "team_4".to_string(),
            ],
            start,
            3,
        );

        assert_eq!(friendlies.len(), 3);
        assert!(
            friendlies
                .iter()
                .all(|fixture| fixture.competition == FixtureCompetition::Friendly)
        );
        assert!(friendlies.iter().all(|fixture| fixture.matchday == 0));
        assert_eq!(friendlies[0].date, "2026-07-11");
        assert_eq!(friendlies[2].date, "2026-07-25");
    }
}
