//! Founding a world's competitions, and keeping them coherent afterwards.
//!
//! `build_foundation_competitions` is the entry point; the plan it works from
//! lives in [`plan`], which is large enough to be its own file. The rest is
//! maintenance: rebuilding around a mid-season takeover date, filling in
//! foundations a loaded save is missing, reserving international windows, and
//! resolving which regions and competitions a career actually simulates.

mod plan;

use chrono::{DateTime, Datelike, Duration, Utc};

use domain::league::{CompetitionFormat, League};
use ofm_core::game::Game;

use super::{
    build_national_teams, competition_required_region_ids, infer_team_region_id,
    preseason_league_year, preseason_season_start,
};
use plan::build_foundation_competition_plan;

// `team_season_anchor` is used by `start_new_game`, which lives a level up, so
// it needs a re-export rather than the plain import the plan builder gets.
pub(super) use plan::team_season_anchor;

pub(super) fn finalize_brazil_state_competition(competition: &mut League) {
    competition.rules.counts_in_season_flow = false;
    competition.rules.knockout_round_gap_days = 7;
}

pub(super) fn build_foundation_competitions(game: &Game) -> Vec<League> {
    let game_start = game.clock.start_date;
    let season = preseason_league_year(&game.clock);
    build_foundation_competition_plan(game, game_start)
        .iter()
        .filter_map(|(def, start)| {
            let mut competition =
                ofm_core::generator::build_explicit_competition(def, season, *start)?;
            // FM-style: if this competition's season already began before the game
            // anchor date, simulate the missing matchdays so the player joins a
            // living in-progress season rather than a blank table.
            if *start <= game_start {
                ofm_core::catchup::simulate_past_fixtures(
                    &mut competition,
                    &game.players,
                    game_start,
                );
            }
            if competition.id.starts_with("br-state-") {
                finalize_brazil_state_competition(&mut competition);
            }
            Some(competition)
        })
        .collect()
}

pub(super) fn rebuild_competitions_for_management_date(
    game: &mut Game,
    management_date: DateTime<Utc>,
) {
    let players = &game.players;
    for competition in &mut game.competitions {
        // International tournaments (the World Cup and its qualifying) own a fixed
        // calendar tied to the cup year, not the club's hemisphere. Re-anchoring
        // them against a club's season start would corrupt their dates (and
        // orphan a future-dated kickoff), so leave them untouched.
        if ofm_core::world_cup::is_world_cup_competition(competition)
            || ofm_core::world_cup::is_world_cup_qualifying(competition)
        {
            continue;
        }
        let (start, is_mid_season) = ofm_core::generator::start_date_at_game_open(
            management_date,
            competition.season_start_month,
            competition.season_start_day,
        );
        let season = start.year() as u32;
        match competition.rules.format {
            CompetitionFormat::LeagueTable => {
                ofm_core::schedule::regenerate_league_for_season(competition, season, start)
            }
            CompetitionFormat::GroupAndKnockout => {
                ofm_core::group_stage::regenerate_for_season(competition, season, start)
            }
            CompetitionFormat::Knockout => {
                ofm_core::schedule::regenerate_knockout_for_season(competition, season, start)
            }
        }
        if is_mid_season {
            ofm_core::catchup::simulate_past_fixtures(competition, players, management_date);
        }
    }

    let existing: std::collections::HashSet<String> = game
        .competitions
        .iter()
        .map(|competition| competition.id.clone())
        .collect();
    let season = preseason_league_year(&game.clock);
    let mut missing_states: Vec<(League, DateTime<Utc>)> =
        build_foundation_competition_plan(game, management_date)
            .into_iter()
            .filter(|(definition, _)| {
                definition.id.starts_with("br-state-") && !existing.contains(&definition.id)
            })
            .filter_map(|(definition, start)| {
                let mut competition =
                    ofm_core::generator::build_explicit_competition(&definition, season, start)?;
                finalize_brazil_state_competition(&mut competition);
                Some((competition, start))
            })
            .collect();
    for (competition, start) in &mut missing_states {
        if *start <= management_date {
            ofm_core::catchup::simulate_past_fixtures(competition, &game.players, management_date);
        }
    }
    game.competitions
        .extend(missing_states.into_iter().map(|(c, _)| c));
}

