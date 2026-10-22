//! Where the world's best players play, by how big the club is (#869, slice 5).
//!
//! Measurement only: nothing here judges the numbers. Clubs are ranked by reputation and split
//! into five equal groups; for each group the report gives its share of the world's S and A
//! players and the mean rating of its first teams, so a change to how players are generated or
//! moved can be read as a before and after. The same figures are given by league tier.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use domain::age::age_on_or_unknown;
use domain::league::CompetitionType;
use domain::player::Player;
use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;
use ofm_core::player_tier::{PlayerTier, player_tier};

use crate::distribution::SENIOR_MIN_AGE;
use crate::driver::SeasonObserver;

/// How many players make a first team.
const FIRST_TEAM: usize = 11;

/// One group of clubs. Shares are fractions of **all** the world's senior S (or A) players,
/// free agents included, so the groups and the unattached row add up to one.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GroupReport {
    pub clubs: usize,
    pub s_share: f64,
    pub a_share: f64,
    /// Mean, over the group's clubs, of the mean rating of each club's eleven best seniors.
    pub first_team_ovr: f64,
}

/// The world on one day. `quintiles[0]` is the lowest-reputation fifth of clubs, `quintiles[4]`
/// the highest; `reputation_range[i]` is the lowest and highest reputation in it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReputationReport {
    pub quintiles: [GroupReport; 5],
    pub reputation_range: [(u32, u32); 5],
    /// Seniors who belong to no club.
    pub unattached: GroupReport,
    /// Clubs by division tier (1 = top flight of each country), in tier order. A club is counted
    /// once, in the highest division it plays in.
    pub tiers: Vec<(u32, GroupReport)>,
}

/// The report for `game` on its own date.
///
/// Only seniors (aged `SENIOR_MIN_AGE` or more, not retired) are counted. Clubs are ranked by
/// reputation, ties broken by id, and split into five groups whose sizes differ by at most one. A
/// player whose `team_id` names no club in `game.teams` counts as unattached.
/// A club is counted once in the by-tier rows, in the highest division it plays in; the S and
/// A shares are fractions of every senior S or A in the world.
pub fn reputation_report(game: &Game) -> ReputationReport {
    let on = game.clock.current_date.date_naive();
    let seniors: Vec<&Player> = game
        .players
        .iter()
        .filter(|player| {
            !player.retired && age_on_or_unknown(on, &player.date_of_birth) >= SENIOR_MIN_AGE
        })
        .collect();
    let mut by_club: HashMap<&str, Vec<&Player>> = HashMap::new();
    for player in &seniors {
        if let Some(club) = player.team_id.as_deref() {
            by_club.entry(club).or_default().push(player);
        }
    }
    let world_s = seniors
        .iter()
        .filter(|p| player_tier(p.ovr) == PlayerTier::S)
        .count();
    let world_a = seniors
        .iter()
        .filter(|p| player_tier(p.ovr) == PlayerTier::A)
        .count();
    let group = |clubs: &[&str]| group_report(clubs, &by_club, world_s, world_a);

    let mut ranked: Vec<(&str, u32)> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), team.reputation))
        .collect();
    ranked.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(b.0)));
    let mut fifths: [Vec<&str>; 5] = Default::default();
    for (index, (club, _)) in ranked.iter().enumerate() {
        fifths[index * 5 / ranked.len()].push(club);
    }
    let mut report = ReputationReport::default();
    for (index, clubs) in fifths.iter().enumerate() {
        report.quintiles[index] = group(clubs);
        let reputations = ranked
            .iter()
            .filter(|(club, _)| clubs.contains(club))
            .map(|(_, reputation)| *reputation);
        report.reputation_range[index] = (
            reputations.clone().min().unwrap_or(0),
            reputations.max().unwrap_or(0),
        );
    }

    report.unattached = unattached_report(game, &seniors, world_s, world_a);
    report.tiers = clubs_by_tier(game)
        .into_iter()
        .map(|(tier, clubs)| (tier, group(&clubs)))
        .collect();
    report
}

/// The S and A shares of the seniors whose `team_id` names no club in `game.teams`.
fn unattached_report(
    game: &Game,
    seniors: &[&Player],
    world_s: usize,
    world_a: usize,
) -> GroupReport {
    let attached: HashSet<&str> = game.teams.iter().map(|team| team.id.as_str()).collect();
    let unattached: Vec<&&Player> = seniors
        .iter()
        .filter(|p| {
            p.team_id
                .as_deref()
                .is_none_or(|club| !attached.contains(club))
        })
        .collect();
    let in_tier = |tier: PlayerTier| {
        unattached
            .iter()
            .filter(|p| player_tier(p.ovr) == tier)
            .count()
    };
    GroupReport {
        clubs: 0,
        s_share: share(in_tier(PlayerTier::S), world_s),
        a_share: share(in_tier(PlayerTier::A), world_a),
        first_team_ovr: 0.0,
    }
}

