//! What a player is worth at each position he plays, as the club may see it: his rating there, how
//! he stands there (natural, can play, out of position) and the three attributes the position cannot
//! do without, with his values. Never a verdict on how well he fits (maintainer, 21 Oct): the
//! numbers and the familiarity word are all the screen says.

use domain::player::{AttributeKey, Player, PlayerAttributes, Position};
use serde::Serialize;
use ts_rs::TS;

use crate::game::Game;
use crate::player_knowledge::{PlayerKnowledge, player_knowledge};
use crate::player_rating::{
    Familiarity, canonical_position, deployed_position, familiarity, key_attributes,
    ovr_from_attributes, primary_position,
};

/// One attribute a position cannot do without, and the player's value in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct KeyAttribute {
    pub attribute: AttributeKey,
    pub value: u8,
}

/// The player at one position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct PositionRating {
    pub position: Position,
    /// What his attributes are worth there (`ovr_from_attributes`), rounded as his rating is.
    pub rating: u8,
    pub familiarity: Familiarity,
    /// The slot the manager plays him in: only ever for the club's own player.
    pub deployed: bool,
    pub key_attributes: Vec<KeyAttribute>,
}

/// Whose attributes the ratings are read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RatingsSource {
    /// His own, as they are today: the club's own player or a rival in its division.
    Live,
    /// A scout's report, as it was on `scouted_on`; not `current` once a season has ended since.
    Report { scouted_on: String, current: bool },
}

/// His ratings at his natural position, then each alternate, then (for the club's own player) the
/// slot he is played in when it is neither.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct PositionRatings {
    pub source: RatingsSource,
    pub rows: Vec<PositionRating>,
}

/// The club's view of `player` at each position he plays, through the knowledge rule: the club's
/// own player and a division rival from his live attributes, a scouted player from the report's (so
/// an old report gives old ratings, and says so), and nobody else at all.
pub fn position_ratings(game: &Game, player: &Player) -> Option<PositionRatings> {
    let (attributes, source, is_own) = match player_knowledge(game, player) {
        PlayerKnowledge::Own => (&player.attributes, RatingsSource::Live, true),
        PlayerKnowledge::Known => (&player.attributes, RatingsSource::Live, false),
        PlayerKnowledge::Scouted {
            scouted_on,
            current,
        } => {
            let report = game
                .scouted_players
                .iter()
                .find(|report| report.player_id == player.id)?;
            (
                &report.attributes,
                RatingsSource::Report {
                    scouted_on,
                    current,
                },
                false,
            )
        }
        PlayerKnowledge::Unknown => return None,
    };
    let deployed = is_own.then(|| deployed_in_own_team(game, player)).flatten();
    let rows = positions_he_plays(player, deployed.as_ref())
        .into_iter()
        .map(|position| PositionRating {
            rating: ovr_from_attributes(attributes, &position).round() as u8,
            familiarity: familiarity(player, &position),
            deployed: deployed.as_ref() == Some(&position),
            key_attributes: key_attributes_of(attributes, &position),
            position,
        })
        .collect();
    Some(PositionRatings { source, rows })
}

/// The slot the manager's own team plays him in (`deployed_position`, the derivation the match
/// lineup is built from), if he is in its starting XI.
fn deployed_in_own_team(game: &Game, player: &Player) -> Option<Position> {
    let club = game.manager.team_id.as_deref()?;
    let team = game.teams.iter().find(|team| team.id == club)?;
    deployed_position(team, &player.id)
}

/// His natural position, his alternates, and the deployed slot when it is none of those; each once,
/// granular.
fn positions_he_plays(player: &Player, deployed: Option<&Position>) -> Vec<Position> {
    let mut positions = vec![primary_position(player)];
    let alternates = player
        .alternate_positions
        .iter()
        .chain(deployed)
        .map(canonical_position);
    for position in alternates {
        if !positions.contains(&position) {
            positions.push(position);
        }
    }
    positions
}

