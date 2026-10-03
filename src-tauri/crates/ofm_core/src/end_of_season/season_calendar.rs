use crate::clock::GameClock;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use domain::league::League;

/// A completed calendar edition advances once, regardless of how late the
/// user's rollover runs. Delayed fixtures must not redefine its season label.
pub(super) fn next_edition(
    competition: &League,
    clock: &GameClock,
    earliest_start: DateTime<Utc>,
) -> (u32, DateTime<Utc>) {
    let mut next_season = competition.season.saturating_add(1);
    // Older saves numbered seasons from the career's start instead of using
    // calendar years. Preserve their existing normalization, without treating
    // a delayed calendar edition as a legacy counter based on its fixture dates.
    let career_seasons = (clock.current_date.year() - clock.start_date.year()).max(0) as u32 + 1;
    if competition.season <= career_seasons {
        let calendar_start = crate::generator::next_season_start(
            earliest_start,
            competition.season_start_month,
            competition.season_start_day,
        );
        next_season = next_season.max(calendar_start.year().max(0) as u32);
    }
    let year_start = i32::try_from(next_season)
        .ok()
        .and_then(|year| NaiveDate::from_ymd_opt(year, 1, 1))
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date| date.and_utc())
        .unwrap_or(earliest_start);
    let configured_start = crate::generator::next_season_start(
        year_start,
        competition.season_start_month,
        competition.season_start_day,
    );
    // Keep the shared four-week break; never create an edition in the past.
    (next_season, configured_start.max(earliest_start))
}

#[cfg(test)]
mod tests {
    use crate::clock::GameClock;
    use crate::end_of_season::advance_to_next_season;
    use crate::game::Game;
    use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
    use domain::league::{
        CompetitionScope, CompetitionType, FixtureCompetition, FixtureStatus, League, MatchResult,
    };
    use domain::manager::Manager;
    use domain::team::Team;

    fn at(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap()
    }

    fn division(id: &str, country: &str, start: DateTime<Utc>, count: usize) -> League {
        let ids: Vec<_> = (0..count).map(|i| format!("{id}-{i}")).collect();
        let mut league = crate::schedule::generate_league(id, start.year() as u32, &ids, start);
        league.id = id.into();
        league.country_id = Some(country.into());
        league.region_id = Some(
            if country == "AR" {
                "south-america"
            } else {
                "europe"
            }
            .into(),
        );
        league.scope = CompetitionScope::Domestic;
        league.season_start_month = start.month() as u8;
        league.season_start_day = start.day() as u8;
        for fixture in &mut league.fixtures {
            fixture.competition_id = id.into();
        }
        complete(&mut league);
        league
    }

    fn complete(league: &mut League) {
        for fixture in &mut league.fixtures {
            fixture.status = FixtureStatus::Completed;
            fixture.result = Some(MatchResult {
                home_goals: 1,
                away_goals: 0,
                ..Default::default()
            });
        }
    }

    fn add(game: &mut Game, league: League) {
        for id in &league.participant_ids {
            if !game.teams.iter().any(|team| &team.id == id) {
                let mut team = Team::new(
                    id.clone(),
                    id.clone(),
                    "TST".into(),
                    "England".into(),
                    "City".into(),
                    "Ground".into(),
                    1000,
                );
                team.football_nation = league.country_id.clone().unwrap_or_default();
                game.teams.push(team);
            }
        }
        game.competitions.push(league);
    }

    fn game(now: DateTime<Utc>, user_start: DateTime<Utc>, country: &str) -> Game {
        let mut manager = Manager::new(
            "manager".into(),
            "First".into(),
            "Last".into(),
            "1980-01-01".into(),
            country.into(),
        );
        manager.hire("user-0".into());
        manager.satisfaction = 80;
        let mut game = Game::new(GameClock::new(now), manager, vec![], vec![], vec![], vec![]);
        add(&mut game, division("user", country, user_start, 2));
        game.sync_legacy_league();
        game
    }

    fn english_game(now: DateTime<Utc>) -> Game {
        game(now, at(now.year() - 1, 8, 1), "ENG")
    }

