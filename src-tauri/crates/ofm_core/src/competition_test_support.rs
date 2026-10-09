use crate::{group_stage, schedule};
use chrono::TimeZone;
use domain::league::{
    CompetitionScope, CompetitionType, FixtureCompetition, FixtureStatus, League, MatchResult,
};

pub(crate) fn start() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2033, 2, 1, 0, 0, 0).unwrap()
}

pub(crate) fn teams(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("club-{i}")).collect()
}

pub(crate) fn finish(competition: &mut League) {
    loop {
        let scheduled: Vec<_> = competition
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, f)| f.status == FixtureStatus::Scheduled)
            .map(|(i, _)| i)
            .collect();
        if scheduled.is_empty() {
            break;
        }
        for i in scheduled {
            let f = &mut competition.fixtures[i];
            f.status = FixtureStatus::Completed;
            f.result = Some(MatchResult {
                home_goals: 2,
                away_goals: 0,
                ..Default::default()
            });
            if f.counts_for_league_standings() {
                for entry in &mut competition.standings {
                    if entry.team_id == f.home_team_id {
                        entry.record_result(2, 0);
                    }
                    if entry.team_id == f.away_team_id {
                        entry.record_result(0, 2);
                    }
                }
            }
            group_stage::process_completed_fixture(competition, i);
        }
        schedule::advance_knockout_competition_round(competition);
    }
}

pub(crate) fn table(n: usize, legs: u8) -> League {
    let mut c = League::new("ar-d1-apertura".into(), "Opening".into(), 2033, &teams(n));
    c.fixtures = schedule::build_round_robin_fixtures_with(
        &c.id,
        &c.participant_ids,
        start(),
        FixtureCompetition::League,
        legs,
        7,
    );
    finish(&mut c);
    c
}

pub(crate) fn cup(n: usize) -> League {
    let mut c = schedule::generate_knockout_cup(
        "Cup",
        2033,
        &teams(n),
        start(),
        CompetitionType::Cup,
        CompetitionScope::Domestic,
    );
    finish(&mut c);
    c
}

pub(crate) fn group_cup() -> League {
    let mut c = group_stage::generate_group_knockout_cup(
        "Group cup",
        2033,
        &teams(8),
        start(),
        CompetitionType::Cup,
        CompetitionScope::Domestic,
    );
    finish(&mut c);
    c
}
