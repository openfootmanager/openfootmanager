//! World Cup inter-confederation playoff qualification and its two parallel finals.

use super::*;

/// Resolve a single playoff tie through the national-team knockout engine
/// (regulation → extra time → penalties); returns the advancing nation code.
fn play_playoff_tie(game: &mut Game, home: &str, away: &str, rng: &mut impl Rng) -> String {
    let home_id = national_team_id_for(game, home);
    let away_id = national_team_id_for(game, away);
    let (home_goals, away_goals, _, _, home_pens, away_pens) =
        crate::national_team::play_national_knockout_match(game, &home_id, &away_id, rng);
    let home_advances = if home_goals != away_goals {
        home_goals > away_goals
    } else {
        home_pens.unwrap_or(0) >= away_pens.unwrap_or(0)
    };
    if home_advances {
        home.to_string()
    } else {
        away.to_string()
    }
}

/// The inter-confederation playoff: `entrants` (ranking order) contest the final
/// [`INTER_CONFED_PLAYOFF_SPOTS`] berths. The top seeds bye to the finals; the
/// rest play a preliminary round; the final winners qualify.
pub(super) fn resolve_inter_confed_playoff(
    game: &mut Game,
    entrants: &[String],
    rng: &mut impl Rng,
) -> Vec<String> {
    let spots = INTER_CONFED_PLAYOFF_SPOTS;
    if entrants.len() <= spots {
        return entrants.to_vec();
    }
    let ranked = ranked_field(game, entrants);
    let seeds: Vec<String> = ranked.iter().take(spots).cloned().collect();
    let rest: Vec<String> = ranked.iter().skip(spots).cloned().collect();

    // Preliminary round: strongest of the rest against weakest.
    let mut prelim_winners: Vec<String> = Vec::new();
    let half = rest.len() / 2;
    for i in 0..half {
        prelim_winners.push(play_playoff_tie(
            game,
            &rest[i],
            &rest[rest.len() - 1 - i],
            rng,
        ));
    }
    if rest.len() % 2 == 1 {
        prelim_winners.push(rest[half].clone());
    }

    // Finals: each seed faces a preliminary winner.
    let mut qualifiers: Vec<String> = Vec::new();
    for (index, seed) in seeds.iter().enumerate() {
        match prelim_winners.get(index) {
            Some(opponent) => qualifiers.push(play_playoff_tie(game, seed, opponent, rng)),
            None => qualifiers.push(seed.clone()),
        }
    }
    qualifiers.truncate(spots);
    qualifiers
}

/// Announce the inter-confederation playoff qualifiers to the world.
pub(super) fn announce_inter_confed_playoff(game: &mut Game, year: u32, winners: &[String]) {
    if winners.is_empty() {
        return;
    }
    let news_id = format!("world_cup_playoff_{year}");
    if game.news.iter().any(|article| article.id == news_id) {
        return;
    }
    let mut params = std::collections::HashMap::new();
    params.insert("year".to_string(), year.to_string());
    let names: Vec<String> = winners
        .iter()
        .map(|code| nations::nation_display_name(code))
        .collect();
    params.insert("nations".to_string(), names.join(", "));
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.news.push(
        NewsArticle::new(
            news_id,
            String::new(),
            String::new(),
            String::new(),
            date,
            NewsCategory::Editorial,
        )
        .with_i18n(
            "be.news.worldCupPlayoff.headline",
            "be.news.worldCupPlayoff.body",
            "be.source.footballHerald",
            params,
        ),
    );
}

/// The playoff's two qualifiers (nation codes), once its finals round — two
/// parallel ties, no byes — has been played. `None` while it is undecided.
pub(super) fn playoff_winners_of(cup: &League) -> Option<Vec<String>> {
    if cup.knockout_rounds.len() < 2 {
        return None;
    }
    let finals = cup.knockout_rounds.last()?;
    if !finals.completed || !finals.bye_team_ids.is_empty() {
        return None;
    }
    let mut winners = Vec::new();
    for fixture_id in &finals.fixture_ids {
        let fixture = cup
            .fixtures
            .iter()
            .find(|fixture| &fixture.id == fixture_id)?;
        winners.push(nation_code_of_national_team(fixture.advancing_team_id()?));
    }
    (winners.len() == INTER_CONFED_PLAYOFF_SPOTS).then_some(winners)
}

/// The qualifiers of `year`'s inter-confederation playoff competition, when it
/// was staged and has been decided.
pub(super) fn decided_playoff_winners(game: &Game, year: u32) -> Option<Vec<String>> {
    let cup = game
        .competitions
        .iter()
        .find(|competition| is_world_cup_playoff(competition) && competition.season == year)?;
    playoff_winners_of(cup)
}

/// Advance the inter-confederation playoff once its current round is fully
/// played. Unlike a standard bracket it crowns no single champion: completing
/// the preliminaries seeds two parallel finals — each bye seed against a
/// preliminary winner, the top seed meeting the winner of the weaker-seeded
/// tie — and completing the finals simply decides the two berths, with no
/// further round.
pub(super) fn advance_world_cup_playoff(cup: &mut League) {
    let rounds_before = cup.knockout_rounds.len();
    crate::schedule::advance_knockout_round_with(cup, |winners, seeds| {
        if seeds.is_empty() {
            return None; // the finals: berths decided, the bracket ends here
        }
        let mut order: Vec<String> = Vec::new();
        for (seed, winner) in seeds.iter().zip(winners.iter().rev()) {
            order.push(seed.clone());
            order.push(winner.clone());
        }
        (!order.is_empty()).then_some(order)
    });
    // The freshly seeded round is the pair of parallel finals.
    if cup.knockout_rounds.len() > rounds_before
        && let Some(finals) = cup.knockout_rounds.last_mut()
    {
        finals.name = "Final".to_string();
    }
}

