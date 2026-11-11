//! World Cup champion announcements and the next host decision.

use super::*;
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::world_history::{WorldCupChampionRecord, WorldCupHostRecord};

/// World Cup honors and host selection must never run for another national tournament.
pub(crate) fn after_matches(game: &mut Game, competition_index: usize, today: &str) {
    if !is_world_cup_competition(&game.competitions[competition_index]) {
        return;
    }
    let mut host_rng = game.rng_for(
        &format!("world-cup/host/{}", game.competitions[competition_index].id),
        today,
    );
    announce_champion_if_decided(game, competition_index, today, &mut host_rng);
    let competition = &game.competitions[competition_index];
    if is_world_cup_playoff(competition)
        && let Some(winners) = playoffs::playoff_winners_of(competition)
    {
        let year = competition.season;
        playoffs::announce_inter_confed_playoff(game, year, &winners);
    }
}

/// The tournament winner, once the final has been played.
pub fn world_cup_champion(competition: &League) -> Option<String> {
    let last_round = competition.knockout_rounds.last()?;
    if !last_round.completed || last_round.fixture_ids.len() != 1 {
        return None;
    }
    let final_fixture = competition
        .fixtures
        .iter()
        .find(|fixture| last_round.fixture_ids.contains(&fixture.id))?;
    let result = final_fixture.result.as_ref()?;
    Some(if result.advancing_is_home() {
        final_fixture.home_team_id.clone()
    } else {
        final_fixture.away_team_id.clone()
    })
}

fn announce_champion_if_decided(
    game: &mut Game,
    competition_index: usize,
    today: &str,
    rng: &mut impl Rng,
) {
    let competition = &game.competitions[competition_index];
    let Some(champion_id) = world_cup_champion(competition) else {
        return;
    };
    let year = competition.season;
    let msg_id = format!("world_cup_champion_{year}");
    if crate::inbox::already_emitted(game, &msg_id) {
        return;
    }

    let (nation, nation_code) = game
        .national_teams
        .iter()
        .find(|team| team.id == champion_id)
        .map(|team| (team.name.clone(), team.football_nation.clone()))
        .unwrap_or((champion_id.clone(), String::new()));

    // The game world's highest honour goes into the hall of fame.
    game.world_history
        .record_world_cup_champion(WorldCupChampionRecord {
            year,
            nation_code,
            nation_name: nation.clone(),
        });

    let mut params = std::collections::HashMap::new();
    params.insert("nation".to_string(), nation);
    params.insert("year".to_string(), year.to_string());

    let announcement_params = params.clone();
    crate::inbox::emit_once(game, &msg_id, || {
        InboxMessage::new(
            msg_id.clone(),
            String::new(),
            String::new(),
            String::new(),
            today.to_string(),
        )
        .with_category(MessageCategory::LeagueInfo)
        .with_priority(MessagePriority::High)
        .with_sender_role("")
        .with_i18n(
            "be.msg.worldCupChampion.subject",
            "be.msg.worldCupChampion.body",
            announcement_params,
        )
        .with_sender_i18n("be.sender.intlLiaison", "be.role.intlLiaison")
    });

    // Front-page news for everyone, participant or not.
    let news_id = format!("world_cup_champion_news_{year}");
    if !game.news.iter().any(|article| article.id == news_id) {
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
                "be.news.worldCupChampion.headline",
                "be.news.worldCupChampion.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }

    // With the champion crowned, the bid race for a future edition is decided.
    award_next_world_cup_host(game, year, today, rng);
}

/// After a World Cup ends, award the next edition whose host is not already
/// known (real-world hosts stand for their years). A handful of strong nations
/// that have not hosted recently form a shortlist; one is chosen at random.
/// Both the bid race and the award make the news, and the host is stored so it
/// auto-qualifies and is seeded into Pot 1 of that tournament's draw.
fn award_next_world_cup_host(game: &mut Game, played_year: u32, today: &str, rng: &mut impl Rng) {
    let next_year = played_year + 4;
    if host_for_year(game, next_year as i32).is_some() {
        return;
    }
    // Hosts used within the last ~24 years are off the table.
    let mut recent: std::collections::HashSet<String> = std::collections::HashSet::new();
    for year in next_year.saturating_sub(24)..next_year {
        if let Some(code) = host_for_year(game, year as i32) {
            recent.insert(code);
        }
    }
    let candidates: Vec<String> = nations::NATION_CATALOG
        .iter()
        .map(|nation| nation.code.to_string())
        .filter(|code| !recent.contains(code))
        .collect();
    let shortlist = ranked_field(game, &candidates);
    if shortlist.is_empty() {
        return;
    }
    let shortlist_len = shortlist.len().min(4);
    let chosen = shortlist[rng.random_range(0..shortlist_len)].clone();
    let nation_name = nations::nation_display_name(&chosen);

    game.world_history
        .record_world_cup_host(WorldCupHostRecord {
            year: next_year,
            nation_code: chosen,
            nation_name: nation_name.clone(),
        });

    let bid_id = format!("world_cup_host_bid_{next_year}");
    if !game.news.iter().any(|article| article.id == bid_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("count".to_string(), shortlist_len.to_string());
        params.insert("year".to_string(), next_year.to_string());
        game.news.push(
            NewsArticle::new(
                bid_id,
                String::new(),
                String::new(),
                String::new(),
                today.to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupHostBid.headline",
                "be.news.worldCupHostBid.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }

    let host_id = format!("world_cup_host_{next_year}");
    if !game.news.iter().any(|article| article.id == host_id) {
        let mut params = std::collections::HashMap::new();
        params.insert("nation".to_string(), nation_name);
        params.insert("year".to_string(), next_year.to_string());
        game.news.push(
            NewsArticle::new(
                host_id,
                String::new(),
                String::new(),
                String::new(),
                today.to_string(),
                NewsCategory::Editorial,
            )
            .with_i18n(
                "be.news.worldCupHostChosen.headline",
                "be.news.worldCupHostChosen.body",
                "be.source.footballHerald",
                params,
            ),
        );
    }
}
