//! Strict, read-only completion proofs for the upcoming edition archive.
//! The legacy rollover guard is not a proof that a cup reached its final.

use chrono::NaiveDate;
use domain::league::{
    CompetitionFormat, Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq, Eq)]
pub struct CompletionProof {
    pub completed_on: NaiveDate,
    pub champion_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionFailure {
    MissingTableSpecification,
    UnfinishedFixtures,
    InvalidFixtureData,
    InvalidTableShape,
    InvalidBracket,
    InvalidGroups,
}

/// Does not mutate fixtures/tables or trigger any lifecycle effect.
/// `league_legs` must come from verified authored metadata, not the clock.
pub fn verify_completed_edition(
    competition: &League,
    league_legs: Option<u8>,
) -> Result<CompletionProof, CompletionFailure> {
    let table = competition.rules.format == CompetitionFormat::LeagueTable;
    let legs = if table {
        Some(
            league_legs
                .filter(|legs| *legs > 0)
                .ok_or(CompletionFailure::MissingTableSpecification)?,
        )
    } else {
        None
    };
    let participants =
        unique_teams(&competition.participant_ids).ok_or(CompletionFailure::InvalidFixtureData)?;
    let fixtures: Vec<_> = competition
        .fixtures
        .iter()
        .filter(|f| {
            if table {
                f.counts_for_league_standings()
            } else {
                !matches!(
                    f.competition,
                    FixtureCompetition::Friendly | FixtureCompetition::PreseasonTournament
                )
            }
        })
        .collect();
    if fixtures.is_empty()
        || fixtures
            .iter()
            .any(|f| f.status != FixtureStatus::Completed)
    {
        return Err(CompletionFailure::UnfinishedFixtures);
    }
    let mut ids = BTreeSet::new();
    let mut completed_on = None;
    for f in &fixtures {
        let date = crate::schedule::date_str_to_utc(&f.date)
            .ok_or(CompletionFailure::InvalidFixtureData)?
            .date_naive();
        if f.id.is_empty()
            || !ids.insert(f.id.as_str())
            || f.competition_id != competition.id
            || f.result.is_none()
            || f.home_team_id == f.away_team_id
            || !participants.contains(f.home_team_id.as_str())
            || !participants.contains(f.away_team_id.as_str())
        {
            return Err(CompletionFailure::InvalidFixtureData);
        }
        completed_on = Some(completed_on.map_or(date, |last: NaiveDate| last.max(date)));
    }
    let champion_id = if let Some(legs) = legs {
        validate_table(
            &competition.participant_ids,
            &competition.standings,
            &fixtures,
            legs,
        )
        .map_err(|()| CompletionFailure::InvalidTableShape)?;
        None
    } else {
        let entrants = if competition.rules.format == CompetitionFormat::GroupAndKnockout {
            validate_groups(competition, &fixtures)?;
            crate::group_stage::knockout_qualifiers(competition)
        } else {
            competition.participant_ids.clone()
        };
        validate_bracket(competition, &fixtures, &entrants)?;
        Some(
            crate::world_cup::world_cup_champion(competition)
                .ok_or(CompletionFailure::InvalidBracket)?,
        )
    };
    Ok(CompletionProof {
        completed_on: completed_on.ok_or(CompletionFailure::UnfinishedFixtures)?,
        champion_id,
    })
}

/// `advancing_team_id` favours home on a level score, so a drawn tie needs a decisive shootout
/// to name a real winner.
fn decided_knockout_tie(fixture: &Fixture) -> bool {
    fixture.result.as_ref().is_some_and(|result| {
        result.home_goals != result.away_goals
            || matches!(
                (result.home_penalties, result.away_penalties),
                (Some(home), Some(away)) if home != away
            )
    })
}

fn unique_teams(ids: &[String]) -> Option<BTreeSet<&str>> {
    let unique: BTreeSet<_> = ids.iter().map(String::as_str).collect();
    (ids.len() >= 2 && unique.len() == ids.len() && !unique.contains("")).then_some(unique)
}

// Validate actual pair coverage, not just a total that a duplicate could satisfy.
// The result writer owns points arithmetic; use that same writer to verify that
// final results have reached the table before allowing an archive snapshot.
fn validate_table(
    team_ids: &[String],
    standings: &[StandingEntry],
    fixtures: &[&Fixture],
    legs: u8,
) -> Result<(), ()> {
    let teams = unique_teams(team_ids).ok_or(())?;
    if legs == 0 || standings.len() != teams.len() {
        return Err(());
    }
    let mut pairs = BTreeMap::<(&str, &str), usize>::new();
    let mut computed: BTreeMap<_, _> = teams
        .iter()
        .map(|id| (*id, StandingEntry::new((*id).to_string())))
        .collect();
    for f in fixtures {
        let result = f.result.as_ref().ok_or(())?;
        *pairs.entry((&f.home_team_id, &f.away_team_id)).or_default() += 1;
        computed
            .get_mut(f.home_team_id.as_str())
            .ok_or(())?
            .record_result(result.home_goals, result.away_goals);
        computed
            .get_mut(f.away_team_id.as_str())
            .ok_or(())?
            .record_result(result.away_goals, result.home_goals);
    }
    for home in &teams {
        for away in &teams {
            if home >= away {
                continue;
            }
            let a = pairs.get(&(*home, *away)).copied().unwrap_or(0);
            let b = pairs.get(&(*away, *home)).copied().unwrap_or(0);
            if a + b != usize::from(legs) || a.abs_diff(b) > usize::from(legs % 2) {
                return Err(());
            }
        }
    }
    let mut seen = BTreeSet::new();
    for saved in standings {
        if !seen.insert(saved.team_id.as_str()) {
            return Err(());
        }
        let expected = computed.get(saved.team_id.as_str()).ok_or(())?;
        if standing_record(saved) != standing_record(expected) {
            return Err(());
        }
    }
    Ok(())
}

fn standing_record(s: &StandingEntry) -> [u32; 7] {
    [
        s.played,
        s.won,
        s.drawn,
        s.lost,
        s.goals_for,
        s.goals_against,
        s.points,
    ]
}

fn validate_groups(competition: &League, fixtures: &[&Fixture]) -> Result<(), CompletionFailure> {
    let fail = CompletionFailure::InvalidGroups;
    if competition.groups.is_empty() {
        return Err(fail);
    }
    let mut grouped = BTreeSet::new();
    let mut grouped_fixtures = BTreeSet::new();
    for group in &competition.groups {
        let teams = unique_teams(&group.team_ids).ok_or(fail)?;
        if teams.iter().any(|id| !grouped.insert(*id)) {
            return Err(fail);
        }
        let matches: Vec<_> = fixtures
            .iter()
            .copied()
            .filter(|f| {
                !competition.is_knockout_fixture(&f.id)
                    && teams.contains(f.home_team_id.as_str())
                    && teams.contains(f.away_team_id.as_str())
            })
            .collect();
        validate_table(
            &group.team_ids,
            &group.standings,
            &matches,
            competition.rules.group_stage_legs,
        )
        .map_err(|()| fail)?;
        grouped_fixtures.extend(matches.into_iter().map(|f| f.id.as_str()));
    }
    let participants = unique_teams(&competition.participant_ids).ok_or(fail)?;
    if grouped != participants
        || fixtures.iter().any(|f| {
            !competition.is_knockout_fixture(&f.id) && !grouped_fixtures.contains(f.id.as_str())
        })
    {
        return Err(fail);
    }
    Ok(())
}

fn validate_bracket(
    competition: &League,
    fixtures: &[&Fixture],
    entrants: &[String],
) -> Result<(), CompletionFailure> {
    let fail = CompletionFailure::InvalidBracket;
    let mut expected = unique_teams(entrants).ok_or(fail)?;
    let fixture_map: BTreeMap<_, _> = fixtures.iter().map(|f| (f.id.as_str(), *f)).collect();
    let mut references = BTreeSet::new();
    if competition.knockout_rounds.is_empty() {
        return Err(fail);
    }
    for (i, round) in competition.knockout_rounds.iter().enumerate() {
        if !round.completed {
            return Err(CompletionFailure::UnfinishedFixtures);
        }
        let bye_count = crate::schedule::knockout_bye_count(expected.len()).ok_or(fail)?;
        if round.bye_team_ids.len() != bye_count
            || round.fixture_ids.len() != (expected.len() - bye_count) / 2
        {
            return Err(fail);
        }
        let mut playing = BTreeSet::new();
        let mut advancing = BTreeSet::new();
        for bye in &round.bye_team_ids {
            if !expected.contains(bye.as_str()) || !playing.insert(bye.as_str()) {
                return Err(fail);
            }
            advancing.insert(bye.as_str());
        }
        for id in &round.fixture_ids {
            if !references.insert(id.as_str()) {
                return Err(fail);
            }
            let f = fixture_map.get(id.as_str()).ok_or(fail)?;
            for team in [f.home_team_id.as_str(), f.away_team_id.as_str()] {
                if !expected.contains(team) || !playing.insert(team) {
                    return Err(fail);
                }
            }
            if !decided_knockout_tie(f) {
                return Err(fail);
            }
            advancing.insert(f.advancing_team_id().ok_or(fail)?);
        }
        if playing != expected {
            return Err(fail);
        }
        let terminal = i + 1 == competition.knockout_rounds.len();
        if terminal != (advancing.len() == 1) {
            return Err(fail);
        }
        expected = advancing;
    }
    if competition.rules.format == CompetitionFormat::Knockout && references.len() != fixtures.len()
    {
        return Err(fail);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition_test_support::{cup, group_cup, table};
    use domain::league::{FixtureCompetition, FixtureStatus, MatchResult};

    fn blocked(c: &League, legs: Option<u8>, reason: CompletionFailure) {
        assert_eq!(verify_completed_edition(c, legs), Err(reason));
    }

    /// Given a final level after regulation with no shootout, or a shootout that is itself level,
    /// when completion is verified, then no champion is fabricated for the home side.
    #[test]
    fn drawn_knockout_tie_without_a_decisive_shootout_is_not_complete() {
        for penalties in [None, Some((3, 3))] {
            let mut competition = cup(4);
            let final_id = competition
                .knockout_rounds
                .last()
                .and_then(|round| round.fixture_ids.first().cloned())
                .unwrap();
            let final_tie = competition
                .fixtures
                .iter_mut()
                .find(|fixture| fixture.id == final_id)
                .unwrap();
            final_tie.result = Some(MatchResult {
                home_goals: 1,
                away_goals: 1,
                home_penalties: penalties.map(|(home, _)| home),
                away_penalties: penalties.map(|(_, away)| away),
                ..Default::default()
            });
            blocked(&competition, None, CompletionFailure::InvalidBracket);
        }
    }

    /// Given completed three and four club tables of one and two legs, when completion is verified, then each full authored schedule has a proof with its actual final date.
    #[test]
    fn completed_tables_respect_authored_legs_and_odd_fields() {
        for n in [3, 4] {
            for legs in [1, 2] {
                let c = table(n, legs);
                let proof = verify_completed_edition(&c, Some(legs)).unwrap();
                assert_eq!(
                    proof.completed_on.to_string(),
                    c.fixtures.last().unwrap().date
                );
                assert_eq!(proof.champion_id, None);
            }
        }
    }

    /// Given a completed table with no verified legs, when completion is verified, then it is blocked rather than assumed two-legged.
    #[test]
    fn missing_table_specification_cannot_invent_two_legs() {
        for legs in [None, Some(0)] {
            blocked(
                &table(4, 2),
                legs,
                CompletionFailure::MissingTableSpecification,
            );
        }
    }

    /// Given a final fixture that is Scheduled or InProgress but carries a stale result, when completion is verified, then the table stays incomplete.
    #[test]
    fn unfinished_table_blocks_even_when_a_result_is_present() {
        for status in [FixtureStatus::Scheduled, FixtureStatus::InProgress] {
            let mut c = table(4, 2);
            c.fixtures.last_mut().unwrap().status = status;
            blocked(&c, Some(2), CompletionFailure::UnfinishedFixtures);
        }
    }

    /// Given the expected fixture count with one pairing duplicated, when completion is verified, then the invalid table shape blocks it.
    #[test]
    fn duplicate_pair_cannot_replace_a_missing_table_match() {
        let mut c = table(4, 2);
        c.fixtures[1].home_team_id = c.fixtures[0].home_team_id.clone();
        c.fixtures[1].away_team_id = c.fixtures[0].away_team_id.clone();
        blocked(&c, Some(2), CompletionFailure::InvalidTableShape);
    }

    /// Given completed fixtures with a missing result, bad date, duplicate id, wrong competition, self-pairing or unknown club, when completion is verified, then each blocks it.
    #[test]
    fn malformed_fixture_cannot_supply_completion_proof() {
        for case in 0..6 {
            let mut c = table(4, 2);
            match case {
                0 => c.fixtures[0].result = None,
                1 => c.fixtures[0].date = "not-a-date".into(),
                2 => c.fixtures[0].id = c.fixtures[1].id.clone(),
                3 => c.fixtures[0].competition_id = "another-edition".into(),
                4 => c.fixtures[0].away_team_id = c.fixtures[0].home_team_id.clone(),
                _ => c.fixtures[0].away_team_id = "outsider".into(),
            }
            blocked(&c, Some(2), CompletionFailure::InvalidFixtureData);
        }
    }

    /// Given an edition with no competitive fixtures, when completion is verified, then it stays incomplete.
    #[test]
    fn empty_edition_cannot_supply_completion_proof() {
        let mut c = table(4, 2);
        c.fixtures.clear();
        blocked(&c, Some(2), CompletionFailure::UnfinishedFixtures);
    }

    /// Given a complete table and an unplayed friendly, when completion is verified, then the friendly is ignored.
    #[test]
    fn optional_friendlies_do_not_block_a_completed_table() {
        let mut c = table(4, 2);
        c.fixtures.push(Fixture {
            competition: FixtureCompetition::Friendly,
            ..Default::default()
        });
        assert!(verify_completed_edition(&c, Some(2)).is_ok());
    }

    /// Given completed cups including odd fields with byes and a final decided on penalties, when completion is verified, then only the terminal bracket yields its actual champion.
    #[test]
    fn terminal_knockout_proof_uses_the_shared_champion_rule() {
        for n in [2, 3, 5, 8] {
            let mut c = cup(n);
            let f = c.fixtures.last_mut().unwrap();
            f.result = Some(MatchResult {
                home_goals: 1,
                away_goals: 1,
                home_penalties: Some(2),
                away_penalties: Some(4),
                ..Default::default()
            });
            let expected = f.away_team_id.clone();
            let proof = verify_completed_edition(&c, None).unwrap();
            assert_eq!(proof.champion_id, Some(expected));
            assert_eq!(proof.champion_id, crate::world_cup::world_cup_champion(&c));
        }
    }

    /// Given played quarterfinals followed by Scheduled semifinals, when completion is verified, then the bracket stays incomplete.
    #[test]
    fn unfinished_knockout_cannot_be_archived() {
        let mut c = cup(8);
        let semifinal_ids = c.knockout_rounds[1].fixture_ids.clone();
        c.knockout_rounds.truncate(2);
        c.knockout_rounds[1].completed = false;
        c.fixtures.retain(|f| f.matchday <= 2);
        for f in &mut c.fixtures {
            if semifinal_ids.contains(&f.id) {
                f.status = FixtureStatus::Scheduled;
                f.result = None;
            }
        }
        blocked(&c, None, CompletionFailure::UnfinishedFixtures);
    }

    /// Given a completed non-terminal round whose successor is missing, when completion is verified, then the missing progression blocks it.
    #[test]
    fn missing_terminal_round_cannot_masquerade_as_a_final() {
        let mut c = cup(8);
        c.knockout_rounds.truncate(1);
        c.fixtures.retain(|f| f.matchday == 1);
        blocked(&c, None, CompletionFailure::InvalidBracket);
    }

    /// Given a completed cup with missing or duplicate references, invalid byes or forged progression, when completion is verified, then each blocks it.
    #[test]
    fn corrupt_bracket_references_cannot_supply_a_champion() {
        for case in 0..4 {
            let mut c = cup(8);
            match case {
                0 => c.knockout_rounds[0].fixture_ids[0] = "missing".into(),
                1 => {
                    c.knockout_rounds[0].fixture_ids[1] =
                        c.knockout_rounds[0].fixture_ids[0].clone()
                }
                2 => c.knockout_rounds[0].bye_team_ids.push("outsider".into()),
                _ => {
                    let semifinal = c.knockout_rounds[1].fixture_ids[0].clone();
                    let loser = c.fixtures[0].away_team_id.clone();
                    c.fixtures
                        .iter_mut()
                        .find(|f| f.id == semifinal)
                        .unwrap()
                        .home_team_id = loser;
                }
            }
            blocked(&c, None, CompletionFailure::InvalidBracket);
        }
    }

    /// Given completed groups that seed an unfinished bracket, when completion is verified before and after the final, then only the completed terminal knockout yields a proof.
    #[test]
    fn completed_groups_wait_for_the_terminal_knockout() {
        let mut c = group_cup();
        assert!(verify_completed_edition(&c, None).is_ok());
        c.fixtures.last_mut().unwrap().status = FixtureStatus::Scheduled;
        blocked(&c, None, CompletionFailure::UnfinishedFixtures);
    }

    /// Given a group cup with a missing or duplicated group match, when completion is verified, then the invalid group shape blocks it.
    #[test]
    fn missing_group_match_cannot_be_hidden_by_a_completed_final() {
        for duplicate in [false, true] {
            let mut c = group_cup();
            if duplicate {
                c.fixtures[1].home_team_id = c.fixtures[0].home_team_id.clone();
                c.fixtures[1].away_team_id = c.fixtures[0].away_team_id.clone();
            } else {
                c.fixtures.remove(0);
            }
            blocked(&c, None, CompletionFailure::InvalidGroups);
        }
    }

    /// Given completed fixtures whose results were never applied to the table or groups, when completion is verified, then no final table is certified.
    #[test]
    fn stale_standings_block_a_completed_fixture_set() {
        let mut c = table(4, 2);
        c.standings[0].played = 0;
        blocked(&c, Some(2), CompletionFailure::InvalidTableShape);
        let mut c = group_cup();
        c.groups[0].standings[0].points = 0;
        blocked(&c, None, CompletionFailure::InvalidGroups);
    }

    /// Given a JSON-loaded completed half and an InProgress sibling, when the completed half is verified repeatedly, then the proofs agree and both competitions stay byte-equivalent.
    #[test]
    fn loaded_completion_proof_preserves_the_live_sibling() {
        let opening = table(4, 2);
        let mut closing = table(4, 2);
        closing.id = "ar-d1-clausura".into();
        closing.fixtures[0].status = FixtureStatus::InProgress;
        let before = serde_json::to_string(&vec![opening, closing]).unwrap();
        let loaded: Vec<League> = serde_json::from_str(&before).unwrap();
        let proof = verify_completed_edition(&loaded[0], Some(2)).unwrap();
        assert_eq!(verify_completed_edition(&loaded[0], Some(2)), Ok(proof));
        assert_eq!(serde_json::to_string(&loaded).unwrap(), before);
    }
}
