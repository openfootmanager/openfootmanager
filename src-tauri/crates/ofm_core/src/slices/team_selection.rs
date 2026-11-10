//! What the team-selection screen shows of every club's squad before a career begins: each
//! player's rating, the squad's average, club facts and the board's opening expectation.
//!
//! The knowledge rule (`player_knowledge`) decides what a manager sees of other clubs' players
//! once he has one; before a career there is no club whose knowledge could apply, and choosing a
//! club blind is no choice (maintainer, wire-knowledge Q1, 2 Nov). So the screen gets its own
//! answer, and only while no career has begun: a manager between jobs later in a career sees
//! players through the knowledge rule, not through this.

use serde::Serialize;
use ts_rs::TS;

use crate::board_objectives::{OpeningExpectation, opening_expectation};
use crate::slices::teams::TeamCardColors;
use domain::league::{CompetitionScope, CompetitionType};
use domain::player::Position;
use domain::team::Team;

use crate::game::Game;
use crate::slices::teams::{average_rating, players_by_club};
use domain::player::Player;

/// Every club's squad as the team-selection screen rates it; no club at all once a career has
/// begun.
#[derive(Debug, Serialize, PartialEq, Eq, TS)]
pub struct TeamSelectionRatings {
    pub clubs: Vec<ClubSquadRatings>,
}

/// One club's squad: its average rating (the club card's rule) and its players, best rated
/// first, so the first eleven are its likely starting side.
#[derive(Debug, Serialize, PartialEq, Eq, TS)]
pub struct ClubSquadRatings {
    pub team_id: String,
    pub average_rating: u8,
    pub players: Vec<RatedPlayer>,
    #[serde(default)]
    pub stadium_name: String,
    #[serde(default)]
    pub stadium_capacity: u32,
    /// Integer euros, as held by the club.
    #[serde(default)]
    pub transfer_budget: i64,
    /// Integer euros per week, as held by the club.
    #[serde(default)]
    pub wage_budget: i64,
    /// Cash on hand in integer euros; may be negative.
    #[serde(default)]
    pub balance: i64,
    #[serde(default)]
    pub colors: TeamCardColors,
    #[serde(default)]
    pub founded_year: u32,
    #[serde(default)]
    pub competitions: Vec<ClubCompetition>,
    #[serde(default)]
    pub key_players: Vec<KeyPlayer>,
    #[serde(default)]
    pub reputation_key: String,
    #[serde(default)]
    pub opening_expectation: OpeningExpectation,
}

#[derive(Debug, Serialize, PartialEq, Eq, TS)]
pub struct RatedPlayer {
    pub player_id: String,
    pub ovr: u8,
}

/// Public identity of a competition the club actually contests.
#[derive(Debug, Serialize, PartialEq, Eq, TS)]
pub struct ClubCompetition {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub name_key: Option<String>,
    #[serde(default)]
    pub season: u32,
    #[serde(default)]
    pub kind: CompetitionType,
    #[serde(default)]
    pub scope: CompetitionScope,
}

#[derive(Debug, Serialize, PartialEq, Eq, TS)]
pub struct KeyPlayer {
    #[serde(default)]
    pub player_id: String,
    #[serde(default)]
    pub name: String,
    /// Natural position, independent of the slot the club currently deploys the player in.
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub ovr: u8,
}

/// The existing club-selection reputation words, now decided alongside the club's facts.
fn reputation_key(reputation: u32) -> &'static str {
    match reputation {
        750.. => "teamSelect.repWorldClass",
        600.. => "teamSelect.repStrong",
        400.. => "teamSelect.repAverage",
        _ => "teamSelect.repDeveloping",
    }
}

