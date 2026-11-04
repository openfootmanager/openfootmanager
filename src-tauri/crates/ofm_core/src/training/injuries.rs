//! Training-ground injuries, at every club that trains today.

use crate::game::Game;
use crate::player_wear::{injury_multiplier_from_fitness, roll_training_injury};
use domain::player::Injury;
use rand::RngExt;
use std::collections::HashMap;

/// The day's chance that a session injures the player it falls on, before his fitness scales it.
const BASE_CHANCE: f64 = 1.0 / 50.0;

/// A player worth this much makes the news when he is hurt in training, wherever he plays.
const NOTABLE_MARKET_VALUE: u64 = 500_000;

/// Rolls the day's training injuries. At each club that trains today, one player not already injured
/// is picked, and his fitness scales a 2% chance that the session injures him. A club with a
/// fixture today, in any competition, did not train.
///
/// One rule for every club: this used to run for the user's club alone, inside the user's random
/// events, so no other club ever lost a player in training. Only the user's club is told, by an
/// inbox message. A notable player's injury makes the news at any club.
///
/// Each club rolls its own dice (keyed on the seed, the date and the club), so another club's
/// squad never changes what happens at this one. Runs after the day's matches, which is why it
/// asks [`super::teams_playing_on`] (date alone, any status), and after the day's injury recovery,
/// so an injury picked up today keeps every one of its days.
pub fn roll_training_injuries(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let injuries = todays_injuries(game, &today);
    for (player_index, injury) in injuries {
        injure(game, player_index, injury, &today);
    }
}

/// Which player each training club loses today, as an index into `game.players`, and the injury.
fn todays_injuries(game: &Game, today: &str) -> Vec<(usize, Injury)> {
    let playing = super::teams_playing_on(game, today);
    let mut fit_by_club: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, player) in game.players.iter().enumerate() {
        if player.injury.is_none()
            && let Some(club) = player.team_id.as_deref()
        {
            fit_by_club.entry(club).or_default().push(index);
        }
    }

    game.teams
        .iter()
        .filter(|team| !playing.contains(&team.id))
        .filter_map(|team| {
            let squad = fit_by_club.get(team.id.as_str())?;
            let mut rng = game.rng_today(&format!("training-injury-{}", team.id));
            let player_index = squad[rng.random_range(0..squad.len())];
            let chance =
                BASE_CHANCE * injury_multiplier_from_fitness(game.players[player_index].fitness);
            rng.random_bool(chance.min(1.0))
                .then(|| (player_index, roll_training_injury(&mut rng)))
        })
        .collect()
}

/// Gives the player his injury, tells the user when he is one of theirs, and puts it in the news
/// when he is notable: worth [`NOTABLE_MARKET_VALUE`] or in his club's starting XI.
fn injure(game: &mut Game, player_index: usize, injury: Injury, today: &str) {
    let player = &game.players[player_index];
    let player_id = player.id.clone();
    let player_name = player.match_name.clone();
    let club_id = player.team_id.clone().unwrap_or_default();
    let club = game.teams.iter().find(|team| team.id == club_id);
    let is_notable = player.market_value >= NOTABLE_MARKET_VALUE
        || club.is_some_and(|club| club.starting_xi_ids.contains(&player_id));
    let club_name = club.map_or_else(|| club_id.clone(), |club| club.name.clone());
    let days = injury.days_remaining;

    if game.manager.team_id.as_deref() == Some(club_id.as_str()) {
        let message_id = format!("training_injury_{player_id}_{today}");
        // Sent already: this day has been played once, and its injury with it.
        if game.emitted_events.contains(&message_id) {
            return;
        }
        let message = crate::random_events::training_injury_message(
            &message_id,
            &player_id,
            &player_name,
            &injury.name,
            days,
            today,
        );
        crate::inbox::emit(game, message);
    }
    game.players[player_index].injury = Some(injury);

    let news_id = format!("injury_news_{player_id}_{today}");
    if is_notable && !game.news.iter().any(|article| article.id == news_id) {
        let date = game.clock.current_date.to_rfc3339();
        game.news.push(crate::news::injury_news_article(
            &news_id,
            &player_id,
            &player_name,
            &club_id,
            &club_name,
            days,
            &date,
        ));
    }
}
