use crate::competition_completion::{CompletionFailure, verify_completed_edition};
use domain::edition_archive::CompletedEdition;
use domain::league::League;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveOutcome {
    Archived,
    AlreadyArchived,
}

pub fn archive_completed_edition(
    archive: &mut Vec<CompletedEdition>,
    competition: &League,
) -> Result<ArchiveOutcome, CompletionFailure> {
    if archive
        .iter()
        .any(|edition| edition.is_edition_of(&competition.id, competition.season))
    {
        return Ok(ArchiveOutcome::AlreadyArchived);
    }
    let legs = competition
        .calendar
        .as_ref()
        .and_then(|calendar| calendar.league_legs);
    let proof = verify_completed_edition(competition, legs)?;
    let champion_id = proof
        .champion_id
        .or_else(|| {
            competition
                .sorted_standings()
                .into_iter()
                .next()
                .map(|entry| entry.team_id)
        })
        .ok_or(CompletionFailure::InvalidTableShape)?;
    archive.push(CompletedEdition {
        competition_id: competition.id.clone(),
        season: competition.season,
        completed_on: proof.completed_on.format("%Y-%m-%d").to_string(),
        champion_id,
        participant_ids: competition.participant_ids.clone(),
        standings: competition.sorted_standings(),
        groups: competition.groups.clone(),
        knockout_rounds: competition.knockout_rounds.clone(),
        fixtures: competition.fixtures.clone(),
    });
    Ok(ArchiveOutcome::Archived)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition_test_support::{cup, group_cup, table};
    use domain::competition_calendar::CalendarMetadata;
    use domain::league::FixtureStatus;

    fn authored_table(clubs: usize, legs: u8) -> League {
        let mut competition = table(clubs, legs);
        competition.calendar = Some(CalendarMetadata {
            league_legs: Some(legs),
            ..Default::default()
        });
        competition
    }

    fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    fn archived(competition: &League) -> Vec<CompletedEdition> {
        let mut archive = Vec::new();
        assert_eq!(
            archive_completed_edition(&mut archive, competition),
            Ok(ArchiveOutcome::Archived)
        );
        archive
    }

    /// Given a finished league, when archived, then one record freezes its table, champion and clubs.
    #[test]
    fn finished_league_is_archived_with_its_final_table() {
        let competition = authored_table(4, 2);
        let archive = archived(&competition);
        let [record] = archive.as_slice() else {
            panic!("expected exactly one record, got {}", archive.len());
        };
        assert!(record.is_edition_of(&competition.id, competition.season));
        assert_eq!(
            json(&record.standings),
            json(&competition.sorted_standings())
        );
        assert_eq!(
            record.champion_id,
            competition.sorted_standings()[0].team_id
        );
        assert_eq!(record.participant_ids, competition.participant_ids);
        assert_eq!(json(&record.fixtures), json(&competition.fixtures));
    }

    /// Given a Scheduled competitive fixture, when archived, then nothing is recorded.
    #[test]
    fn league_with_a_scheduled_fixture_is_not_archived() {
        let mut competition = authored_table(4, 2);
        competition.fixtures[0].status = FixtureStatus::Scheduled;
        let mut archive = Vec::new();
        assert_eq!(
            archive_completed_edition(&mut archive, &competition),
            Err(CompletionFailure::UnfinishedFixtures)
        );
        assert!(archive.is_empty());
    }

    /// Given a table whose legs were never authored, when archived, then it is blocked, not guessed.
    #[test]
    fn league_without_authored_legs_is_not_archived() {
        let competition = table(4, 2);
        let mut archive = Vec::new();
        assert_eq!(
            archive_completed_edition(&mut archive, &competition),
            Err(CompletionFailure::MissingTableSpecification)
        );
        assert!(archive.is_empty());
    }

    /// Given a finished cup, when archived, then the record holds the bracket and the final's winner.
    #[test]
    fn finished_knockout_archives_bracket_and_champion() {
        let competition = cup(8);
        let archive = archived(&competition);
        assert_eq!(archive[0].knockout_rounds, competition.knockout_rounds);
        assert_eq!(
            Some(archive[0].champion_id.clone()),
            crate::world_cup::world_cup_champion(&competition)
        );
    }

    /// Given a finished group cup, when archived, then the record holds groups and bracket.
    #[test]
    fn finished_group_cup_archives_groups_and_bracket() {
        let competition = group_cup();
        let archive = archived(&competition);
        assert_eq!(json(&archive[0].groups), json(&competition.groups));
        assert_eq!(archive[0].knockout_rounds, competition.knockout_rounds);
    }

    /// Given a cup with an unplayed semifinal, when archived, then nothing is recorded.
    #[test]
    fn unfinished_knockout_is_not_archived() {
        let mut competition = cup(8);
        let last_round = competition
            .knockout_rounds
            .last()
            .unwrap()
            .fixture_ids
            .clone();
        for fixture in &mut competition.fixtures {
            if last_round.contains(&fixture.id) {
                fixture.status = FixtureStatus::Scheduled;
                fixture.result = None;
            }
        }
        let mut archive = Vec::new();
        assert!(archive_completed_edition(&mut archive, &competition).is_err());
        assert!(archive.is_empty());
    }

    /// Given an archived edition, when archived again, then there is still one unchanged record.
    #[test]
    fn archiving_twice_keeps_the_first_record() {
        let competition = authored_table(4, 2);
        let mut archive = archived(&competition);
        let first = archive.clone();
        assert_eq!(
            archive_completed_edition(&mut archive, &competition),
            Ok(ArchiveOutcome::AlreadyArchived)
        );
        assert_eq!(json(&archive), json(&first));
    }

    /// Given an archived edition whose live competition then resets, when archived again, then the freeze holds.
    #[test]
    fn archive_is_immutable_after_the_live_competition_resets() {
        let mut competition = authored_table(4, 2);
        let mut archive = archived(&competition);
        let frozen = archive.clone();
        competition
            .standings
            .iter_mut()
            .for_each(|entry| *entry = domain::league::StandingEntry::new(entry.team_id.clone()));
        competition.fixtures.clear();
        assert_eq!(
            archive_completed_edition(&mut archive, &competition),
            Ok(ArchiveOutcome::AlreadyArchived)
        );
        assert_eq!(json(&archive), json(&frozen));
    }

    /// Given a finished half and a live sibling in one family, when the half is archived, then the sibling is untouched and unrecorded.
    #[test]
    fn archive_keeps_sibling_halves_separate() {
        let apertura = authored_table(4, 2);
        let mut clausura = authored_table(4, 2);
        clausura.id = "ar-d1-clausura".into();
        clausura.fixtures.iter_mut().for_each(|f| {
            f.competition_id = clausura.id.clone();
            f.status = FixtureStatus::Scheduled;
            f.result = None;
        });
        let before = serde_json::to_value(&clausura).unwrap();
        let archive = archived(&apertura);
        assert_eq!(archive.len(), 1);
        assert_eq!(archive[0].competition_id, apertura.id);
        assert!(!archive.iter().any(|r| r.competition_id == clausura.id));
        assert_eq!(serde_json::to_value(&clausura).unwrap(), before);
    }

    /// Given any finished edition, when archived, then the competition itself is unchanged.
    #[test]
    fn archiving_does_not_mutate_the_competition() {
        let competition = authored_table(4, 2);
        let before = serde_json::to_value(&competition).unwrap();
        archived(&competition);
        assert_eq!(serde_json::to_value(&competition).unwrap(), before);
    }
}