fn club_competitions(game: &Game, club_id: &str) -> Vec<ClubCompetition> {
    game.competitions_in_play()
        .iter()
        .filter(|competition| Game::club_takes_part_in(competition, club_id))
        .map(|competition| ClubCompetition {
            id: competition.id.clone(),
            name: competition.name.clone(),
            name_key: competition.name_key.clone(),
            season: competition.season,
            kind: competition.kind.clone(),
            scope: competition.scope.clone(),
        })
        .collect()
}

fn project_club(game: &Game, team: &Team, squad: &[&Player]) -> ClubSquadRatings {
    ClubSquadRatings {
        team_id: team.id.clone(),
        average_rating: average_rating(squad),
        players: squad
            .iter()
            .map(|player| RatedPlayer {
                player_id: player.id.clone(),
                ovr: player.ovr,
            })
            .collect(),
        stadium_name: team.stadium_name.clone(),
        stadium_capacity: team.stadium_capacity,
        transfer_budget: team.transfer_budget,
        wage_budget: team.wage_budget,
        balance: team.finance,
        colors: TeamCardColors {
            primary: team.colors.primary.clone(),
            secondary: team.colors.secondary.clone(),
        },
        founded_year: team.founded_year,
        competitions: club_competitions(game, &team.id),
        key_players: squad
            .iter()
            .take(5)
            .map(|player| KeyPlayer {
                player_id: player.id.clone(),
                name: player.match_name.clone(),
                position: player.natural_position.clone(),
                ovr: player.ovr,
            })
            .collect(),
        reputation_key: reputation_key(team.reputation).into(),
        opening_expectation: opening_expectation(game, team),
    }
}

/// Before a career has begun: the manager has no club and has never had one.
fn before_a_career(game: &Game) -> bool {
    game.manager.team_id.is_none() && game.manager.career_history.is_empty()
}

