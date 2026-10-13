//! The nightly tier: runs the PR gate has no time for. Every test is `#[ignore]`d so
//! `cargo test --workspace` never plays them; the nightly workflow runs
//! `cargo test -p season-harness --test nightly -- --ignored`.
//!
//! Budget: a few minutes. The PR gate (`gate.rs`) stays under two minutes and is not touched here.

use std::collections::BTreeSet;

use domain::league::FixtureStatus;
use domain::message::InboxMessage;
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use ofm_core::squad_floor::{can_put_a_side_out, squad_shortfall};
use ofm_core::world_cup::{is_world_cup_competition, world_cup_champion};
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::invariants::{CLUB_RESTS_BETWEEN_MATCHES, Invariants};
use season_harness::known_bug::expect_bug;
use season_harness::worlds::WorldSpec;

const SEEDS: &[u64] = &[1, 2, 3];

/// AI clubs short of the squad floor, named, in the state the game is in right now.
///
/// The player's own club is left out: the game tops up AI squads, and the harness makes no
/// signings for the player, so a retirement there leaves a gap that a player would fill.
fn floor_breaches(game: &Game) -> Vec<String> {
    game.teams
        .iter()
        .filter(|team| game.manager.team_id.as_deref() != Some(team.id.as_str()))
        .filter_map(|team| {
            let shortfall = squad_shortfall(game, &team.id);
            (!shortfall.is_empty()).then(|| format!("{} is short: {shortfall:?}", team.id))
        })
        .collect()
}

/// Checks the squad floor twice a season: when the season is complete, after a year of daily
/// squad planning, and straight after the rollover, which retires players and moves contracts.
#[derive(Default)]
struct SquadFloorWatch {
    breaches: Vec<String>,
}

impl SquadFloorWatch {
    fn note(&mut self, game: &Game, when: &str, season: u32) {
        self.breaches.extend(
            floor_breaches(game)
                .into_iter()
                .map(|line| format!("[season {season}, {when}] {line}")),
        );
    }
}

impl SeasonObserver for SquadFloorWatch {
    fn on_before_rollover(&mut self, game: &Game, season: u32) {
        self.note(game, "season complete", season);
    }

    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, season: u32) {
        self.note(game, "after the rollover", season);
    }
}

/// The rule for the player's own club, which the game never signs for: it is warned when it goes
/// below the floor and again whenever its shortfall changes (never a repeat for an unchanged one,
/// so the inbox is not flooded), and at kick-off it can still put a side out (the kick-off gate
/// tops it up only when it cannot).
#[derive(Default)]
struct PlayerClubWatch {
    shortfall: String,
    changed_on: String,
    reported: bool,
    problems: Vec<String>,
}

const SQUAD_WARNING_KEY: &str = "be.msg.squadBelowFloor";

fn is_squad_warning(message: &InboxMessage) -> bool {
    message
        .body_key
        .as_deref()
        .is_some_and(|key| key.starts_with(SQUAD_WARNING_KEY))
}

impl PlayerClubWatch {
    fn check_warning(&mut self, game: &Game, club: &str, today: &str, season: u32, day: u32) {
        let shortfall = squad_shortfall(game, club);
        let described = if shortfall.is_empty() {
            String::new()
        } else {
            format!("{shortfall:?}")
        };
        if described != self.shortfall {
            self.shortfall = described;
            self.changed_on = today.to_string();
            self.reported = false;
        }
        if shortfall.is_empty() || self.reported {
            return;
        }
        // The daily check that warns runs after the day that changed the squad, so a change gets
        // one day's grace before a missing warning counts.
        let warned = game.messages.iter().any(|message| {
            message.date.as_str() >= self.changed_on.as_str() && is_squad_warning(message)
        });
        if !warned && self.changed_on.as_str() < today {
            let last_warning = game
                .messages
                .iter()
                .filter(|message| is_squad_warning(message))
                .map(|message| message.date.as_str())
                .max()
                .unwrap_or("never");
            self.reported = true;
            self.problems.push(format!(
                "[season {season} day {day}] {club}'s shortfall changed on {} to {} and was not warned about (last warning: {last_warning})",
                self.changed_on, self.shortfall
            ));
        }
    }

