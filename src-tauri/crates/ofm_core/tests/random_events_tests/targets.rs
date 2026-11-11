//! Who a random event lands on: the user's own fit players and his own club's matches, never
//! another club's. Each test rolls one day over many seeds, because the day's dice are keyed on
//! the seed and the date.

use super::*;

const SEEDS: u64 = 2000;

fn seeded(seed: u64) -> Game {
    let mut game = make_game();
    game.seed = seed;
    game
}

/// Each seed's day from `make`, rolled once.
fn days_from(make: impl Fn(u64) -> Game) -> Vec<Game> {
    (0..SEEDS)
        .map(|seed| {
            let mut game = make(seed);
            check_random_events(&mut game);
            game
        })
        .collect()
}

fn message<'a>(game: &'a Game, prefix: &str) -> Option<&'a InboxMessage> {
    game.messages.iter().find(|m| m.id.starts_with(prefix))
}

/// The player each day's message from `prefix` is about, over the days that sent one.
fn players_named(days: &[Game], prefix: &str) -> Vec<String> {
    days.iter()
        .filter_map(|game| message(game, prefix)?.context.player_id.clone())
        .collect()
}

/// The player each day's international call-up went to, over the days that sent one.
fn called_up(days: &[Game]) -> Vec<String> {
    days.iter()
        .filter_map(|game| {
            let rest = message(game, "intl_callup_")?
                .id
                .strip_prefix("intl_callup_")?;
            rest.rsplit_once('_').map(|(player, _)| player.to_string())
        })
        .collect()
}

/// Every player but p10 out injured.
fn ten_injured(game: &mut Game) {
    for player in game.players.iter_mut().filter(|p| p.id != "p10") {
        player.injury = Some(domain::player::Injury {
            name: "Existing injury".to_string(),
            days_remaining: 5,
        });
    }
}

/// p0 alone at the user's club, the other ten at another one.
fn ten_at_another_club(game: &mut Game) {
    for player in game.players.iter_mut().filter(|p| p.id != "p0") {
        player.team_id = Some("team2".to_string());
    }
}

fn distinct(players: &[String]) -> Vec<&str> {
    let mut names: Vec<&str> = players.iter().map(String::as_str).collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Given the squad at morale 70 and no match to play, over two thousand seeds
/// When the press writes a story about a player
/// Then his morale moves and nobody else's does
#[test]
fn a_media_story_moves_only_its_players_morale() {
    let days = days_from(seeded);
    let stories: Vec<(&Game, String)> = days
        .iter()
        .filter_map(|game| Some((game, message(game, "media_")?.context.player_id.clone()?)))
        .collect();

    assert!(!stories.is_empty(), "no media story in {SEEDS} seeds");
    for (game, player) in stories {
        let moved: Vec<&str> = game
            .players
            .iter()
            .filter(|p| p.morale != 70)
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(moved, vec![player.as_str()], "seed {}", game.seed);
    }
}

/// Given only a scheduled match between two other clubs, three days away, over two thousand seeds
/// When the day's random events roll
/// Then nobody is called up: a call-up needs the user's own match
#[test]
fn another_clubs_match_brings_no_international_callups() {
    let days = days_from(|seed| {
        let mut game = seeded(seed);
        let date = date_in(&game, 3);
        game.league = Some(league_of(vec![fixture(
            "theirs",
            date,
            "team2",
            "team3",
            FixtureStatus::Scheduled,
        )]));
        game
    });

    assert_eq!(called_up(&days), Vec::<String>::new());
}

/// Given ten of the eleven injured and the user's match in three days, over two thousand seeds
/// When a player is called up
/// Then it is always the one fit player, p10
#[test]
fn injured_players_are_never_called_up() {
    let days = days_from(|seed| {
        let mut game = seeded(seed);
        a_match_in_three_days(&mut game);
        ten_injured(&mut game);
        game
    });

    let called = called_up(&days);
    assert!(!called.is_empty(), "no call-up in {SEEDS} seeds");
    assert_eq!(distinct(&called), vec!["p10"]);
}

/// Given p0 alone at the user's club, ten players at another, and the user's match in three days,
///       over two thousand seeds
/// When a player is called up
/// Then it is always p0
#[test]
fn another_clubs_players_are_never_reported_called_up() {
    let days = days_from(|seed| {
        let mut game = seeded(seed);
        a_match_in_three_days(&mut game);
        ten_at_another_club(&mut game);
        game
    });

    let called = called_up(&days);
    assert!(!called.is_empty(), "no call-up in {SEEDS} seeds");
    assert_eq!(distinct(&called), vec!["p0"]);
}

/// Given ten of the eleven injured, over two thousand seeds
/// When a rival is said to want a player
/// Then it is always the one fit player, p10
#[test]
fn rivals_are_never_said_to_want_an_injured_player() {
    let days = days_from(|seed| {
        let mut game = seeded(seed);
        ten_injured(&mut game);
        game
    });

    let wanted = players_named(&days, "rival_interest_");
    assert!(!wanted.is_empty(), "no rival interest in {SEEDS} seeds");
    assert_eq!(distinct(&wanted), vec!["p10"]);
}

/// Given p0 alone at the user's club and ten players at another, over two thousand seeds
/// When a rival is said to want a player
/// Then it is always p0
#[test]
fn rivals_are_never_said_to_want_another_clubs_player() {
    let days = days_from(|seed| {
        let mut game = seeded(seed);
        ten_at_another_club(&mut game);
        game
    });

    let wanted = players_named(&days, "rival_interest_");
    assert!(!wanted.is_empty(), "no rival interest in {SEEDS} seeds");
    assert_eq!(distinct(&wanted), vec!["p0"]);
}

fn played(id: &str, date: &str, home: &str, away: &str, home_goals: u8, away_goals: u8) -> Fixture {
    Fixture {
        result: Some(MatchResult {
            home_goals,
            away_goals,
            ..Default::default()
        }),
        ..fixture(id, date.to_string(), home, away, FixtureStatus::Completed)
    }
}

/// Given the user's last three results all defeats, then a later away win between two other clubs
/// When the day's random events roll
/// Then the board still writes about the three defeats
#[test]
fn other_clubs_results_do_not_hide_the_users_defeats_from_the_board() {
    let mut game = make_game();
    game.league = Some(league_of(vec![
        played("f1", "2025-06-01", "team1", "opp1", 0, 2),
        played("f2", "2025-06-05", "opp2", "team1", 3, 1),
        played("f3", "2025-06-10", "team1", "opp3", 0, 1),
        played("f4", "2025-06-12", "opp4", "opp5", 0, 2),
    ]));

    check_random_events(&mut game);

    let board: Vec<&str> = game
        .messages
        .iter()
        .filter(|m| m.id.starts_with("board_confidence_"))
        .map(|m| m.id.as_str())
        .collect();
    assert_eq!(board, vec!["board_confidence_2025-06-10"]);
}