/// Every club's squad as the team-selection screen rates it, before a career begins; no club once
/// one has begun (see [`TeamSelectionRatings`]). The players are grouped by club in one pass, since
/// this runs under the game's lock and a world holds thousands of them.
pub fn query_team_selection(game: &Game) -> TeamSelectionRatings {
    if !before_a_career(game) {
        return TeamSelectionRatings { clubs: Vec::new() };
    }
    let by_club = players_by_club(&game.players);
    let clubs = game
        .teams
        .iter()
        .map(|team| {
            let mut squad: Vec<&Player> =
                by_club.get(team.id.as_str()).cloned().unwrap_or_default();
            squad.sort_by(|left, right| right.ovr.cmp(&left.ovr).then(left.id.cmp(&right.id)));
            project_club(game, team, &squad)
        })
        .collect();
    TeamSelectionRatings { clubs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::{Manager, ManagerCareerEntry};
    use domain::player::Position;
    use domain::team::Team;

    fn team(id: &str) -> Team {
        Team::new(
            id.into(),
            id.into(),
            id.into(),
            "ENG".into(),
            "Town".into(),
            "Ground".into(),
            20_000,
        )
    }

    fn rated(id: &str, club: &str, ovr: u8) -> Player {
        let mut player = Player::new(
            id.into(),
            id.into(),
            id.into(),
            "2000-01-01".into(),
            "ENG".into(),
            Position::Midfielder,
            crate::test_support::uniform_attributes(60),
        );
        player.team_id = Some(club.into());
        player.ovr = ovr;
        player
    }

    /// A world before any career: two clubs, a fresh manager with no club.
    fn world() -> Game {
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        Game::new(
            clock,
            manager,
            vec![team("north"), team("south")],
            vec![
                rated("n1", "north", 70),
                rated("n2", "north", 81),
                rated("n3", "north", 74),
                rated("n4", "north", 73),
                rated("s1", "south", 60),
            ],
            vec![],
            vec![],
        )
    }

    /// Given distinctive club finances, colours and ground details, when selection is queried,
    /// then every fact comes from that club rather than a default or its neighbour.
    #[test]
    fn club_facts_come_from_the_selected_club() {
        let mut game = world();
        let club = &mut game.teams[0];
        club.stadium_name = "North End".into();
        club.stadium_capacity = 18_432;
        club.transfer_budget = 8_765_432;
        club.wage_budget = 123_456;
        club.finance = -654_321;
        club.colors.primary = "#123456".into();
        club.colors.secondary = "#abcdef".into();
        club.founded_year = 1897;
        let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
        let facts = &wire["clubs"][0];
        assert_eq!(facts["stadium_name"], "North End");
        assert_eq!(facts["stadium_capacity"], 18_432);
        assert_eq!(facts["transfer_budget"], 8_765_432);
        assert_eq!(facts["wage_budget"], 123_456);
        assert_eq!(facts["balance"], -654_321);
        assert_eq!(facts["colors"]["primary"], "#123456");
        assert_eq!(facts["colors"]["secondary"], "#abcdef");
        assert_eq!(facts["founded_year"], 1897);
    }

    /// Given more than five players, a tie, and a better rival, when selection is queried,
    /// then the five strongest own players carry their real position, name and OVR in stable order.
    #[test]
    fn key_players_are_the_best_five_of_the_clubs_own_squad() {
        let mut game = world();
        game.players.extend([
            rated("n5", "north", 81),
            rated("n6", "north", 69),
            rated("n7", "north", 72),
            rated("rival", "south", 99),
        ]);
        game.players[1].position = Position::Defender;
        game.players[1].natural_position = Position::Striker;
        game.players[1].match_name = "Ada North".into();
        game.players[1].full_name = "Ada Maria North".into();
        let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
        let keys = wire["clubs"][0]["key_players"]
            .as_array()
            .expect("key players");
        assert_eq!(keys.len(), 5);
        let ids: Vec<_> = keys
            .iter()
            .map(|p| p["player_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["n2", "n5", "n3", "n4", "n7"]);
        assert_eq!(keys[0]["name"], "Ada North");
        assert_eq!(keys[0]["position"], "Striker");
        assert_eq!(keys[0]["ovr"], 81);
    }

    /// Given empty and short squads, when selection is queried, then key players are not padded.
    #[test]
    fn short_and_empty_squads_only_return_existing_key_players() {
        let mut game = world();
        game.players
            .retain(|player| player.team_id.as_deref() == Some("south"));
        let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
        assert_eq!(wire["clubs"][0]["key_players"], serde_json::json!([]));
        assert_eq!(wire["clubs"][1]["key_players"].as_array().unwrap().len(), 1);
    }

    /// Given league, cup and continental entries plus an unrelated cup and a legacy mirror,
    /// when selection is queried, then membership and translated competition metadata are preserved.
    #[test]
    fn competitions_are_only_the_clubs_actual_entries() {
        use domain::league::{CompetitionType, League};
        let mut game = world();
        let mut league = League::new(
            "league".into(),
            "Northern League".into(),
            2026,
            &["north".into()],
        );
        league.name_key = Some("custom.league".into());
        let mut cup = League::new("cup".into(), "Northern Cup".into(), 2026, &["north".into()]);
        cup.kind = CompetitionType::Cup;
        let mut continental = cup.clone();
        continental.id = "continental".into();
        continental.kind = CompetitionType::ContinentalClub;
        continental.scope = domain::league::CompetitionScope::Continental;
        let mut unrelated = cup.clone();
        unrelated.id = "other".into();
        unrelated.participant_ids = vec!["south".into()];
        unrelated.standings.clear();
        game.league = Some(league.clone());
        game.competitions = vec![league, cup, continental, unrelated];
        let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
        let entries = wire["clubs"][0]["competitions"]
            .as_array()
            .expect("competitions");
        let ids: Vec<_> = entries.iter().map(|c| c["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["league", "cup", "continental"]);
        assert_eq!(entries[0]["name"], "Northern League");
        assert_eq!(entries[0]["name_key"], "custom.league");
        assert_eq!(entries[0]["season"], 2026);
        assert_eq!(entries[0]["kind"], "League");
        assert_eq!(entries[1]["kind"], "Cup");
        assert_eq!(entries[2]["kind"], "ContinentalClub");
        assert_eq!(entries[2]["scope"], "Continental");
    }

    /// Given a legacy league with no modern competition list, when selection is queried,
    /// then its standings still identify the club's competition without inventing any entries.
    #[test]
    fn legacy_competitions_use_standings_when_participants_are_absent() {
        use domain::league::League;
        let mut game = world();
        let mut league = League::new(
            "legacy".into(),
            "Legacy League".into(),
            2026,
            &["north".into()],
        );
        league.participant_ids.clear();
        game.league = Some(league);
        let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
        assert_eq!(wire["clubs"][0]["competitions"][0]["id"], "legacy");
        assert_eq!(wire["clubs"][1]["competitions"], serde_json::json!([]));
    }

    /// Given clubs of every reputation, an unrelated mirror and a smaller later split table,
    /// when each club is hired and objectives generated, then its preview has that exact target.
    #[test]
    fn opening_expectations_match_the_board_after_hiring() {
        use domain::league::League;
        for reputation in [0, 399, 400, 499, 500, 649, 650, 799, 800, 1000] {
            let mut game = world();
            game.teams[0].reputation = reputation;
            let ids: Vec<_> = (0..20)
                .map(|i| {
                    if i == 0 {
                        "north".into()
                    } else {
                        format!("club{i}")
                    }
                })
                .collect();
            let league = League::new("north-league".into(), "League".into(), 2026, &ids);
            let other = League::new(
                "other-league".into(),
                "Other".into(),
                2026,
                &["south".into(), "rival".into()],
            );
            let later_split =
                League::new("later-split".into(), "Later split".into(), 2026, &ids[..4]);
            game.league = Some(other.clone());
            game.competitions = vec![other, league, later_split];
            let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
            let preview = &wire["clubs"][0]["opening_expectation"];
            crate::job_offers::hire_manager(&mut game, "north", "2026-07-01").unwrap();
            crate::board_objectives::generate_objectives(&mut game);
            let target = game
                .board_objectives
                .iter()
                .find(|o| o.objective_type == crate::game::ObjectiveType::LeaguePosition)
                .unwrap()
                .target;
            assert_eq!(
                preview["target_position"], target,
                "reputation {reputation}"
            );
            let (expected_target, word) = match reputation {
                800.. => (1, "title"),
                650.. => (5, "topHalf"),
                400.. => (10, "midTable"),
                _ => (15, "survival"),
            };
            assert_eq!(target, expected_target);
            assert_eq!(
                preview["label_key"],
                format!("teamSelect.expectations.{word}")
            );
        }
    }

    /// Given a top-quarter target that earns a continental berth, when selection is queried,
    /// then its label names continental qualification without changing the board target.
    #[test]
    fn a_continental_expectation_requires_a_berth_at_the_target_position() {
        use domain::league::{Berth, BerthRule, CompetitionType, League};
        for (last_qualifying_position, word) in [(4, "topHalf"), (5, "continental")] {
            let mut game = world();
            game.teams[0].reputation = 650;
            let ids: Vec<_> = (0..20)
                .map(|i| {
                    if i == 0 {
                        "north".into()
                    } else {
                        format!("club{i}")
                    }
                })
                .collect();
            let mut league = League::new("league".into(), "League".into(), 2026, &ids);
            league.berths.push(Berth {
                target: "continental".into(),
                rule: BerthRule::PositionRange {
                    from: 1,
                    to: last_qualifying_position,
                },
                fallback_to: None,
            });
            let mut continental =
                League::new("continental".into(), "Champions Cup".into(), 2026, &[]);
            continental.kind = CompetitionType::ContinentalClub;
            game.competitions = vec![league, continental];
            let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
            let preview = &wire["clubs"][0]["opening_expectation"];
            assert_eq!(preview["target_position"], 5);
            assert_eq!(
                preview["label_key"],
                format!("teamSelect.expectations.{word}")
            );
        }
    }

    /// Given a generated two-country world, when a career starts through its real bootstrap,
    /// then the opening preview and the first board objective name the same league target.
    #[test]
    fn opening_expectation_survives_the_real_career_bootstrap() {
        let mut game = world();
        game.teams[0].reputation = 650;
        for id in ["brazil-a", "brazil-b"] {
            let mut club = team(id);
            club.country = "BR".into();
            club.football_nation = "BR".into();
            game.teams.push(club);
        }
        crate::world::ensure_multi_competition_foundations(&mut game);
        let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
        let preview = &wire["clubs"][0]["opening_expectation"]["target_position"];
        crate::career::begin_career(
            &mut game,
            "north",
            crate::career::CareerScope::default(),
            domain::stats::StatsState::default(),
        )
        .unwrap();
        crate::board_objectives::generate_objectives(&mut game);
        let target = game
            .board_objectives
            .iter()
            .find(|o| o.objective_type == crate::game::ObjectiveType::LeaguePosition)
            .unwrap()
            .target;
        assert_eq!(*preview, target);
    }

    /// Given the existing reputation boundaries, when selection is queried,
    /// then the backend selects the same translated reputation word the screen already uses.
    #[test]
    fn reputation_words_follow_the_existing_boundaries() {
        for (reputation, key) in [
            (0, "repDeveloping"),
            (399, "repDeveloping"),
            (400, "repAverage"),
            (599, "repAverage"),
            (600, "repStrong"),
            (749, "repStrong"),
            (750, "repWorldClass"),
            (1000, "repWorldClass"),
        ] {
            let mut game = world();
            game.teams[0].reputation = reputation;
            let wire = serde_json::to_value(query_team_selection(&game)).unwrap();
            assert_eq!(
                wire["clubs"][0]["reputation_key"],
                format!("teamSelect.{key}")
            );
        }
    }

    /// Given a world before any career
    /// When the team-selection screen asks for the squads
    /// Then every club comes with its players best rated first and its average by the club card's
    ///      rule, rounded down: 70, 81, 74 and 73 average 74.5, shown as 74
    #[test]
    fn before_a_career_every_club_is_rated() {
        let ratings = query_team_selection(&world());

        let north = &ratings.clubs[0];
        assert_eq!(north.team_id, "north");
        let order: Vec<(&str, u8)> = north
            .players
            .iter()
            .map(|rated| (rated.player_id.as_str(), rated.ovr))
            .collect();
        assert_eq!(order, vec![("n2", 81), ("n3", 74), ("n4", 73), ("n1", 70)]);
        assert_eq!(north.average_rating, 74);
        assert_eq!(ratings.clubs[1].average_rating, 60);
    }

    /// Given the manager has taken a club
    /// When the squads are asked for
    /// Then none comes: from then on the club's knowledge decides what he sees
    #[test]
    fn once_a_career_has_a_club_nothing_is_rated_here() {
        let mut game = world();
        game.manager.hire("north".into());

        assert!(query_team_selection(&game).clubs.is_empty());
    }

    /// Given a manager between jobs later in his career (no club, but a career behind him)
    /// When the squads are asked for
    /// Then none comes: being out of work is not a new career's team selection
    #[test]
    fn a_manager_between_jobs_is_not_shown_every_rating() {
        let mut game = world();
        game.manager.career_history.push(ManagerCareerEntry {
            team_id: "north".into(),
            team_name: "north".into(),
            start_date: "2024-07-01".into(),
            end_date: Some("2026-05-30".into()),
            matches: 76,
            wins: 30,
            draws: 20,
            losses: 26,
            best_league_position: Some(5),
        });

        assert!(query_team_selection(&game).clubs.is_empty());
    }
}
