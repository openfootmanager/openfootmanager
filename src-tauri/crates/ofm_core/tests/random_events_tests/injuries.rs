//! Training injuries: the days they can happen on, the clubs they happen at, and which ones make
//! the news. Each test rolls one day over many seeds, because the day's dice are keyed on the seed
//! and the date: rolling the same day twice draws the same numbers.

use super::*;
use domain::league::CompetitionType;
use domain::news::{NewsArticle, NewsCategory};

const SEEDS: u64 = 1000;

/// A squad of three at fitness 1, so a 6% chance a day of a training injury, all worth nothing:
/// p0 on the bench, p1 and p2 in the starting XI. Another club, with its own XI, is listed first.
fn three_unfit_players(seed: u64) -> Game {
    let mut game = make_game();
    game.seed = seed;
    game.players.truncate(3);
    for player in &mut game.players {
        player.fitness = 1;
        player.market_value = 0;
    }
    game.teams[0].starting_xi_ids = vec!["p1".to_string(), "p2".to_string()];
    let mut other = make_team("team0", "Other Town");
    other.starting_xi_ids = vec!["q1".to_string()];
    game.teams.insert(0, other);
    game
}

/// Who was hurt in training today, if anyone.
fn injured_in_training(game: &Game) -> Option<String> {
    game.messages
        .iter()
        .find_map(|m| m.id.strip_prefix("training_injury_"))
        .and_then(|rest| rest.rsplit_once('_'))
        .map(|(player, _)| player.to_string())
}

/// Each seed's day from `make`, rolled once, kept when someone was hurt in training.
fn training_injuries(make: impl Fn(u64) -> Game) -> Vec<(String, Game)> {
    (0..SEEDS)
        .filter_map(|seed| {
            let mut game = make(seed);
            roll_training_injuries(&mut game);
            injured_in_training(&game).map(|player| (player, game))
        })
        .collect()
}

fn injury_story<'a>(game: &'a Game, player_id: &str) -> Option<&'a NewsArticle> {
    game.news.iter().find(|article| {
        article.id.starts_with("injury_news_")
            && article.player_ids.iter().any(|id| id == player_id)
    })
}

/// [`three_unfit_players`], with three unfit players worth nothing at Other Town too, the club the
/// user does not manage: q0 on its bench, q1 in its starting XI, q2 its third man.
fn two_clubs_of_unfit_players(seed: u64) -> Game {
    let mut game = three_unfit_players(seed);
    for index in 0..3 {
        let mut player = make_player(&format!("q{index}"), &format!("Q {index}"), "team0");
        player.fitness = 1;
        player.market_value = 0;
        game.players.push(player);
    }
    game
}

/// The players at `club` carrying an injury.
fn injured_at<'a>(game: &'a Game, club: &str) -> Vec<&'a str> {
    game.players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(club) && player.injury.is_some())
        .map(|player| player.id.as_str())
        .collect()
}

/// Each seed's day from `make`, rolled once, kept when someone at `club` was hurt.
fn days_with_an_injury_at(club: &str, make: impl Fn(u64) -> Game) -> Vec<Game> {
    (0..SEEDS)
        .map(|seed| {
            let mut game = make(seed);
            roll_training_injuries(&mut game);
            game
        })
        .filter(|game| !injured_at(game, club).is_empty())
        .collect()
}

fn with_a_match(game: &mut Game, days_away: i64) {
    let date = date_in(game, days_away);
    game.league = Some(league_of(vec![fixture(
        "match",
        date,
        "team1",
        "team2",
        FixtureStatus::Scheduled,
    )]));
}

/// Given a squad of three at fitness 1 and a scheduled match today, over a thousand seeds
/// When the day's random events roll
/// Then nobody is hurt in training
#[test]
fn a_match_today_means_no_training_injuries() {
    let injuries = training_injuries(|seed| {
        let mut game = three_unfit_players(seed);
        with_a_match(&mut game, 0);
        game
    });

    let seeds: Vec<u64> = injuries.iter().map(|(_, game)| game.seed).collect();
    assert_eq!(seeds, Vec::<u64>::new());
}