/// Each club once, under the highest division it plays in. A division's tier is its place among
/// its own country's leagues by `priority` (1 = top flight): `priority` itself is a counter that
/// runs on across countries, so it cannot be compared between them. Cups and other competitions do
/// not place a club. Clubs within a tier are sorted, so a report is the same every time.
fn clubs_by_tier(game: &Game) -> BTreeMap<u32, Vec<&str>> {
    let leagues = || {
        game.competitions
            .iter()
            .filter(|competition| competition.kind == CompetitionType::League)
    };
    let mut priorities: BTreeMap<Option<&str>, BTreeSet<u32>> = BTreeMap::new();
    for league in leagues() {
        priorities
            .entry(league.country_id.as_deref())
            .or_default()
            .insert(league.priority);
    }
    let mut tier_of: HashMap<&str, u32> = HashMap::new();
    for league in leagues() {
        let tier = priorities[&league.country_id.as_deref()]
            .range(..league.priority)
            .count() as u32
            + 1;
        for standing in &league.standings {
            tier_of
                .entry(standing.team_id.as_str())
                .and_modify(|best| *best = (*best).min(tier))
                .or_insert(tier);
        }
    }
    let mut tiers: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    for (club, tier) in tier_of {
        tiers.entry(tier).or_default().push(club);
    }
    for clubs in tiers.values_mut() {
        clubs.sort_unstable();
    }
    tiers
}

fn share(count: usize, of: usize) -> f64 {
    if of == 0 {
        0.0
    } else {
        count as f64 / of as f64
    }
}

fn group_report(
    clubs: &[&str],
    by_club: &HashMap<&str, Vec<&Player>>,
    world_s: usize,
    world_a: usize,
) -> GroupReport {
    let (mut s, mut a) = (0usize, 0usize);
    let mut first_teams = Vec::new();
    for club in clubs {
        let players = by_club.get(club).map_or(&[][..], Vec::as_slice);
        for player in players {
            match player_tier(player.ovr) {
                PlayerTier::S => s += 1,
                PlayerTier::A => a += 1,
                PlayerTier::Rest => {}
            }
        }
        let mut ratings: Vec<u32> = players.iter().map(|p| u32::from(p.ovr)).collect();
        ratings.sort_unstable_by(|x, y| y.cmp(x));
        ratings.truncate(FIRST_TEAM);
        if !ratings.is_empty() {
            first_teams.push(f64::from(ratings.iter().sum::<u32>()) / ratings.len() as f64);
        }
    }
    GroupReport {
        clubs: clubs.len(),
        s_share: share(s, world_s),
        a_share: share(a, world_a),
        first_team_ovr: if first_teams.is_empty() {
            0.0
        } else {
            first_teams.iter().sum::<f64>() / first_teams.len() as f64
        },
    }
}

/// The report after each season's rollover, in order.
#[derive(Default)]
pub struct ReputationLog {
    pub after_each_season: Vec<ReputationReport>,
}

impl SeasonObserver for ReputationLog {
    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {
        self.after_each_season.push(reputation_report(game));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;

    fn attributes() -> PlayerAttributes {
        PlayerAttributes {
            pace: 50,
            stamina: 50,
            strength: 50,
            agility: 50,
            passing: 50,
            shooting: 50,
            tackling: 50,
            dribbling: 50,
            defending: 50,
            positioning: 50,
            vision: 50,
            decisions: 50,
            composure: 50,
            aggression: 50,
            teamwork: 50,
            leadership: 50,
            handling: 50,
            reflexes: 50,
            aerial: 50,
        }
    }

    fn club(id: &str, reputation: u32) -> Team {
        let mut team = Team::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "England".to_string(),
            "Town".to_string(),
            "Ground".to_string(),
            10_000,
        );
        team.reputation = reputation;
        team
    }