fn key_attributes_of(attributes: &PlayerAttributes, position: &Position) -> Vec<KeyAttribute> {
    key_attributes(position)
        .into_iter()
        .map(|attribute| KeyAttribute {
            attribute,
            value: attribute.value_in(attributes),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::game::ScoutedPlayer;
    use crate::test_support::uniform_attributes;
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::team::Team;

    const MY_CLUB: &str = "mine";

    /// A central midfielder who can also play defensive midfield, every attribute `level`.
    fn midfielder(id: &str, team_id: &str, level: u8) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            uniform_attributes(level),
        );
        player.natural_position = Position::CentralMidfielder;
        player.alternate_positions = vec![Position::DefensiveMidfielder];
        player.team_id = Some(team_id.to_string());
        player
    }

    fn team(id: &str, starting_xi: &[&str]) -> Team {
        let mut team = Team::new(
            id.to_string(),
            id.to_string(),
            id[..3.min(id.len())].to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            25_000,
        );
        team.formation = "4-4-2".to_string();
        team.starting_xi_ids = starting_xi.iter().map(|id| id.to_string()).collect();
        team
    }

    /// My club and "rival" share division one; "faraway" plays in division two.
    fn world(players: Vec<Player>, teams: Vec<Team>) -> Game {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        manager.team_id = Some(MY_CLUB.to_string());
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
        let division = |id: &str, clubs: &[&str]| {
            let clubs: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
            League::new(id.to_string(), id.to_string(), 2026, &clubs)
        };
        game.competitions = vec![
            division("division-one", &[MY_CLUB, "rival"]),
            division("division-two", &["faraway", "other"]),
        ];
        game
    }

    fn ratings_of(game: &Game, player_id: &str) -> Option<PositionRatings> {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        position_ratings(game, player)
    }

    fn row(ratings: &PositionRatings, position: Position) -> &PositionRating {
        ratings
            .rows
            .iter()
            .find(|row| row.position == position)
            .unwrap_or_else(|| panic!("no {position:?} row in {ratings:?}"))
    }

    /// The rating `attributes` give at `position`, rounded as a rating is.
    fn rated(attributes: &PlayerAttributes, position: Position) -> u8 {
        ovr_from_attributes(attributes, &position).round() as u8
    }

    /// Given the club's own central midfielder (who can also play defensive midfield), played at
    ///       right back (the fifth slot of its 4-4-2)
    /// When his position ratings are read
    /// Then they are his natural position, his alternate and the slot he is played in, in that
    ///      order: natural, can play, out of position (deployed), each rated from his attributes
    ///      with its three key attributes
    #[test]
    fn the_clubs_own_player_is_rated_where_he_plays_and_where_he_is_played() {
        let own = midfielder("own", MY_CLUB, 70);
        let game = world(
            vec![own.clone()],
            vec![team(MY_CLUB, &["gk", "lb", "cb1", "cb2", "own"])],
        );

        let ratings = ratings_of(&game, "own").unwrap();

        let positions: Vec<_> = ratings
            .rows
            .iter()
            .map(|row| row.position.clone())
            .collect();
        assert_eq!(
            positions,
            vec![
                Position::CentralMidfielder,
                Position::DefensiveMidfielder,
                Position::RightBack
            ]
        );
        let right_back = row(&ratings, Position::RightBack);
        assert_eq!(right_back.familiarity, Familiarity::OutOfPosition);
        assert!(right_back.deployed);
        assert_eq!(
            right_back.rating,
            rated(&own.attributes, Position::RightBack)
        );
        assert_eq!(
            row(&ratings, Position::DefensiveMidfielder).familiarity,
            Familiarity::CanPlay
        );
        let natural = row(&ratings, Position::CentralMidfielder);
        assert_eq!(natural.familiarity, Familiarity::Natural);
        assert!(!natural.deployed);
        assert_eq!(
            natural.key_attributes,
            vec![
                KeyAttribute {
                    attribute: AttributeKey::Passing,
                    value: 70
                },
                KeyAttribute {
                    attribute: AttributeKey::Vision,
                    value: 70
                },
                KeyAttribute {
                    attribute: AttributeKey::Decisions,
                    value: 70
                },
            ]
        );
        assert_eq!(ratings.source, RatingsSource::Live);
    }

    /// Given the club's own midfielder played at right back
    /// When his position ratings are written for the screens
    /// Then they come in the documented shape: a tagged source, snake_case familiarity words and
    ///      attribute names spelled as the screens' attribute labels are
    #[test]
    fn position_ratings_reach_the_screens_in_the_documented_shape() {
        let own = midfielder("own", MY_CLUB, 70);
        let game = world(
            vec![own],
            vec![team(MY_CLUB, &["gk", "lb", "cb1", "cb2", "own"])],
        );

        let json = serde_json::to_value(ratings_of(&game, "own").unwrap()).unwrap();

        assert_eq!(json["source"], serde_json::json!({ "kind": "live" }));
        assert_eq!(json["rows"][0]["position"], "CentralMidfielder");
        assert_eq!(json["rows"][0]["familiarity"], "natural");
        assert_eq!(json["rows"][1]["familiarity"], "can_play");
        assert_eq!(json["rows"][2]["familiarity"], "out_of_position");
        assert_eq!(json["rows"][2]["deployed"], true);
        assert_eq!(
            json["rows"][0]["key_attributes"][0],
            serde_json::json!({ "attribute": "passing", "value": 70 })
        );
    }

    /// Given the club's own midfielder played in central midfield, his natural position
    /// When his position ratings are read
    /// Then no extra row appears: his natural row is the one marked deployed
    #[test]
    fn a_player_played_in_his_natural_position_gets_no_extra_row() {
        let own = midfielder("own", MY_CLUB, 70);
        let xi = ["gk", "lb", "cb1", "cb2", "rb", "lm", "own"];
        let game = world(vec![own], vec![team(MY_CLUB, &xi)]);

        let ratings = ratings_of(&game, "own").unwrap();

        assert_eq!(ratings.rows.len(), 2);
        assert!(row(&ratings, Position::CentralMidfielder).deployed);
    }

    /// Given a rival in the club's division, in his own club's XI at right back
    /// When his position ratings are read
    /// Then they are his natural position and alternate only, from his live attributes: where
    ///      another club plays him is not the club's to mark
    #[test]
    fn a_division_rival_is_rated_where_he_plays_but_not_where_his_club_plays_him() {
        let rival = midfielder("rival-player", "rival", 66);
        let game = world(
            vec![rival],
            vec![team("rival", &["gk", "lb", "cb1", "cb2", "rival-player"])],
        );

        let ratings = ratings_of(&game, "rival-player").unwrap();

        assert_eq!(ratings.rows.len(), 2);
        assert!(ratings.rows.iter().all(|row| !row.deployed));
        assert_eq!(ratings.source, RatingsSource::Live);
        assert_eq!(
            row(&ratings, Position::CentralMidfielder).rating,
            rated(&uniform_attributes(66), Position::CentralMidfielder)
        );
    }

    /// A world holding "far", in another division, live attributes 80 everywhere, whose scout
    /// rated them 40 everywhere on `scouted_on`; the last season ended on 25 May.
    fn scouted_far(scouted_on: &str) -> Game {
        let far = midfielder("far", "faraway", 80);
        let mut game = world(vec![far], vec![]);
        game.last_season_rollover_on = Some("2026-05-25".to_string());
        game.scouted_players.push(ScoutedPlayer {
            player_id: "far".to_string(),
            scouted_on: scouted_on.to_string(),
            attributes: uniform_attributes(40),
            potential: None,
            scout_name: None,
        });
        game
    }

    /// Given a player in another division, live attributes 80, whose scout reported 40 this season
    /// When his position ratings are read
    /// Then every rating and key attribute comes from the report (40), never his live 80, and the
    ///      ratings say whose they are: the report's, dated, current
    #[test]
    fn a_scouted_player_is_rated_from_the_report_never_his_live_attributes() {
        let game = scouted_far("2026-09-01");

        let ratings = ratings_of(&game, "far").unwrap();

        assert_eq!(
            ratings.source,
            RatingsSource::Report {
                scouted_on: "2026-09-01".to_string(),
                current: true,
            }
        );
        for row in &ratings.rows {
            assert_eq!(
                row.rating,
                rated(&uniform_attributes(40), row.position.clone())
            );
            assert!(
                row.key_attributes.iter().all(|key| key.value == 40),
                "{row:?}"
            );
        }
        assert_ne!(
            rated(&uniform_attributes(40), Position::CentralMidfielder),
            rated(&uniform_attributes(80), Position::CentralMidfielder),
            "the fixture tells the report from the live player"
        );
    }

    /// Given the same report, written before the season ended
    /// When his position ratings are read
    /// Then they are still the report's, marked not current
    #[test]
    fn an_old_reports_ratings_say_they_are_out_of_date() {
        let game = scouted_far("2026-03-10");

        let ratings = ratings_of(&game, "far").unwrap();

        assert_eq!(
            ratings.source,
            RatingsSource::Report {
                scouted_on: "2026-03-10".to_string(),
                current: false,
            }
        );
    }

    /// Given a player in another division, never scouted
    /// When his position ratings are read
    /// Then there are none: the club knows nothing of his attributes
    #[test]
    fn an_unknown_player_has_no_position_ratings() {
        let far = midfielder("far", "faraway", 80);
        let game = world(vec![far], vec![]);

        assert_eq!(ratings_of(&game, "far"), None);
    }
}