pub(super) fn ensure_multi_competition_foundations(game: &mut Game) {
    if game.national_teams.is_empty() {
        game.national_teams = build_national_teams(game);
    }
    if game.competitions.is_empty() {
        game.competitions = build_foundation_competitions(game);
    }
    if game.active_region_ids.is_empty() {
        game.active_region_ids = game
            .competitions
            .iter()
            .filter_map(|competition| competition.region_id.clone())
            .collect();
        game.active_region_ids.sort();
        game.active_region_ids.dedup();
    }
    if game.active_competition_ids.is_empty() {
        game.active_competition_ids = game
            .competitions
            .iter()
            .map(|competition| competition.id.clone())
            .collect();
    }
    ensure_international_windows(game);
    game.sync_legacy_league();
}

/// Schedule national-team friendlies on international windows and keep club
/// fixtures off those dates, so call-ups never clash with club matches.
/// Idempotent: existing national-team fixtures (e.g. from a loaded save) are
/// left untouched, and shifting already-clear club fixtures is a no-op.
pub(super) fn ensure_international_windows(game: &mut Game) {
    // A career that opens during a World Cup summer stages the tournament right
    // away: the World Cup is otherwise created only at season rollover, which a
    // fresh save beginning in a cup summer (e.g. mid-2026) never reaches, so the
    // edition would simply never happen. It fills the summer break, so no window
    // friendlies/qualifiers are scheduled when it runs.
    let now = game.clock.current_date;
    let opens_in_world_cup_summer =
        ofm_core::world_cup::is_world_cup_summer(now.year()) && (6..=8).contains(&now.month());
    if opens_in_world_cup_summer
        && ofm_core::world_cup::schedule_world_cup_if_due(game, now + Duration::days(2))
    {
        for national_team in game.national_teams.iter_mut() {
            national_team.fixtures.clear();
        }
        return;
    }

    let window_dates =
        ofm_core::national_team::international_window_dates(preseason_season_start(&game.clock));
    if window_dates.is_empty() {
        return;
    }

    let needs_fixtures = game
        .national_teams
        .iter()
        .all(|team| team.fixtures.is_empty());
    let qualifying_running = game
        .competitions
        .iter()
        .any(ofm_core::world_cup::is_world_cup_qualifying);
    let leads_into_world_cup =
        ofm_core::world_cup::season_leads_into_world_cup(preseason_season_start(&game.clock));
    let starts_qualifying = ofm_core::world_cup::season_starts_world_cup_qualifying(
        preseason_season_start(&game.clock),
    );
    if needs_fixtures && !qualifying_running {
        // A career starting two seasons before a World Cup opens with the full
        // home-and-away qualifying campaign; one starting the season before
        // squeezes in a compressed campaign; any other season opens with
        // friendlies.
        if starts_qualifying {
            ofm_core::world_cup::schedule_world_cup_qualifying(
                game,
                preseason_season_start(&game.clock).year() + 2,
                &window_dates,
            );
        } else if leads_into_world_cup {
            ofm_core::world_cup::schedule_world_cup_qualifying(
                game,
                preseason_season_start(&game.clock).year() + 1,
                &window_dates,
            );
        } else {
            ofm_core::national_team::schedule_national_team_friendlies(
                &mut game.national_teams,
                &window_dates,
                &mut rand::rng(),
            );
        }
    }

    // Qualifying spreads each window's matches across a multi-day block, so club
    // fixtures must keep clear of the whole span rather than just the openers.
    let reserved_dates = if leads_into_world_cup || starts_qualifying || qualifying_running {
        ofm_core::national_team::international_window_span_dates(&window_dates)
    } else {
        window_dates.clone()
    };
    for competition in &mut game.competitions {
        // The World Cup and its qualifying own the reserved window — they are the
        // reason it is reserved — so shifting them off it would move the fixtures
        // we just scheduled there. Only club competitions step aside.
        if ofm_core::world_cup::is_world_cup_competition(competition)
            || ofm_core::world_cup::is_world_cup_qualifying(competition)
        {
            continue;
        }
        ofm_core::schedule::shift_fixtures_off_reserved_dates(competition, &reserved_dates);
    }
    ofm_core::schedule::append_south_american_preseason_friendlies(
        &mut game.competitions,
        &reserved_dates,
    );
    ofm_core::schedule::append_other_preseason_friendlies(&mut game.competitions, &reserved_dates);
}