    fn check_kick_off(&mut self, game: &Game, club: &str, today: &str, season: u32, day: u32) {
        let played_today = game
            .competitions
            .iter()
            .flat_map(|competition| &competition.fixtures)
            .any(|fixture| {
                fixture.date == today
                    && fixture.status == FixtureStatus::Completed
                    && (fixture.home_team_id == club || fixture.away_team_id == club)
            });
        if played_today && !can_put_a_side_out(game, club) {
            self.problems.push(format!(
                "[season {season} day {day}] {club} played with fewer than 11 seniors or no goalkeeper"
            ));
        }
    }
}

impl SeasonObserver for PlayerClubWatch {
    fn on_day_end(&mut self, game: &Game, season: u32, day: u32) {
        let Some(club) = game.manager.team_id.clone() else {
            return;
        };
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        self.check_warning(game, &club, &today, season, day);
        self.check_kick_off(game, &club, &today, season, day);
    }
}

fn players_club(game: &Game) -> String {
    game.manager.team_id.clone().expect("the player has a club")
}

fn drop_midfielders(game: &mut Game, club: &str) {
    game.players.retain(|player| {
        player.team_id.as_deref() != Some(club)
            || player.position != domain::player::Position::Midfielder
    });
}

fn a_warning_for(game: &Game) -> InboxMessage {
    InboxMessage::new(
        "squad_below_floor_test".to_string(),
        String::new(),
        String::new(),
        String::new(),
        game.clock.current_date.format("%Y-%m-%d").to_string(),
    )
    .with_i18n(
        &format!("{SQUAD_WARNING_KEY}.subject"),
        &format!("{SQUAD_WARNING_KEY}.body"),
        std::collections::HashMap::new(),
    )
}

/// Given the player's club has fallen below the floor and nobody has warned them,
/// When two days pass,
/// Then the watch reports it; with the warning sent it does not.
#[test]
#[ignore = "nightly tier"]
fn the_player_club_below_the_floor_must_have_been_warned() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let club = players_club(&game);
    drop_midfielders(&mut game, &club);
    let mut silent = PlayerClubWatch::default();
    silent.on_day_end(&game, 1, 1);
    assert!(silent.problems.is_empty(), "one day's grace");
    game.clock.advance_days(1);
    silent.on_day_end(&game, 1, 2);
    assert_eq!(silent.problems.len(), 1, "{:?}", silent.problems);

    let mut warned = PlayerClubWatch::default();
    let warning = a_warning_for(&game);
    game.messages.push(warning);
    warned.on_day_end(&game, 1, 1);
    game.clock.advance_days(1);
    warned.on_day_end(&game, 1, 2);
    assert!(warned.problems.is_empty(), "{:?}", warned.problems);
}

/// Given the player's club has only ten seniors and a fixture today that is completed,
/// When the day ends,
/// Then the watch reports a club that kicked off without a side; with eleven it does not.
#[test]
#[ignore = "nightly tier"]
fn the_player_club_must_be_able_to_field_a_side_at_kick_off() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let club = players_club(&game);
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let fixture = game
        .competitions
        .iter_mut()
        .flat_map(|competition| competition.fixtures.iter_mut())
        .find(|fixture| fixture.home_team_id == club || fixture.away_team_id == club)
        .expect("the club has a fixture");
    fixture.date = today.clone();
    fixture.status = FixtureStatus::Completed;
    let mut fine = PlayerClubWatch::default();
    fine.on_day_end(&game, 1, 1);
    assert!(fine.problems.is_empty(), "{:?}", fine.problems);

    let keepers: Vec<String> = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(club.as_str()))
        .filter(|player| player.position == domain::player::Position::Goalkeeper)
        .map(|player| player.id.clone())
        .collect();
    game.players.retain(|player| !keepers.contains(&player.id));
    let mut without_keeper = PlayerClubWatch::default();
    without_keeper.on_day_end(&game, 1, 1);

    assert_eq!(
        without_keeper
            .problems
            .iter()
            .filter(|problem| problem.contains("goalkeeper"))
            .count(),
        1,
        "{:?}",
        without_keeper.problems
    );
}

