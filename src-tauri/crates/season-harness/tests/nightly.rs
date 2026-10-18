//! The nightly tier: runs the PR gate has no time for. Every test is `#[ignore]`d so
//! `cargo test --workspace` never plays them; the nightly workflow runs
//! `cargo test -p season-harness --test nightly -- --ignored`.
//!
//! Budget: a few minutes. The PR gate (`gate.rs`) stays under two minutes and is not touched here.

use std::collections::BTreeSet;

use domain::league::FixtureStatus;
use domain::message::InboxMessage;
use domain::player::{Player, Position, SquadRole};
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use ofm_core::squad_floor::{Shortage, can_put_a_side_out, squad_shortfall};
use ofm_core::world_cup::{is_world_cup_competition, world_cup_champion};
use season_harness::calendar::{reloaded, unplayed_clashes};
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::invariants::Invariants;
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

/// The rule for the player's own club, which the game never signs for: it is warned when a
/// shortage enters and again when one gets worse (never a repeat for a shortage that has not
/// changed, so the inbox is not flooded, and no warning for one that merely leaves: the
/// maintainer's rule of 12 Oct 2026), and at kick-off it can still put a side out (the kick-off
/// gate tops it up only when it cannot).
#[derive(Default)]
struct PlayerClubWatch {
    /// The shortages the club had at the last day end, each with how many seniors it has where it
    /// is short: the same pair the game keeps a warning marker for.
    known: BTreeSet<String>,
    /// What entered or got worse on `changed_on`, still owed a warning.
    owed: String,
    changed_on: String,
    reported: bool,
    /// A change in the shortfall that was never warned about (#787).
    unwarned: Vec<String>,
    /// A day the club played without eleven seniors or a goalkeeper.
    could_not_field_a_side: Vec<String>,
}

const SQUAD_WARNING_KEY: &str = "be.msg.squadBelowFloor";

/// How many seniors the club has where it is short: in the group, or in all.
fn seniors_in(game: &Game, club: &str, shortage: &Shortage) -> usize {
    game.players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(club))
        .filter(|player| player.squad_role == SquadRole::Senior)
        .filter(|player| match shortage {
            Shortage::Group(group) => player.position.to_group_position() == *group,
            Shortage::Seniors => true,
        })
        .count()
}

fn is_squad_warning(message: &InboxMessage) -> bool {
    message
        .body_key
        .as_deref()
        .is_some_and(|key| key.starts_with(SQUAD_WARNING_KEY))
}