pub(super) fn resolve_simulation_scope(
    game: &Game,
    team_id: &str,
    requested_region_ids: Option<Vec<String>>,
    requested_competition_ids: Option<Vec<String>>,
) -> Result<(Vec<String>, Vec<String>), String> {
    use std::collections::BTreeSet;

    let managed_team = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .ok_or("be.error.teamNotFound".to_string())?;

    let mut active_region_ids: BTreeSet<String> = requested_region_ids
        .unwrap_or_default()
        .into_iter()
        .collect();
    active_region_ids.insert(infer_team_region_id(managed_team));

    let mut active_competition_ids: BTreeSet<String> = requested_competition_ids
        .unwrap_or_default()
        .into_iter()
        .filter(|competition_id| {
            game.competitions
                .iter()
                .any(|competition| competition.id == *competition_id)
        })
        .collect();

    for competition in game.competitions.iter().filter(|competition| {
        competition
            .participant_ids
            .iter()
            .any(|participant_id| participant_id == team_id)
    }) {
        active_competition_ids.insert(competition.id.clone());
    }

    if active_competition_ids.is_empty() {
        for competition in &game.competitions {
            let required_regions = competition_required_region_ids(competition);
            if required_regions.is_empty()
                || required_regions
                    .iter()
                    .all(|region_id| active_region_ids.contains(region_id))
            {
                active_competition_ids.insert(competition.id.clone());
            }
        }
    }

    for competition in game
        .competitions
        .iter()
        .filter(|competition| active_competition_ids.contains(&competition.id))
    {
        for region_id in competition_required_region_ids(competition) {
            active_region_ids.insert(region_id);
        }
    }

    let mut resolved_region_ids: Vec<String> = active_region_ids.into_iter().collect();
    resolved_region_ids.sort();

    let mut resolved_competition_ids: Vec<String> = active_competition_ids.into_iter().collect();
    resolved_competition_ids.sort_by_key(|competition_id| {
        game.competitions
            .iter()
            .find(|competition| competition.id == *competition_id)
            .map(|competition| competition.priority)
            .unwrap_or(u32::MAX)
    });

    Ok((resolved_region_ids, resolved_competition_ids))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::game::start_date_for_year;
    use crate::commands::game::testkit::{make_bootstrap_test_game, manager_for, nation_team};
    use chrono::TimeZone;
    use domain::league::{CompetitionScope, CompetitionType, FixtureCompetition};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;

    #[test]
    fn world_cup_summer_career_stages_and_surfaces_the_tournament() {
        use ofm_core::world_cup::is_world_cup_competition;
        // A career opening in the 2026 World Cup summer.
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("team-1"),
            vec![nation_team("team-1", "ES", 500)],
            vec![],
            vec![],
            vec![],
        );
        // A non-empty active scope so staging registers the tournament as active.
        game.active_competition_ids = vec!["dummy".to_string()];

        ensure_international_windows(&mut game);

        let world_cup = game
            .competitions
            .iter()
            .find(|competition| is_world_cup_competition(competition))
            .expect("a World Cup summer career stages the tournament");
        assert!(
            game.active_competition_ids.contains(&world_cup.id),
            "the World Cup is surfaced in the active scope"
        );
        assert!(
            game.news
                .iter()
                .any(|article| article.id.starts_with("world_cup_kickoff_")),
            "a kickoff news article is published"
        );
    }

    #[test]
    fn rebuilding_competitions_leaves_the_world_cup_schedule_intact() {
        use ofm_core::world_cup::is_world_cup_competition;
        // A 2026 World Cup summer career, staged at the June anchor.
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("team-1"),
            vec![nation_team("team-1", "ES", 500)],
            vec![],
            vec![],
            vec![],
        );
        game.active_competition_ids = vec!["dummy".to_string()];
        ensure_international_windows(&mut game);

        // Capture the World Cup's id and fixture dates before any re-anchoring.
        let (wc_id, before): (String, Vec<String>) = {
            let world_cup = game
                .competitions
                .iter()
                .find(|competition| is_world_cup_competition(competition))
                .expect("the World Cup is staged");
            (
                world_cup.id.clone(),
                world_cup
                    .fixtures
                    .iter()
                    .map(|fixture| fixture.date.clone())
                    .collect(),
            )
        };
        assert!(
            !before.is_empty(),
            "the staged World Cup has fixtures to protect"
        );

        // Re-anchor competitions to a February management date — the Argentina
        // mid-season scenario that previously orphaned the cup's June schedule.
        let management_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
        rebuild_competitions_for_management_date(&mut game, management_date);

        let world_cup = game
            .competitions
            .iter()
            .find(|competition| competition.id == wc_id)
            .expect("the World Cup survives the re-anchor");
        let after: Vec<String> = world_cup
            .fixtures
            .iter()
            .map(|fixture| fixture.date.clone())
            .collect();
        assert_eq!(
            before, after,
            "the World Cup keeps its June schedule through a February re-anchor"
        );
        assert!(
            after
                .iter()
                .all(|date| date.starts_with("2026-06") || date.starts_with("2026-07")),
            "World Cup fixtures stay in the cup window, not pulled back to February"
        );
    }

    #[test]
    fn non_world_cup_year_career_stages_no_tournament() {
        use ofm_core::world_cup::is_world_cup_competition;
        let clock = GameClock::new(Utc.with_ymd_and_hms(2027, 7, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("team-1"),
            vec![nation_team("team-1", "ES", 500)],
            vec![],
            vec![],
            vec![],
        );

        ensure_international_windows(&mut game);

        assert!(
            !game.competitions.iter().any(is_world_cup_competition),
            "no World Cup is staged outside a cup summer"
        );
    }

    /// Characterization test: locks the STRUCTURE of the generated foundation
    /// world (kinds, scopes, regions, countries, priorities, participant and
    /// fixture counts, formats) so the Phase E "unify built-ins through the
    /// resolver" refactor can prove it preserves behavior (modulo ids).
    #[test]
    fn foundation_competitions_structure_is_stable() {
        // A 30-club nation (→ two divisions: 20 + 10), a 6-club nation (one
        // division), and a 1-club nation (skipped). All in one region, so the
        // continental field stays under four entrants and no continental cup
        // is created — keeping the structure fully deterministic.
        let mut teams = Vec::new();
        for index in 0..30 {
            teams.push(nation_team(
                &format!("esp-{index:02}"),
                "ESP",
                1000 - index as u32,
            ));
        }
        for index in 0..6 {
            teams.push(nation_team(
                &format!("fra-{index}"),
                "FRA",
                500 - index as u32,
            ));
        }
        teams.push(nation_team("and-0", "AND", 100));

        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let competitions = build_foundation_competitions(&game);

        type CompetitionSummary = (
            CompetitionType,
            CompetitionScope,
            Option<String>,
            Option<String>,
            usize,
            u32,
            CompetitionFormat,
        );

        let summary: Vec<CompetitionSummary> = competitions
            .iter()
            .map(|competition| {
                (
                    competition.kind.clone(),
                    competition.scope.clone(),
                    competition.region_id.clone(),
                    competition.country_id.clone(),
                    competition.participant_ids.len(),
                    competition.priority,
                    competition.rules.format.clone(),
                )
            })
            .collect();

        let europe = || Some("europe".to_string());
        assert_eq!(
            summary,
            vec![
                (
                    CompetitionType::League,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("ESP".to_string()),
                    20,
                    0,
                    CompetitionFormat::LeagueTable
                ),
                (
                    CompetitionType::League,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("ESP".to_string()),
                    10,
                    1,
                    CompetitionFormat::LeagueTable
                ),
                (
                    CompetitionType::Cup,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("ESP".to_string()),
                    30,
                    2,
                    CompetitionFormat::Knockout
                ),
                (
                    CompetitionType::League,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("FRA".to_string()),
                    6,
                    3,
                    CompetitionFormat::LeagueTable
                ),
                (
                    CompetitionType::Cup,
                    CompetitionScope::Domestic,
                    europe(),
                    Some("FRA".to_string()),
                    6,
                    4,
                    CompetitionFormat::Knockout
                ),
            ],
        );

        // League tables carry a full double round robin and a standings row per
        // club; the refactor must preserve both.
        let top_division = &competitions[0];
        assert_eq!(top_division.standings.len(), 20);
        assert_eq!(top_division.fixtures.len(), 20 * 19);
        assert_eq!(competitions[3].fixtures.len(), 6 * 5);

        // No continental cup for a single-region field.
        assert!(!competitions
            .iter()
            .any(|competition| competition.kind == CompetitionType::ContinentalClub));

        // Default continental berths: first division awards positions 1–4, the
        // cup awards its winner, the second division awards nothing.
        use domain::league::BerthRule;
        let top_division = &competitions[0];
        assert_eq!(top_division.berths.len(), 1);
        assert_eq!(top_division.berths[0].target, "continental-champions-cup");
        assert!(matches!(
            top_division.berths[0].rule,
            BerthRule::PositionRange { from: 1, to: 4 }
        ));
        assert!(
            competitions[1].berths.is_empty(),
            "second division awards no berth"
        );
        let cup = &competitions[2];
        assert!(matches!(
            cup.berths.first().map(|berth| &berth.rule),
            Some(BerthRule::CupWinner)
        ));
    }

    #[test]
    fn season_start_anchor_buffers_preseason_for_non_august_calendars() {
        // An Asian (February) club: the Season-Start clock should land a
        // pre-season buffer before the first competitive fixture — not on
        // matchday one — so the player gets a pre-season with playable
        // friendlies. (Regression for Asian/Oceanian leagues, which used to
        // re-anchor straight onto their opener.)
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "Japan".to_string(),
        );
        let mut team = domain::team::Team::new(
            "jp-1".to_string(),
            "Tokyo FC".to_string(),
            "TFC".to_string(),
            "JP".to_string(),
            "Tokyo".to_string(),
            "Stadium".to_string(),
            10_000,
        );
        team.football_nation = "JP".to_string();

        let mut game = Game::new(clock, manager, vec![team], vec![], vec![], vec![]);
        let mut league = League::new(
            "jp-league".to_string(),
            "JP League".to_string(),
            2026,
            &["jp-1".to_string()],
        );
        league.region_id = Some("asia".to_string());
        league.fixtures.push(domain::league::Fixture {
            id: "f1".to_string(),
            competition_id: "jp-league".to_string(),
            matchday: 1,
            date: "2026-02-07".to_string(),
            home_team_id: "jp-1".to_string(),
            away_team_id: "jp-2".to_string(),
            competition: FixtureCompetition::League,
            status: domain::league::FixtureStatus::Scheduled,
            result: None,
        });
        game.competitions = vec![league];

        let anchor =
            super::team_season_anchor(&game, "jp-1").expect("a non-August league re-anchors");
        // The first competitive fixture (Feb 7) minus the 30-day pre-season buffer.
        assert_eq!(anchor, Utc.with_ymd_and_hms(2026, 1, 8, 0, 0, 0).unwrap());
    }

    #[test]
    fn brazil_foundations_use_the_2026_calendar_and_regional_state_series() {
        let cities = [
            "São Paulo",
            "Rio",
            "Belo Horizonte",
            "Porto Alegre",
            "Salvador",
            "Recife",
            "Curitiba",
            "Fortaleza",
            "Goiânia",
            "Santos",
            "Campinas",
            "Belém",
            "Manaus",
            "Vitória",
            "Natal",
            "Florianópolis",
            "Cuiabá",
            "Maceió",
            "Bragantino",
            "Juiz de Fora",
        ];
        let mut teams: Vec<_> = (0..40)
            .map(|index| {
                let mut team = nation_team(&format!("br-{index}"), "BR", 1000 - index);
                team.city = cities[index as usize % cities.len()].to_string();
                team
            })
            .collect();
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap());
        let mut game = Game::new(
            clock,
            manager_for("br-0"),
            std::mem::take(&mut teams),
            vec![],
            vec![],
            vec![],
        );
        game.competitions = build_foundation_competitions(&game);
        ofm_core::schedule::append_south_american_preseason_friendlies(&mut game.competitions, &[]);

        let serie_a = game
            .competitions
            .iter()
            .find(|competition| competition.id == "br-d1")
            .unwrap();
        let serie_b = game
            .competitions
            .iter()
            .find(|competition| competition.id == "br-d2")
            .unwrap();
        assert_eq!(serie_a.season_start_day, 28);
        assert_eq!(serie_a.season_start_month, 1);
        assert_eq!(serie_b.season_start_day, 21);
        assert_eq!(serie_b.season_start_month, 3);
        assert!(serie_a
            .fixtures
            .iter()
            .any(|fixture| fixture.competition == FixtureCompetition::League
                && fixture.date == "2026-01-28"));
        assert!(serie_b
            .fixtures
            .iter()
            .any(|fixture| fixture.competition == FixtureCompetition::League
                && fixture.date == "2026-03-21"));
        let friendly_dates: Vec<&str> = serie_a
            .fixtures
            .iter()
            .filter(|fixture| fixture.competition == FixtureCompetition::Friendly)
            .map(|fixture| fixture.date.as_str())
            .collect();
        assert_eq!(
            friendly_dates
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>(),
            ["2025-12-21", "2025-12-28", "2026-01-04"]
                .into_iter()
                .collect()
        );

        let states: Vec<_> = game
            .competitions
            .iter()
            .filter(|competition| competition.id.starts_with("br-state-"))
            .collect();
        assert_eq!(states.len(), 4);
        assert!(states
            .iter()
            .all(|competition| !competition.rules.counts_in_season_flow
                && competition.rules.group_stage_legs == 1
                && competition.name_key.is_some()));
        for team in &game.teams {
            assert_eq!(
                states
                    .iter()
                    .filter(|competition| competition.participant_ids.contains(&team.id))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn management_date_rebuild_preserves_authored_competition_identity() {
        let teams = vec![
            nation_team("br-a", "BR", 500),
            nation_team("br-b", "BR", 400),
        ];
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let mut game = Game::new(clock, manager_for("br-a"), teams, vec![], vec![], vec![]);
        let mut authored = ofm_core::schedule::generate_league(
            "Authored Brazil Championship",
            2026,
            &["br-a".to_string(), "br-b".to_string()],
            Utc.with_ymd_and_hms(2026, 1, 28, 0, 0, 0).unwrap(),
        );
        authored.id = "authored-brasileirao".to_string();
        authored.country_id = Some("BR".to_string());
        authored.region_id = Some("south-america".to_string());
        authored.season_start_month = 1;
        authored.season_start_day = 28;
        game.competitions = vec![authored];

        let anchor = Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap();
        game.clock.start_date = anchor;
        game.clock.current_date = anchor;
        rebuild_competitions_for_management_date(&mut game, anchor);

        let competition = game
            .competitions
            .iter()
            .find(|competition| competition.id == "authored-brasileirao")
            .unwrap();
        assert_eq!(competition.season, 2026);
        assert!(competition
            .fixtures
            .iter()
            .any(|fixture| fixture.date == "2026-01-28"));
    }

    #[test]
    fn resolve_simulation_scope_auto_enables_required_regions_and_team_competitions() {
        let mut game = make_bootstrap_test_game();
        game.teams[0].football_nation = "BR".to_string();
        game.teams[1].football_nation = "GB".to_string();

        let mut domestic = League::new(
            "domestic-1".to_string(),
            "Brazil League".to_string(),
            2032,
            &["team1".to_string()],
        );
        domestic.region_id = Some("south-america".to_string());
        domestic.required_region_ids = vec!["south-america".to_string()];
        domestic.priority = 0;

        let mut continental = League::new(
            "continental-1".to_string(),
            "Continental Champions Cup".to_string(),
            2032,
            &["team1".to_string(), "team2".to_string()],
        );
        continental.scope = CompetitionScope::Continental;
        continental.required_region_ids = vec!["south-america".to_string(), "europe".to_string()];
        continental.priority = 1;

        game.competitions = vec![domestic.clone(), continental.clone()];

        let (active_regions, active_competitions) = resolve_simulation_scope(
            &game,
            "team1",
            Some(vec!["south-america".to_string()]),
            Some(vec![continental.id.clone()]),
        )
        .unwrap();

        assert_eq!(
            active_regions,
            vec!["europe".to_string(), "south-america".to_string()]
        );
        assert_eq!(
            active_competitions,
            vec![domestic.id.clone(), continental.id.clone()]
        );
    }

    #[test]
    fn resolve_simulation_scope_defaults_to_team_region_when_no_scope_is_provided() {
        let mut game = make_bootstrap_test_game();
        game.teams[0].football_nation = "BR".to_string();

        let mut domestic = League::new(
            "domestic-1".to_string(),
            "Brazil League".to_string(),
            2032,
            &["team1".to_string()],
        );
        domestic.region_id = Some("south-america".to_string());
        domestic.required_region_ids = vec!["south-america".to_string()];
        domestic.priority = 0;
        game.competitions = vec![domestic.clone()];

        let (active_regions, active_competitions) =
            resolve_simulation_scope(&game, "team1", None, None).unwrap();

        assert_eq!(active_regions, vec!["south-america".to_string()]);
        assert_eq!(active_competitions, vec![domestic.id.clone()]);
    }
}