    fn player(id: &str, club: Option<&str>, ovr: u8, born: &str) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            born.to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attributes(),
        );
        player.team_id = club.map(str::to_string);
        player.ovr = ovr;
        player
    }

    /// Ten clubs "c0".."c9", reputation 100 + 10 * index, with the given players.
    fn world(players: Vec<Player>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".to_string(),
            "M".to_string(),
            "M".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = (0..10)
            .rev()
            .map(|index| club(&format!("c{index}"), 100 + 10 * index))
            .collect();
        Game::new(clock, manager, teams, players, vec![], vec![])
    }

    fn league(id: &str, country: &str, priority: u32, clubs: &[&str]) -> domain::league::League {
        let teams: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
        let mut league = domain::league::League::new(id.to_string(), id.to_string(), 1, &teams);
        league.priority = priority;
        league.country_id = Some(country.to_string());
        league
    }

    /// Given ten clubs of rising reputation (listed out of order),
    /// When the world is reported,
    /// Then each fifth has two clubs and the top fifth holds the two highest reputations.
    #[test]
    fn clubs_are_split_into_five_equal_fifths_by_reputation() {
        let report = reputation_report(&world(vec![]));

        assert!(report.quintiles.iter().all(|group| group.clubs == 2));
        assert_eq!(report.reputation_range[0], (100, 110));
        assert_eq!(report.reputation_range[4], (180, 190));
    }

    /// Given one S player at the top club, one at the bottom club and one with no club,
    /// When the world is reported,
    /// Then a third of the world's S players sit in each of the top fifth, the bottom fifth and
    ///      the unattached row, and the shares add to one.
    #[test]
    fn the_shares_of_s_players_follow_the_club_and_add_up_with_the_unattached() {
        let report = reputation_report(&world(vec![
            player("top", Some("c9"), 85, "1995-01-01"),
            player("bottom", Some("c0"), 85, "1995-01-01"),
            player("free", None, 85, "1995-01-01"),
        ]));

        let third = 1.0 / 3.0;
        assert_eq!(report.quintiles[4].s_share, third);
        assert_eq!(report.quintiles[0].s_share, third);
        assert_eq!(report.unattached.s_share, third);
        let total: f64 =
            report.quintiles.iter().map(|g| g.s_share).sum::<f64>() + report.unattached.s_share;
        assert!((total - 1.0).abs() < 1e-12);
    }

    /// Given A players at two clubs and S players elsewhere,
    /// When the world is reported,
    /// Then the A shares count A players only, apart from the S shares.
    #[test]
    fn s_and_a_are_counted_apart() {
        let report = reputation_report(&world(vec![
            player("a1", Some("c9"), 78, "1995-01-01"),
            player("a2", Some("c0"), 79, "1995-01-01"),
            player("s1", Some("c0"), 90, "1995-01-01"),
        ]));

        assert_eq!(report.quintiles[4].a_share, 0.5);
        assert_eq!(report.quintiles[4].s_share, 0.0);
        assert_eq!(report.quintiles[0].a_share, 0.5);
        assert_eq!(report.quintiles[0].s_share, 1.0);
    }

    /// Given a youth rated S at the top club,
    /// When the world is reported,
    /// Then he is in no share: like the tier shares elsewhere, only seniors are counted.
    #[test]
    fn a_youth_and_a_retiree_are_not_counted() {
        let mut retired = player("old", Some("c9"), 90, "1980-01-01");
        retired.retired = true;
        let report = reputation_report(&world(vec![
            player("kid", Some("c9"), 90, "2008-01-01"),
            retired,
        ]));

        assert_eq!(report.quintiles[4].s_share, 0.0);
        assert_eq!(report.unattached.s_share, 0.0);
    }

    /// Given a club of twelve players rated 70 for eleven and 10 for the twelfth,
    /// When the world is reported,
    /// Then its first team is the eleven best, rated 70.
    #[test]
    fn the_first_team_is_the_eleven_best() {
        let mut players: Vec<Player> = (0..11)
            .map(|index| player(&format!("p{index}"), Some("c9"), 70, "1995-01-01"))
            .collect();
        players.push(player("weak", Some("c9"), 10, "1995-01-01"));

        let report = reputation_report(&world(players));

        // c9 is one of the top fifth's two clubs; c8 has no players and counts for nothing.
        assert_eq!(report.quintiles[4].first_team_ovr, 70.0);
    }

    /// Given a free-agent A player, a free-agent S player and an ordinary free agent,
    /// When the world is reported,
    /// Then the unattached row counts the A player as all of the world's A and none of its S, and the S player the reverse.
    #[test]
    fn unattached_a_and_s_players_are_counted_apart() {
        let report = reputation_report(&world(vec![
            player("free-a", None, 78, "1995-01-01"),
            player("free-s", None, 90, "1995-01-01"),
            player("free-ordinary", None, 60, "1995-01-01"),
            player("club-s", Some("c9"), 90, "1995-01-01"),
        ]));

        assert_eq!(report.unattached.a_share, 1.0);
        assert_eq!(report.unattached.s_share, 0.5);
    }

    /// Given two clubs of the top fifth whose elevens are rated 60 and 80,
    /// When the world is reported,
    /// Then the fifth's first-team rating is their mean, 70.
    #[test]
    fn a_fifths_first_team_rating_is_the_mean_of_its_clubs() {
        let mut players = Vec::new();
        for (club, ovr) in [("c8", 60), ("c9", 80)] {
            players
                .extend((0..11).map(|index| {
                    player(&format!("{club}-{index}"), Some(club), ovr, "1995-01-01")
                }));
        }

        let report = reputation_report(&world(players));

        assert_eq!(report.quintiles[4].first_team_ovr, 70.0);
    }

    /// Given the log observing a season's end,
    /// When the season ends,
    /// Then it keeps that season's report.
    #[test]
    fn the_log_keeps_a_report_for_each_season_end() {
        let game = world(vec![player("s", Some("c9"), 90, "1995-01-01")]);
        let mut log = ReputationLog::default();

        log.on_season_end(&game, &EndOfSeasonSummary::default(), 1);

        assert_eq!(log.after_each_season, vec![reputation_report(&game)]);
    }

    /// Given a club that stands in a tier-2 league and also in a tier-1 league,
    /// When the world is reported,
    /// Then it is counted once, in tier 1.
    #[test]
    fn a_club_in_two_leagues_is_counted_in_the_higher_tier() {
        let mut game = world(vec![]);
        game.competitions = vec![
            league("second", "x", 2, &["c0", "c1", "c2"]),
            league("first", "x", 1, &["c0"]),
        ];

        let report = reputation_report(&game);

        let counts: Vec<(u32, usize)> = report
            .tiers
            .iter()
            .map(|(tier, group)| (*tier, group.clubs))
            .collect();
        assert_eq!(counts, vec![(1, 1), (2, 2)]);
    }

    /// Given two countries whose leagues carry the world's running priority counter
    ///       (0 and 1 in one country, 2 and 3 in the other),
    /// When the world is reported,
    /// Then each country's top flight is tier 1 and its second division tier 2: two rows, not four.
    #[test]
    fn tiers_are_counted_within_each_country() {
        let mut game = world(vec![]);
        game.competitions = vec![
            league("a1", "a", 0, &["c0", "c1"]),
            league("a2", "a", 1, &["c2", "c3"]),
            league("b1", "b", 2, &["c4", "c5"]),
            league("b2", "b", 3, &["c6", "c7"]),
        ];

        let report = reputation_report(&game);

        let counts: Vec<(u32, usize)> = report
            .tiers
            .iter()
            .map(|(tier, group)| (*tier, group.clubs))
            .collect();
        assert_eq!(counts, vec![(1, 4), (2, 4)]);
    }

    /// Given a world with several tiers,
    /// When it is reported twice,
    /// Then the two reports are exactly equal: club order within a tier does not depend on hashing.
    #[test]
    fn two_reports_of_one_game_are_identical() {
        let mut game = world(vec![
            player("p0", Some("c0"), 90, "1995-01-01"),
            player("p1", Some("c1"), 78, "1995-01-01"),
            player("p2", Some("c2"), 77, "1995-01-01"),
        ]);
        game.competitions = vec![league("l", "a", 0, &["c0", "c1", "c2", "c3", "c4", "c5"])];

        assert_eq!(reputation_report(&game), reputation_report(&game));
    }

    /// Given a cup holding clubs that no league holds,
    /// When the world is reported,
    /// Then the cup places no club in any tier.
    #[test]
    fn a_cup_does_not_place_a_club_in_a_tier() {
        let mut game = world(vec![]);
        let mut cup = league("cup", "x", 0, &["c0", "c1"]);
        cup.kind = CompetitionType::Cup;
        game.competitions = vec![cup];

        assert!(reputation_report(&game).tiers.is_empty());
    }

    /// Given no teams at all, when the world is reported, then nothing divides by zero.
    #[test]
    fn a_world_without_clubs_reports_zeros() {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".to_string(),
            "M".to_string(),
            "M".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);

        assert_eq!(reputation_report(&game), ReputationReport::default());
    }

    /// Given a generated world, when it is reported, then the shares add up and every club is in
    /// exactly one fifth.
    #[test]
    fn a_generated_world_is_fully_accounted_for() {
        let game = crate::worlds::WorldSpec::gate(1)
            .build()
            .expect("the world builds");

        let report = reputation_report(&game);

        let clubs: usize = report.quintiles.iter().map(|g| g.clubs).sum();
        assert_eq!(clubs, game.teams.len());
        let s: f64 =
            report.quintiles.iter().map(|g| g.s_share).sum::<f64>() + report.unattached.s_share;
        assert!((s - 1.0).abs() < 1e-9, "S shares add to {s}");
        assert!(
            !report.tiers.is_empty(),
            "a generated world has league tiers"
        );
    }
}