impl PlayerClubWatch {
    fn check_warning(&mut self, game: &Game, club: &str, today: &str, season: u32, day: u32) {
        let now: BTreeSet<String> = squad_shortfall(game, club)
            .iter()
            .map(|(shortage, _)| {
                format!(
                    "{shortage:?} with {} seniors",
                    seniors_in(game, club, shortage)
                )
            })
            .collect();
        let entered: Vec<String> = now.difference(&self.known).cloned().collect();
        if !entered.is_empty() {
            self.owed = entered.join(", ");
            self.changed_on = today.to_string();
            self.reported = false;
        }
        self.known = now;
        if self.known.is_empty() {
            self.owed.clear();
        }
        if self.owed.is_empty() || self.reported {
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
            self.unwarned.push(format!(
                "[season {season} day {day}] {club}'s shortfall changed on {}: {} entered or got worse and was not warned about (last warning: {last_warning})",
                self.changed_on, self.owed
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
            self.could_not_field_a_side.push(format!(
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
        // `process_day` advances the clock as its last step, so the clock now reads the day after
        // the one just processed, while a warning sent during that processing is dated the day
        // itself. The warning is compared against the day it could have been sent.
        let processed = game
            .clock
            .current_date
            .date_naive()
            .pred_opt()
            .map_or_else(|| today.clone(), |day| day.format("%Y-%m-%d").to_string());
        self.check_warning(game, &club, &processed, season, day);
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
    assert!(silent.unwarned.is_empty(), "one day's grace");
    game.clock.advance_days(1);
    silent.on_day_end(&game, 1, 2);
    assert_eq!(silent.unwarned.len(), 1, "{:?}", silent.unwarned);

    let mut warned = PlayerClubWatch::default();
    let warning = a_warning_for(&game);
    game.messages.push(warning);
    warned.on_day_end(&game, 1, 1);
    game.clock.advance_days(1);
    warned.on_day_end(&game, 1, 2);
    assert!(warned.unwarned.is_empty(), "{:?}", warned.unwarned);
}

/// Takes every senior of the club in `group` out of the world and returns them, so they can come
/// back; `keep` of them stay.
fn take_out(game: &mut Game, club: &str, group: Position, keep: usize) -> Vec<Player> {
    let mut kept = 0;
    let mut taken = Vec::new();
    let mut staying = Vec::new();
    for player in std::mem::take(&mut game.players) {
        let in_group = player.team_id.as_deref() == Some(club)
            && player.squad_role == SquadRole::Senior
            && player.position.to_group_position() == group;
        if in_group && kept < keep {
            kept += 1;
            staying.push(player);
        } else if in_group {
            taken.push(player);
        } else {
            staying.push(player);
        }
    }
    game.players = staying;
    taken
}

/// Runs the watch over `days` further days, ending each at the clock's next day as the driver does.
fn watch_days(watch: &mut PlayerClubWatch, game: &mut Game, days: u32) {
    for day in 0..days {
        watch.on_day_end(game, 1, day + 1);
        game.clock.advance_days(1);
    }
}

fn warn_about_every_shortage(game: &mut Game) {
    let warning = a_warning_for(game);
    game.messages.push(warning);
}

/// Given the player's club short in defence and in attack, warned about both,
/// When the defence is filled and the attack gap stays as it was,
/// Then nothing is owed: the game warns when a shortage enters or gets worse, not when one leaves
///      (the maintainer's rule of 12 Oct 2026). This is the 4 Jul 2037 case of the red nightly.
#[test]
#[ignore = "nightly tier"]
fn a_shortage_that_leaves_while_another_stays_is_not_news() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let club = players_club(&game);
    take_out(&mut game, &club, Position::Forward, 1);
    let defenders = take_out(&mut game, &club, Position::Defender, 0);
    let mut watch = PlayerClubWatch::default();
    game.clock.advance_days(1);
    warn_about_every_shortage(&mut game);
    watch_days(&mut watch, &mut game, 2);

    game.players.extend(defenders);
    assert!(
        squad_shortfall(&game, &club)
            .iter()
            .all(|(shortage, _)| *shortage == Shortage::Group(Position::Forward)),
        "the defence is filled and only the forwards are short: {:?}",
        squad_shortfall(&game, &club)
    );
    watch_days(&mut watch, &mut game, 3);

    assert!(watch.unwarned.is_empty(), "{:?}", watch.unwarned);
}

/// Given the player's club one forward short and warned about it,
/// When it loses its last forward and nothing is sent,
/// Then the watch reports it: a shortage that gets worse is news.
#[test]
#[ignore = "nightly tier"]
fn a_shortage_that_gets_worse_is_news() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let club = players_club(&game);
    take_out(&mut game, &club, Position::Forward, 1);
    let mut watch = PlayerClubWatch::default();
    game.clock.advance_days(1);
    warn_about_every_shortage(&mut game);
    watch_days(&mut watch, &mut game, 2);
    assert!(watch.unwarned.is_empty(), "{:?}", watch.unwarned);

    take_out(&mut game, &club, Position::Forward, 0);
    watch_days(&mut watch, &mut game, 3);

    assert_eq!(watch.unwarned.len(), 1, "{:?}", watch.unwarned);
}

/// Given the player's club short in defence and one forward short, warned about both,
/// When the defence is filled while the club loses its last forward, and nothing is sent,
/// Then the watch still reports it: one shortage leaving does not hide another getting worse.
#[test]
#[ignore = "nightly tier"]
fn a_shortage_getting_worse_is_news_even_while_another_leaves() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let club = players_club(&game);
    take_out(&mut game, &club, Position::Forward, 1);
    let defenders = take_out(&mut game, &club, Position::Defender, 0);
    let mut watch = PlayerClubWatch::default();
    game.clock.advance_days(1);
    warn_about_every_shortage(&mut game);
    watch_days(&mut watch, &mut game, 2);
    assert!(watch.unwarned.is_empty(), "{:?}", watch.unwarned);

    game.players.extend(defenders);
    take_out(&mut game, &club, Position::Forward, 0);
    watch_days(&mut watch, &mut game, 3);

    assert_eq!(watch.unwarned.len(), 1, "{:?}", watch.unwarned);
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
    assert!(
        fine.could_not_field_a_side.is_empty(),
        "{:?}",
        fine.could_not_field_a_side
    );

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
        without_keeper.could_not_field_a_side.len(),
        1,
        "{:?}",
        without_keeper.could_not_field_a_side
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
///      rule holds.
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
        assert!(
            player_club.could_not_field_a_side.is_empty(),
            "seed {seed}: the player's club could not field a side:\n{}",
            player_club.could_not_field_a_side.join("\n")
        );
        assert!(
            player_club.unwarned.is_empty(),
            "seed {seed}: the player's club was not warned of a change in its shortfall:\n{}",
            player_club.unwarned.join("\n")
        );
        assert!(
            invariants.is_clean(),
            "seed {seed} broke an invariant:\n{}",
            invariants.report()
        );
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

/// Given a generated world played through four seasons, saved and reloaded between each,
/// When every rollover has generated the next season's calendar,
/// Then no club has two matches to play within the minimum rest in it.
#[test]
#[ignore = "nightly tier"]
fn four_seasons_generate_clash_free_calendars_with_reloads() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let mut clashes = Vec::new();

    for season in 1..=4 {
        let mut invariants = Invariants::new(&game);
        run_seasons(&mut game, RunOptions::seasons(1), &mut invariants)
            .unwrap_or_else(|error| panic!("season {season}: {error}"));
        // The run ends on the rollover, so this is the calendar it has just generated.
        clashes.extend(
            unplayed_clashes(&game)
                .into_iter()
                .map(|clash| format!("after season {season}: {clash}")),
        );
        game = reloaded(&game);
    }

    assert_eq!(clashes, Vec::<String>::new());
}
