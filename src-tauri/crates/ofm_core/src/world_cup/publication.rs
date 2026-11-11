//! Publish a settled World Cup calendar, preserving the distinction between new and shown dates.

use super::*;

pub(super) fn publish(game: &mut Game, competition: League) {
    let published = crate::national_team::published_tournament_fixtures(game);
    publish_with_history(game, competition, &published);
}

pub(super) fn publish_with_history(
    game: &mut Game,
    mut competition: League,
    published: &std::collections::BTreeSet<(String, String)>,
) {
    crate::calendar_identity::ensure_competition_calendar(&mut competition);
    let id = competition.id.clone();
    game.competitions.push(competition);
    crate::national_team::settle_drawn_tournaments(
        game,
        game.clock.current_date.date_naive(),
        published,
    );
    if !game.active_competition_ids.is_empty() {
        game.active_competition_ids.push(id);
    }
}

pub(super) struct QualifyingWindows {
    pub(super) full_campaign: bool,
    pub(super) dates: Vec<String>,
    pub(super) matches_per_window: usize,
    pub(super) slots: usize,
    pub(super) group_size: usize,
    pub(super) legs: u8,
}

pub(super) fn qualifying_windows(wc_year: i32, dates: &[String]) -> QualifyingWindows {
    // A campaign opening two years before the finals has two seasons of windows.
    let first_year = chrono::NaiveDate::parse_from_str(&dates[0], "%Y-%m-%d")
        .map(|date| date.year())
        .unwrap_or(wc_year - 1);
    let full_campaign = wc_year - first_year >= 2;
    let mut dates = dates.to_vec();
    let (matches_per_window, slots, group_size, legs) = if full_campaign {
        dates.extend(season_window_dates(first_year + 1));
        (
            MATCHDAYS_PER_WINDOW,
            full_campaign_slots(dates.len()),
            FULL_CAMPAIGN_GROUP_SIZE,
            2,
        )
    } else {
        (1, dates.len(), QUALIFYING_GROUP_SIZE, 1)
    };
    QualifyingWindows {
        full_campaign,
        dates,
        matches_per_window,
        slots,
        group_size,
        legs,
    }
}

pub(super) fn announce_qualifying(game: &mut Game, wc_year: i32, nations: usize, first_day: &str) {
    let news_id = format!("world_cup_qualifying_{wc_year}");
    if !game.news.iter().any(|article| article.id == news_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("year".to_string(), wc_year.to_string());
        params.insert("nations".to_string(), nations.to_string());
        game.news.push(
            NewsArticle::new(
                news_id,
                String::new(),
                String::new(),
                String::new(),
                first_day.to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupQualifying.headline",
                "be.news.worldCupQualifying.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }
}

pub(super) fn qualifying_competition(wc_year: i32) -> League {
    let competition_id = format!("{QUALIFYING_COMPETITION_PREFIX}{wc_year}");
    let mut competition = League::new(
        competition_id.clone(),
        format!("World Cup Qualifying {wc_year}"),
        wc_year as u32,
        &[],
    );
    competition.kind = CompetitionType::InternationalNation;
    competition.scope = CompetitionScope::International;
    competition.rules.format = CompetitionFormat::LeagueTable;
    competition.standings.clear();
    competition.priority = 9_000;
    competition.name_key = Some("tournaments.competitions.worldCupQualifying".to_string());

    competition
}