/// Given a squad of three at fitness 1 whose match today is already played (training runs after the
///      day's matches, so the fixture reads Completed), over a thousand seeds
/// When the day's training injuries roll
/// Then nobody is hurt in training: a club that played today did not train
#[test]
fn a_match_played_today_means_no_training_injuries() {
    let injuries = training_injuries(|seed| {
        let mut game = three_unfit_players(seed);
        let today = date_in(&game, 0);
        game.league = Some(league_of(vec![fixture(
            "match",
            today,
            "team1",
            "team2",
            FixtureStatus::Completed,
        )]));
        game
    });

    let seeds: Vec<u64> = injuries.iter().map(|(_, game)| game.seed).collect();
    assert_eq!(seeds, Vec::<u64>::new());
}

/// Given a squad of three at fitness 1 with a cup tie today and nothing in the league, over a
///      thousand seeds
/// When the day's training injuries roll
/// Then nobody is hurt in training: a cup tie is a match day like a league game
#[test]
fn a_cup_tie_today_means_no_training_injuries() {
    let injuries = training_injuries(|seed| {
        let mut game = three_unfit_players(seed);
        let today = date_in(&game, 0);
        let mut cup = league_of(vec![fixture(
            "tie",
            today,
            "team2",
            "team1",
            FixtureStatus::Scheduled,
        )]);
        cup.id = "cup".to_string();
        cup.kind = CompetitionType::Cup;
        game.competitions = vec![league_of(Vec::new()), cup];
        game
    });

    let seeds: Vec<u64> = injuries.iter().map(|(_, game)| game.seed).collect();
    assert_eq!(seeds, Vec::<u64>::new());
}

/// Given the user's three unfit players and three more at Other Town, a club the user does not
///      manage, over a thousand seeds
/// When the day's training injuries roll
/// Then Other Town loses a player on 20 to 110 of them, as the user's club does (6% a day), and
///      the user's inbox never reports one of its players
#[test]
fn a_club_the_user_does_not_manage_has_training_injuries_too() {
    let days = days_with_an_injury_at("team0", two_clubs_of_unfit_players);

    assert!(
        (20..=110).contains(&days.len()),
        "{} in {SEEDS} seeds",
        days.len()
    );
    for game in &days {
        assert_eq!(
            injured_in_training(game).filter(|player| player.starts_with('q')),
            None,
            "seed {}: the inbox reported another club's injury",
            game.seed
        );
    }
}

/// Given Other Town's three unfit players and a manager out of work, over a thousand seeds
/// When the day's training injuries roll
/// Then Other Town still loses a player on 20 to 110 of them, and no inbox message is sent
#[test]
fn training_injuries_go_on_while_the_manager_is_out_of_work() {
    let days = days_with_an_injury_at("team0", |seed| {
        let mut game = two_clubs_of_unfit_players(seed);
        game.manager.team_id = None;
        game
    });

    assert!(
        (20..=110).contains(&days.len()),
        "{} in {SEEDS} seeds",
        days.len()
    );
    for game in &days {
        assert!(game.messages.is_empty(), "seed {}", game.seed);
    }
}

/// Given the user's three unfit players, alone in one world and beside Other Town's three in
///      another, over a thousand seeds
/// When the day's training injuries roll in both
/// Then the user's club loses the same player on the same seeds in both: each club rolls its own
///      dice, so another club's squad does not change what happens at the user's
#[test]
fn another_clubs_squad_does_not_change_the_users_training_injuries() {
    for seed in 0..SEEDS {
        let mut alone = three_unfit_players(seed);
        let mut beside = two_clubs_of_unfit_players(seed);
        roll_training_injuries(&mut alone);
        roll_training_injuries(&mut beside);

        assert_eq!(
            injured_at(&alone, "team1"),
            injured_at(&beside, "team1"),
            "seed {seed}"
        );
    }
}

/// Given Other Town's q1 in its starting XI and worth nothing, over a thousand seeds
/// When q1 is hurt in training
/// Then the news reports it under Other Town: one notability rule for every club
#[test]
fn a_starter_hurt_in_training_at_another_club_makes_the_news() {
    let days: Vec<Game> = days_with_an_injury_at("team0", two_clubs_of_unfit_players)
        .into_iter()
        .filter(|game| injured_at(game, "team0") == ["q1"])
        .collect();

    assert!(!days.is_empty(), "q1 never hurt in {SEEDS} seeds");
    for game in &days {
        let story = injury_story(game, "q1").expect("an injured starter makes the news");
        assert_eq!(
            story.i18n_params.get("team").map(String::as_str),
            Some("Other Town"),
            "seed {}",
            game.seed
        );
    }
}