/// Stage the inter-confederation playoff as a real, visible knockout in the
/// campaign's final international window once every qualifying group has
/// finished. Only a full campaign gets one — it completes in the March window
/// with the June window free — and only in the real six-entrant shape (two
/// seeds bye to the finals, four contest the preliminaries); a compressed
/// campaign, or a degenerate world, resolves its playoff synthetically at the
/// rollover instead.
pub(crate) fn stage_world_cup_playoff_if_ready(
    game: &mut Game,
    today: &str,
    published: &std::collections::BTreeSet<(String, String)>,
) {
    let Some((year, playoff_date, ranked)) = playoff_field_if_ready(game, today) else {
        return;
    };
    let cup = build_playoff(game, year, &playoff_date, &ranked);
    super::publication::publish_with_history(game, cup, published);
    announce_playoff_draw(game, year, &ranked, today);
}

fn playoff_field_if_ready(game: &Game, today: &str) -> Option<(u32, String, Vec<String>)> {
    // The cheap gates run first: this is called on every day that simulates a
    // qualifying fixture, and sorting every group's standings below is only
    // worth it once the campaign has actually finished.
    if game.competitions.iter().any(is_world_cup_playoff) {
        return None;
    }
    let year = game
        .competitions
        .iter()
        .find(|competition| is_world_cup_qualifying(competition))
        .map(|competition| competition.season)?;
    let all_played = game
        .competitions
        .iter()
        .filter(|competition| is_world_cup_qualifying(competition))
        .all(|competition| {
            competition
                .fixtures
                .iter()
                .all(|fixture| fixture.status != FixtureStatus::Scheduled)
        });
    if !all_played {
        return None;
    }
    // The playoff owns the campaign's last window (June of the cup year); a
    // campaign still running by then has no room for a scheduled bracket.
    let playoff_date = season_window_dates(year as i32 - 1).pop()?;
    if today >= playoff_date.as_str() {
        return None;
    }
    let (year, groups_by_confed, entrants_by_confed) = qualifying_groups_by_confederation(game)?;
    // Quotas and the playoff only exist at 48-team scale.
    if entrants_by_confed.values().sum::<usize>() < FORMAT_48.field {
        return None;
    }

    let host = host_for_year(game, year as i32);
    let outcome = split_qualifying_outcome(
        game,
        &groups_by_confed,
        &entrants_by_confed,
        host.as_deref(),
    );
    let ranked = ranked_field(game, &outcome.playoff_entrants);
    if ranked.len() != INTER_CONFED_PLAYOFF_SPOTS + 4 {
        return None;
    }
    Some((year, playoff_date, ranked))
}

fn build_playoff(game: &Game, year: u32, playoff_date: &str, ranked: &[String]) -> League {
    let entrant_ids: Vec<String> = {
        let seeds = &ranked[..INTER_CONFED_PLAYOFF_SPOTS];
        let rest = &ranked[INTER_CONFED_PLAYOFF_SPOTS..];
        // Seeds first (they bye to the finals), then the preliminaries pair
        // the strongest of the rest against the weakest.
        let order = [
            seeds[0].clone(),
            seeds[1].clone(),
            rest[0].clone(),
            rest[3].clone(),
            rest[1].clone(),
            rest[2].clone(),
        ];
        order
            .iter()
            .map(|code| national_team_id_for(game, code))
            .collect()
    };

    let competition_id = format!("{PLAYOFF_COMPETITION_PREFIX}{year}");
    let mut cup = League::new(
        competition_id,
        format!("World Cup Play-off {year}"),
        year,
        &[],
    );
    cup.kind = CompetitionType::InternationalNation;
    cup.scope = CompetitionScope::International;
    cup.rules.format = CompetitionFormat::Knockout;
    cup.rules.knockout_round_gap_days = KNOCKOUT_GAP_DAYS;
    cup.rules.knockout_matches_per_day = 2;
    cup.standings.clear();
    cup.priority = 9_500;
    cup.name_key = Some("tournaments.competitions.worldCupPlayoff".to_string());
    cup.participant_ids = entrant_ids.clone();

    let prelim_start = crate::schedule::date_str_to_utc(playoff_date).unwrap_or_else(Utc::now);
    crate::schedule::seed_knockout_round(
        &mut cup,
        &entrant_ids,
        prelim_start,
        FixtureCompetition::InternationalNation,
    );
    if let Some(preliminaries) = cup.knockout_rounds.last_mut() {
        preliminaries.name = "Semifinal".to_string();
    }

    cup
}

fn announce_playoff_draw(game: &mut Game, year: u32, ranked: &[String], today: &str) {
    // News: the playoff line-up is set.
    let news_id = format!("world_cup_playoff_draw_{year}");
    if !game.news.iter().any(|article| article.id == news_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("year".to_string(), year.to_string());
        let names: Vec<String> = ranked
            .iter()
            .map(|code| nations::nation_display_name(code))
            .collect();
        params.insert("nations".to_string(), names.join(", "));
        game.news.push(
            NewsArticle::new(
                news_id,
                String::new(),
                String::new(),
                String::new(),
                today.to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupPlayoffDraw.headline",
                "be.news.worldCupPlayoffDraw.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }
}
