//! Training-ground injuries, at every club that trains today.

use crate::game::Game;
use crate::player_wear::{injury_multiplier_from_fitness, roll_training_injury};
use domain::player::{Injury, Player};
use domain::team::Team;
use rand::RngExt;
use std::collections::HashMap;

/// The day's chance that a session injures the player it falls on, before his fitness scales it.
const BASE_CHANCE: f64 = 1.0 / 50.0;

/// How many of a club's most valuable players are notable, wherever they play in its side
/// ([`is_notable`]).
const NOTABLE_MOST_VALUABLE: usize = 3;

/// Rolls the day's training injuries. At each club that trains today, one player not already injured
/// is picked, and his fitness scales a 2% chance that the session injures him. A club with a
/// fixture today, in any competition, did not train.
///
/// One rule for every club: this used to run for the user's club alone, inside the user's random
/// events, so no other club ever lost a player in training. Only the user's club is told, by an
/// inbox message, and only a notable player in the user's league makes the news
/// ([`injury_makes_the_news`]).
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

/// Gives the player his injury, tells the user when he is one of theirs, and puts it in the news when
/// [`injury_makes_the_news`].
fn injure(game: &mut Game, player_index: usize, injury: Injury, today: &str) {
    let player = &game.players[player_index];
    let player_id = player.id.clone();
    let player_name = player.match_name.clone();
    let club_id = player.team_id.clone().unwrap_or_default();
    let club = game.teams.iter().find(|team| team.id == club_id);
    let makes_the_news = club.is_some_and(|club| injury_makes_the_news(game, player, club));
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
    if makes_the_news && !game.news.iter().any(|article| article.id == news_id) {
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

/// Whether `player`'s training injury makes the news: he is [notable](is_notable) and the user
/// follows his club, the user's own or one in the user's league. Anyone else hurt is injured
/// silently: the news covers the division the user plays in (maintainer, 4 Nov 2026), as the season
/// preview does.
fn injury_makes_the_news(game: &Game, player: &Player, club: &Team) -> bool {
    user_follows(game, &club.id) && is_notable(game, player, club)
}

/// Whether `player` is notable at `club` (maintainer, 4 Nov 2026): he is in the side the club would
/// field (`turn::squad::match_day_selection`, the same pick kick-off makes), or among its
/// [`NOTABLE_MOST_VALUABLE`] most valuable players. Equal values go to the
/// lower id, so exactly that many are. It used to be any player worth 500,000, which in a top
/// division is nearly everyone: a training-injury story every two or three days.
fn is_notable(game: &Game, player: &Player, club: &Team) -> bool {
    let squad = crate::match_day_eligibility::club_players(&game.players, &club.id);
    let fielded = crate::turn::squad::match_day_selection(game, &club.id, &squad, None);
    if fielded.eleven.iter().any(|starter| starter.id == player.id) {
        return true;
    }
    let ranked_ahead = |other: &Player| {
        other.market_value > player.market_value
            || (other.market_value == player.market_value && other.id < player.id)
    };
    let more_valuable = squad
        .iter()
        .copied()
        .filter(|other| ranked_ahead(other))
        .count();
    more_valuable < NOTABLE_MOST_VALUABLE
}

/// Whether the user follows `club_id`: the user's club, or a club of the division
/// [`crate::end_of_season::user_division`] gives, which for a manager out of work is the league the
/// game mirrors, as for the season preview (#1220).
fn user_follows(game: &Game, club_id: &str) -> bool {
    let user_club = game.manager.team_id.as_deref();
    user_club == Some(club_id)
        || crate::end_of_season::user_division(game, user_club.unwrap_or_default()).is_some_and(
            |division| {
                division.participant_ids.iter().any(|id| id == club_id)
                    || division
                        .standings
                        .iter()
                        .any(|standing| standing.team_id == club_id)
            },
        )
}

#[cfg(test)]
mod tests {
    use super::is_notable;
    use crate::clock::GameClock;
    use crate::game::Game;
    use crate::player_rating::formation_slots;
    use crate::test_support::uniform_attributes;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, Position};
    use domain::team::Team;

    fn club(id: &str) -> Team {
        let mut team = Team::new(
            id.into(),
            id.into(),
            id.into(),
            "ENG".into(),
            "Town".into(),
            "Ground".into(),
            10_000,
        );
        team.formation = "4-4-2".into();
        team
    }

    fn player(id: &str, club_id: &str, position: Position, rating: u8, value: u64) -> Player {
        let mut player = Player::new(
            id.into(),
            id.into(),
            id.into(),
            "1995-01-01".into(),
            "ENG".into(),
            position,
            uniform_attributes(rating),
        );
        player.team_id = Some(club_id.into());
        player.ovr = rating;
        player.market_value = value;
        player
    }

    /// Eleven naturals for the 4-4-2's slots at `club_id`, rated 70 and worth `value`: xi0..xi10.
    fn first_eleven(club_id: &str, value: u64) -> Vec<Player> {
        formation_slots("4-4-2")
            .into_iter()
            .enumerate()
            .map(|(index, slot)| player(&format!("xi{index}"), club_id, slot, 70, value))
            .collect()
    }

    /// A reserve at `club_id`, rated 40 so no club fields him over its 70s, worth `value`.
    fn reserve(club_id: &str, value: u64) -> Player {
        player("reserve", club_id, Position::Midfielder, 40, value)
    }

    /// A game whose user manages club "user", with `teams` and `players`.
    fn game(teams: Vec<Team>, players: Vec<Player>) -> Game {
        let mut manager = Manager::new(
            "mgr".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        manager.hire("user".into());
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap());
        Game::new(clock, manager, teams, players, vec![], vec![])
    }

    fn notable(game: &Game, player_id: &str, club_id: &str) -> bool {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        let club = game.teams.iter().find(|t| t.id == club_id).unwrap();
        is_notable(game, player, club)
    }

    /// Given the user's club fielding eleven 70s worth nothing, and a reserve worth nothing ahead
    ///      of them by id
    /// When a starter low in the order of value, xi5, is weighed
    /// Then he is notable: a player in the side his club would field is, whatever his value
    #[test]
    fn a_player_in_the_side_his_club_would_field_is_notable_whatever_his_value() {
        let mut players = first_eleven("user", 0);
        players.push(reserve("user", 0));
        let game = game(vec![club("user")], players);

        assert!(notable(&game, "xi5", "user"));
    }

    /// Given a reserve the club would not field, worth 1,000,000 against its starters' 100,000
    /// When he is weighed
    /// Then he is notable: he is among the club's three most valuable
    #[test]
    fn a_reserve_among_his_clubs_three_most_valuable_is_notable() {
        let mut players = first_eleven("user", 100_000);
        players.push(reserve("user", 1_000_000));
        let game = game(vec![club("user")], players);

        assert!(notable(&game, "reserve", "user"));
    }

    /// Given a reserve the club would not field, worth 50,000 against its starters' 100,000
    /// When he is weighed
    /// Then he is not notable: at least three teammates are worth more, and he does not play
    #[test]
    fn a_reserve_outside_his_clubs_three_most_valuable_is_not_notable() {
        let mut players = first_eleven("user", 100_000);
        players.push(reserve("user", 50_000));
        let game = game(vec![club("user")], players);

        assert!(!notable(&game, "reserve", "user"));
    }

    /// Given an AI club whose saved XI names its reserve, rated 40, while it would field its 70s,
    ///      the reserve worth less than any of them
    /// When the reserve is weighed
    /// Then he is not notable: what counts is the side the club would field, and an AI club picks
    ///      its own, not its saved XI
    #[test]
    fn an_ai_clubs_saved_eleven_does_not_make_a_reserve_notable() {
        let mut ai = club("ai");
        ai.starting_xi_ids = vec!["reserve".into()];
        let mut players = first_eleven("ai", 100_000);
        players.push(reserve("ai", 50_000));
        let game = game(vec![club("user"), ai], players);

        assert!(!notable(&game, "reserve", "ai"));
    }

    /// Given four reserves worth the same, none of whom the club would field
    /// When each is weighed
    /// Then the three with the lowest ids are notable and the fourth is not: equal values go to the
    ///      lower id, so exactly three are
    #[test]
    fn equal_values_go_to_the_lower_id() {
        let mut players = first_eleven("user", 0);
        for id in ["r1", "r2", "r3", "r4"] {
            players.push(player(id, "user", Position::Midfielder, 40, 500_000));
        }
        let game = game(vec![club("user")], players);

        let notable_reserves: Vec<&str> = ["r1", "r2", "r3", "r4"]
            .into_iter()
            .filter(|id| notable(&game, id, "user"))
            .collect();
        assert_eq!(notable_reserves, vec!["r1", "r2", "r3"]);
    }
}
