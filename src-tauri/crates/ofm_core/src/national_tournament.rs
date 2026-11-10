//! The match-day path for every national tournament, independent of scope and region.

use crate::{game::Game, national_team, world_cup};
use domain::league::{CompetitionType, FixtureStatus, League, MatchResult};

/// Play national tournaments due today, including those outside the user's active simulation scope.
pub fn process_fixtures_due(game: &mut Game, today: &str) -> usize {
    process_selected_fixtures_due(game, today, |competition| {
        competition.kind == CompetitionType::InternationalNation
    })
}

/// The World Cup's rollover catch-up can restrict this same match path to its own competitions.
pub(crate) fn process_selected_fixtures_due(
    game: &mut Game,
    today: &str,
    selected: impl Fn(&League) -> bool,
) -> usize {
    let indices: Vec<_> = game
        .competitions
        .iter()
        .enumerate()
        .filter(|(_, competition)| {
            competition.kind == CompetitionType::InternationalNation && selected(competition)
        })
        .map(|(index, _)| index)
        .collect();
    let mut simulated = 0;
    let mut played_world_cup = false;
    for competition_index in indices {
        let due: Vec<_> = game.competitions[competition_index]
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| {
                fixture.date == today && fixture.status == FixtureStatus::Scheduled
            })
            .map(|(index, _)| index)
            .collect();
        if due.is_empty() {
            continue;
        }
        played_world_cup |=
            world_cup::is_world_cup_competition(&game.competitions[competition_index]);
        simulated += due.len();
        for fixture_index in due {
            play_fixture(game, competition_index, fixture_index, today);
        }
        world_cup::apply_world_cup_match_policies(game, competition_index, today);
    }
    if played_world_cup {
        world_cup::stage_world_cup_playoff_if_ready(game, today);
    }
    simulated
}

fn play_fixture(game: &mut Game, competition_index: usize, fixture_index: usize, today: &str) {
    let competition = &game.competitions[competition_index];
    let fixture = &competition.fixtures[fixture_index];
    let (home, away) = (fixture.home_team_id.clone(), fixture.away_team_id.clone());
    let is_knockout = competition.is_knockout_fixture(&fixture.id);
    // Preserve existing World Cup streams so the extraction cannot change a saved campaign.
    let tag = if world_cup::is_world_cup_competition(competition) {
        format!("world-cup/{home}/{away}")
    } else {
        format!("national-tournament/{}/{}", competition.id, fixture.id)
    };
    let mut rng = game.rng_for(&tag, today);
    let result = play_result(game, &home, &away, is_knockout, &mut rng);
    let (home_goals, away_goals, shootout) = (
        result.home_goals,
        result.away_goals,
        result.home_penalties.is_some(),
    );
    let competition = &mut game.competitions[competition_index];
    let fixture = &mut competition.fixtures[fixture_index];
    fixture.status = FixtureStatus::Completed;
    fixture.result = Some(result);
    crate::group_stage::process_completed_fixture(competition, fixture_index);
    if !world_cup::advance_special_round(competition) {
        crate::schedule::advance_knockout_competition_round(competition);
    }
    game.world_history.apply_national_result(
        &world_cup::nation_code_of_national_team(&home),
        &world_cup::nation_code_of_national_team(&away),
        home_goals,
        away_goals,
        shootout,
    );
}

fn play_result(
    game: &mut Game,
    home: &str,
    away: &str,
    is_knockout: bool,
    rng: &mut impl rand::Rng,
) -> MatchResult {
    if is_knockout {
        let (home_goals, away_goals, home_scorers, away_scorers, home_penalties, away_penalties) =
            national_team::play_national_knockout_match(game, home, away, rng);
        MatchResult {
            home_goals,
            away_goals,
            home_scorers,
            away_scorers,
            home_penalties,
            away_penalties,
            ..Default::default()
        }
    } else {
        let (home_goals, away_goals, home_scorers, away_scorers) =
            national_team::play_national_match(game, home, away, rng);
        MatchResult {
            home_goals,
            away_goals,
            home_scorers,
            away_scorers,
            ..Default::default()
        }
    }
}