/// Given a club with no players,
/// When the squad floor is read,
/// Then the club is named. The watch below is only as good as this.
#[test]
#[ignore = "nightly tier"]
fn a_club_with_no_players_is_a_floor_breach() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    assert!(floor_breaches(&game).is_empty(), "the built world is sound");
    let player_club = game.manager.team_id.clone();
    let emptied = game
        .teams
        .iter()
        .map(|team| team.id.clone())
        .find(|id| Some(id) != player_club.as_ref())
        .expect("the world has an AI club");
    game.players
        .retain(|player| player.team_id.as_deref() != Some(emptied.as_str()));

    let breaches = floor_breaches(&game);

    assert!(
        breaches.iter().any(|line| line.starts_with(&emptied)),
        "{breaches:?}"
    );
}

/// Given generated worlds from three seeds,
/// When each is played through five seasons,
/// Then no AI club is below the squad floor at the end of any season, the player's club was warned
///      whenever it was below the floor and could always field a side at kick-off, and every other
///      rule holds
///      (bar the #553 clash that the gate carries).
#[test]
#[ignore = "nightly tier"]
fn five_seasons_leave_no_club_below_the_squad_floor() {
    for &seed in SEEDS {
        let mut game = WorldSpec::gate(seed).build().expect("the world builds");
        let mut invariants = Invariants::new(&game);
        let mut watch = SquadFloorWatch::default();
        let mut player_club = PlayerClubWatch::default();

        run_seasons(
            &mut game,
            RunOptions::seasons(5),
            &mut (&mut invariants, (&mut watch, &mut player_club)),
        )
        .unwrap_or_else(|error| panic!("seed {seed}: {error}"));

        assert!(
            watch.breaches.is_empty(),
            "seed {seed} left clubs below the floor:\n{}",
            watch.breaches.join("\n")
        );
        let (unwarned, unplayable): (Vec<_>, Vec<_>) = player_club
            .problems
            .iter()
            .partition(|problem| problem.contains("was not warned about"));
        assert!(
            unplayable.is_empty(),
            "seed {seed}: the player's club was not looked after:\n{}",
            unplayable
                .iter()
                .map(|problem| problem.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
        expect_bug(787, "was not warned about", || {
            assert!(
                unwarned.is_empty(),
                "seed {seed}: the player's club:\n{}",
                unwarned
                    .iter()
                    .map(|problem| problem.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        });
        let others = invariants.report_except(CLUB_RESTS_BETWEEN_MATCHES);
        assert!(
            others.is_empty(),
            "seed {seed} broke an invariant:\n{others}"
        );
        let clashes = invariants.report_of(CLUB_RESTS_BETWEEN_MATCHES);
        expect_bug(553, "days apart", || {
            assert!(clashes.is_empty(), "seed {seed}:\n{clashes}");
        });
    }
}

/// Notes the day a World Cup final has been played and who won it.
///
/// A tournament that already had a champion when the run began is not counted until it is
/// rebuilt and unfinished again, so an earlier edition cannot stand in for the one under test.
struct WorldCupWatch {
    already_decided: BTreeSet<String>,
    champions: Vec<(u32, String)>,
}

impl WorldCupWatch {
    fn new(game: &Game) -> Self {
        let already_decided = game
            .competitions
            .iter()
            .filter(|competition| is_world_cup_competition(competition))
            .filter(|competition| world_cup_champion(competition).is_some())
            .map(|competition| competition.id.clone())
            .collect();
        Self {
            already_decided,
            champions: Vec::new(),
        }
    }
}

impl SeasonObserver for WorldCupWatch {
    fn on_day_end(&mut self, game: &Game, season: u32, _day: u32) {
        for competition in game
            .competitions
            .iter()
            .filter(|competition| is_world_cup_competition(competition))
        {
            let Some(champion) = world_cup_champion(competition) else {
                self.already_decided.remove(&competition.id);
                continue;
            };
            if !self.already_decided.contains(&competition.id)
                && !self.champions.iter().any(|(played, _)| *played == season)
            {
                self.champions.push((season, champion));
            }
        }
    }
}

/// Given a world that opens in 2033, so the 2034 World Cup falls within the first two seasons,
/// When those seasons are played,
/// Then the World Cup is played to its final and has a champion (the #433 retest).
#[test]
#[ignore = "nightly tier"]
fn a_world_cup_is_played_to_its_final() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let mut watch = WorldCupWatch::new(&game);

    run_seasons(&mut game, RunOptions::seasons(2), &mut watch).expect("the seasons play");

    let champion = watch
        .champions
        .first()
        .map(|(_, champion)| champion.as_str());
    assert!(
        champion.is_some_and(|id| !id.is_empty()),
        "no World Cup reached a final in two seasons from 2033"
    );
}

/// Rates of the players who have played enough to have a season average.
#[derive(Default)]
struct RatingWatch {
    seasons: Vec<RatedSeason>,
}

struct RatedSeason {
    season: u32,
    rated: usize,
    mean: f64,
    share_at_least_8_5: f64,
}

const ENOUGH_APPEARANCES: u32 = 10;

impl SeasonObserver for RatingWatch {
    fn on_before_rollover(&mut self, game: &Game, season: u32) {
        let ratings: Vec<f64> = game
            .players
            .iter()
            .filter(|player| {
                player.stats.appearances >= ENOUGH_APPEARANCES && player.stats.avg_rating > 0.0
            })
            .map(|player| f64::from(player.stats.avg_rating))
            .collect();
        let rated = ratings.len();
        if rated == 0 {
            return;
        }
        let count = rated as f64;
        let mut top: Vec<(&str, f32)> = game
            .players
            .iter()
            .filter(|player| player.stats.appearances >= 5 && player.stats.avg_rating > 0.0)
            .map(|player| (player.id.as_str(), player.stats.avg_rating))
            .collect();
        top.sort_by(|a, b| b.1.total_cmp(&a.1));
        eprintln!(
            "season {season}: top five average ratings {:?}",
            &top[..top.len().min(5)]
        );
        self.seasons.push(RatedSeason {
            season,
            rated,
            mean: ratings.iter().sum::<f64>() / count,
            share_at_least_8_5: ratings.iter().filter(|rating| **rating >= 8.5).count() as f64
                / count,
        });
    }
}

/// Given generated worlds from three seeds,
/// When each is played through three seasons,
/// Then at every season end the mean season rating of players with 10+ appearances is between
///      5.9 and 6.4, and fewer than 2% of those players average 8.5 or more.
#[test]
#[ignore = "nightly tier"]
fn ratings_stay_within_their_bounds() {
    for &seed in SEEDS {
        let mut game = WorldSpec::gate(seed).build().expect("the world builds");
        let mut watch = RatingWatch::default();

        run_seasons(&mut game, RunOptions::seasons(3), &mut watch)
            .unwrap_or_else(|error| panic!("seed {seed}: {error}"));

        assert_eq!(
            watch.seasons.len(),
            3,
            "seed {seed}: every season has players with {ENOUGH_APPEARANCES}+ rated appearances"
        );
        for season in &watch.seasons {
            eprintln!(
                "seed {seed} season {}: {} rated, mean {:.2}, {:.2}% at 8.5+",
                season.season,
                season.rated,
                season.mean,
                season.share_at_least_8_5 * 100.0
            );
            assert!(
                (5.9..=6.4).contains(&season.mean),
                "seed {seed} season {}: mean rating {:.2} is outside 5.9..=6.4",
                season.season,
                season.mean
            );
            assert!(
                season.share_at_least_8_5 < 0.02,
                "seed {seed} season {}: {:.2}% average 8.5 or more",
                season.season,
                season.share_at_least_8_5 * 100.0
            );
        }
    }
}