    fn find<'a>(game: &'a Game, id: &str) -> &'a League {
        game.competitions.iter().find(|c| c.id == id).unwrap()
    }

    fn assert_renewed(game: &Game, id: &str, season: u32, start: DateTime<Utc>) {
        let league = find(game, id);
        assert_eq!(league.season, season, "{id} advances one edition");
        let fixtures: Vec<_> = league
            .fixtures
            .iter()
            .filter(|f| f.competition != FixtureCompetition::Friendly)
            .collect();
        assert!(!fixtures.is_empty());
        assert!(fixtures.iter().all(|f| f.status == FixtureStatus::Scheduled
            && f.result.is_none()
            && f.date >= game.clock.current_date.format("%Y-%m-%d").to_string()));
        assert_eq!(
            fixtures.iter().map(|f| &f.date).min().unwrap(),
            &start.format("%Y-%m-%d").to_string()
        );
        assert!(league.standings.iter().all(|row| row.played == 0));
    }

    #[test]
    fn overdue_foreign_league_advances_one_edition() {
        let now = at(2034, 4, 18);
        let mut game = english_game(now);
        let division = division("ar-d1-apertura", "AR", at(2033, 2, 1), 2);
        let participants = division.participant_ids.clone();
        add(&mut game, division);
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "ar-d1-apertura", 2034, now + Duration::days(28));
        let renewed = find(&game, "ar-d1-apertura");
        assert_eq!(renewed.participant_ids, participants);
        assert_eq!(
            (renewed.season_start_month, renewed.season_start_day),
            (2, 1)
        );
    }

    #[test]
    fn overdue_foreign_knockout_advances_one_edition() {
        let now = at(2034, 4, 18);
        let mut game = english_game(now);
        let mut cup = division("ar-cup", "AR", at(2033, 2, 1), 2);
        cup.kind = CompetitionType::Cup;
        cup.rules.format = domain::league::CompetitionFormat::Knockout;
        crate::schedule::regenerate_knockout_for_season(&mut cup, 2033, at(2033, 2, 1));
        complete(&mut cup);
        add(&mut game, cup);
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "ar-cup", 2034, now + Duration::days(28));
        assert_eq!(find(&game, "ar-cup").knockout_rounds.len(), 1);
    }

    #[test]
    fn overdue_group_knockout_advances_one_edition() {
        let now = at(2034, 4, 18);
        let mut game = english_game(now);
        let mut cup = division("groups", "AR", at(2033, 2, 1), 4);
        cup.kind = CompetitionType::Cup;
        cup.rules.format = domain::league::CompetitionFormat::GroupAndKnockout;
        crate::group_stage::regenerate_for_season(&mut cup, 2033, at(2033, 2, 1));
        complete(&mut cup);
        add(&mut game, cup);
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "groups", 2034, now + Duration::days(28));
        assert!(
            find(&game, "groups")
                .groups
                .iter()
                .flat_map(|group| &group.standings)
                .all(|row| row.played == 0)
        );
    }

    #[test]
    fn foreign_renewal_keeps_advancing_after_an_on_time_rollover() {
        let mut game = english_game(at(2035, 1, 1));
        add(
            &mut game,
            division("ar-d1-apertura", "AR", at(2034, 2, 1), 2),
        );
        for (now, expected) in [
            (at(2035, 1, 1), 2035),
            (at(2036, 4, 18), 2036),
            (at(2037, 4, 18), 2037),
        ] {
            game.clock.current_date = now;
            for competition in &mut game.competitions {
                complete(competition);
            }
            advance_to_next_season(&mut game).unwrap();
            let start = at(expected as i32, 2, 1).max(now + Duration::days(28));
            assert_renewed(&game, "ar-d1-apertura", expected, start);
        }
    }

    #[test]
    fn southern_manager_renews_overdue_european_edition() {
        let now = at(2033, 11, 17);
        let mut game = game(now, at(2033, 2, 1), "AR");
        add(&mut game, division("eng-d1", "ENG", at(2032, 8, 1), 2));
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "eng-d1", 2033, now + Duration::days(28));
    }

    #[test]
    fn future_opener_keeps_its_authored_calendar() {
        let mut game = english_game(at(2034, 4, 18));
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "user", 2034, at(2034, 8, 1));
    }

    #[test]
    fn unfinished_foreign_edition_is_retained() {
        let now = at(2034, 4, 18);
        let mut game = english_game(now);
        let mut foreign = division("ar-d1-apertura", "AR", at(2034, 2, 1), 8);
        for fixture in &mut foreign.fixtures {
            if fixture.date >= now.format("%Y-%m-%d").to_string() {
                fixture.status = FixtureStatus::Scheduled;
                fixture.result = None;
            }
        }
        // A retained real season already has its played preseason. This also
        // isolates #654 from #652's separate past-friendly bug on develop.
        let mut friendly = foreign.fixtures[0].clone();
        friendly.id = "played-preseason".into();
        friendly.competition = FixtureCompetition::Friendly;
        foreign.fixtures.insert(0, friendly);
        let before = serde_json::to_value(&foreign).unwrap();
        add(&mut game, foreign);
        advance_to_next_season(&mut game).unwrap();
        assert_eq!(
            serde_json::to_value(find(&game, "ar-d1-apertura")).unwrap(),
            before
        );
    }

    #[test]
    fn unfinished_user_season_refuses_without_mutation() {
        let mut game = english_game(at(2034, 4, 18));
        game.competitions[0].fixtures[1].status = FixtureStatus::Scheduled;
        game.competitions[0].fixtures[1].result = None;
        let before = serde_json::to_value(&game).unwrap();
        assert_eq!(
            advance_to_next_season(&mut game).unwrap_err(),
            "be.error.seasonNotComplete"
        );
        assert_eq!(serde_json::to_value(&game).unwrap(), before);
    }

    #[test]
    fn loaded_overdue_edition_renews_once() {
        let now = at(2034, 4, 18);
        let mut game = english_game(now);
        add(
            &mut game,
            division("ar-d1-apertura", "AR", at(2033, 2, 1), 2),
        );
        let mut loaded: Game = serde_json::from_slice(&serde_json::to_vec(&game).unwrap()).unwrap();
        advance_to_next_season(&mut loaded).unwrap();
        assert_renewed(&loaded, "ar-d1-apertura", 2034, now + Duration::days(28));
        let mut loaded: Game =
            serde_json::from_slice(&serde_json::to_vec(&loaded).unwrap()).unwrap();
        let before = serde_json::to_value(&loaded).unwrap();
        assert!(advance_to_next_season(&mut loaded).is_err());
        assert_eq!(serde_json::to_value(&loaded).unwrap(), before);
    }

    #[test]
    fn future_stamped_edition_keeps_dates_aligned() {
        let mut game = english_game(at(2034, 4, 18));
        let mut future = division("future", "AR", at(2033, 8, 1), 2);
        future.season = 2038;
        add(&mut game, future);
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "future", 2039, at(2039, 8, 1));
    }

    #[test]
    fn leap_day_opener_uses_existing_calendar_clamping() {
        let mut game = english_game(at(2034, 1, 1));
        let mut foreign = division("leap", "AR", at(2033, 2, 28), 2);
        foreign.season_start_day = 29;
        add(&mut game, foreign);
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "leap", 2034, at(2034, 2, 28));
    }

    #[test]
    fn legacy_counter_uses_its_fixture_calendar() {
        let mut game = english_game(at(2026, 5, 20));
        game.competitions[0].season = 1;
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "user", 2026, at(2026, 8, 1));
    }
    #[test]
    fn multi_year_delayed_save_advances_one_edition_each_time() {
        let mut game = english_game(at(2037, 4, 18));
        add(
            &mut game,
            division("ar-d1-apertura", "AR", at(2033, 2, 1), 2),
        );
        for (now, expected) in [(at(2037, 4, 18), 2034), (at(2038, 4, 18), 2035)] {
            game.clock.current_date = now;
            for competition in &mut game.competitions {
                complete(competition);
            }
            let mut loaded: Game =
                serde_json::from_slice(&serde_json::to_vec(&game).unwrap()).unwrap();
            advance_to_next_season(&mut loaded).unwrap();
            assert_renewed(
                &loaded,
                "ar-d1-apertura",
                expected,
                now + Duration::days(28),
            );
            game = loaded;
        }
    }

    #[test]
    fn later_legacy_counter_keeps_calendar_normalization() {
        let mut game = english_game(at(2030, 5, 20));
        game.clock.start_date = at(2026, 7, 1);
        game.competitions[0].season = 5;
        advance_to_next_season(&mut game).unwrap();
        assert_renewed(&game, "user", 2030, at(2030, 8, 1));
    }
}