/// Given the same squad and a scheduled match tomorrow, over a thousand seeds
/// When today's random events roll
/// Then someone is hurt in training on 20 to 110 of them, as on a day with no match (6% each)
#[test]
fn a_match_tomorrow_leaves_todays_training_as_risky_as_any_day() {
    let injuries = training_injuries(|seed| {
        let mut game = three_unfit_players(seed);
        with_a_match(&mut game, 1);
        game
    });

    assert!(
        (20..=110).contains(&injuries.len()),
        "{} in {SEEDS} seeds",
        injuries.len()
    );
}

/// Given a squad of three worth nothing, and another club listed first, over a thousand seeds
/// When a starter is hurt in training
/// Then the news reports it: a starter is notable whatever his value
#[test]
fn an_injured_starter_makes_the_news_whatever_his_value() {
    let starters: Vec<(String, Game)> = training_injuries(three_unfit_players)
        .into_iter()
        .filter(|(player, _)| player != "p0")
        .collect();

    assert!(!starters.is_empty(), "no starter hurt in {SEEDS} seeds");
    for (player, game) in &starters {
        assert!(
            injury_story(game, player).is_some(),
            "seed {}: {player} was hurt and the news missed it",
            game.seed
        );
    }
}

/// Given the same squad, over a thousand seeds
/// When p0, on the bench and worth nothing, is hurt in training
/// Then the news leaves it out
#[test]
fn an_injured_reserve_worth_nothing_stays_out_of_the_news() {
    let reserve: Vec<(String, Game)> = training_injuries(three_unfit_players)
        .into_iter()
        .filter(|(player, _)| player == "p0")
        .collect();

    assert!(!reserve.is_empty(), "p0 never hurt in {SEEDS} seeds");
    for (_, game) in &reserve {
        assert!(
            injury_story(game, "p0").is_none(),
            "seed {}: a reserve worth nothing made the news",
            game.seed
        );
    }
}

/// Given p0 on the bench but worth 1,000,000, and another club listed first, over a thousand seeds
/// When p0 is hurt in training
/// Then the story names his club, Test FC
#[test]
fn injury_news_names_the_injured_players_club() {
    let injuries = training_injuries(|seed| {
        let mut game = three_unfit_players(seed);
        game.players[0].market_value = 1_000_000;
        game
    });
    let p0: Vec<&Game> = injuries
        .iter()
        .filter(|(player, _)| player == "p0")
        .map(|(_, game)| game)
        .collect();

    assert!(!p0.is_empty(), "p0 never hurt in {SEEDS} seeds");
    for game in p0 {
        let story = injury_story(game, "p0").expect("a player worth 1,000,000 makes the news");
        assert_eq!(
            story.i18n_params.get("team").map(String::as_str),
            Some("Test FC"),
            "seed {}",
            game.seed
        );
    }
}

/// Given p0 worth 1,000,000 and the news already holding an unrelated story, over a thousand seeds
/// When p0 is hurt in training
/// Then his injury still makes the news
#[test]
fn an_unrelated_story_does_not_keep_an_injury_out_of_the_news() {
    let injuries = training_injuries(|seed| {
        let mut game = three_unfit_players(seed);
        game.players[0].market_value = 1_000_000;
        game.news.push(NewsArticle::new(
            "unrelated".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "2025-06-14".to_string(),
            NewsCategory::InjuryNews,
        ));
        game
    });
    let p0: Vec<&Game> = injuries
        .iter()
        .filter(|(player, _)| player == "p0")
        .map(|(_, game)| game)
        .collect();

    assert!(!p0.is_empty(), "p0 never hurt in {SEEDS} seeds");
    for game in p0 {
        assert!(
            injury_story(game, "p0").is_some(),
            "seed {}: the injury was left out",
            game.seed
        );
    }
}
