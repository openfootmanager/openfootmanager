//! A player who has stopped playing: who he was, what he became, and when he left.

use serde::{Deserialize, Serialize};

use crate::player::{
    CareerEntry, Footedness, Player, PlayerAttributes, PlayerMedia, PlayerMovementEntry,
    PlayerMovementKind, Position,
};

/// The frozen record of a retired player. It keeps what the game still shows about
/// a person (name, nationality, career, final ability) and drops everything that
/// only matters while he plays: offers, morale, training, injury, season stats.
///
/// Once archived it is never rewritten; the archive is keyed by `id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetiredPlayer {
    pub id: String,
    pub match_name: String,
    pub full_name: String,
    pub date_of_birth: String,
    pub nationality: String,
    #[serde(default)]
    pub football_nation: String,
    #[serde(default)]
    pub birth_country: Option<String>,
    #[serde(default)]
    pub media: PlayerMedia,
    pub natural_position: Position,
    #[serde(default)]
    pub alternate_positions: Vec<Position>,
    #[serde(default)]
    pub footedness: Footedness,
    pub weak_foot: u8,
    /// Final ability, as it was on the day he retired.
    pub attributes: PlayerAttributes,
    pub ovr: u8,
    pub potential: u8,
    pub career: Vec<CareerEntry>,
    pub movements: Vec<PlayerMovementEntry>,
    pub retired_on: String,
    pub retired_season: u32,
    /// The club he left the game from, when he had one.
    #[serde(default)]
    pub last_team_id: Option<String>,
}

impl RetiredPlayer {
    pub fn from_player(player: &Player, retired_on: &str, retired_season: u32) -> Self {
        let movements: Vec<PlayerMovementEntry> =
            player.movement_history.into_iter().cloned().collect();
        let last_team_id = movements
            .iter()
            .rev()
            .find(|movement| movement.kind == PlayerMovementKind::Retired)
            .and_then(|movement| movement.from_team_id.clone())
            .or_else(|| player.team_id.clone());
        Self {
            id: player.id.clone(),
            match_name: player.match_name.clone(),
            full_name: player.full_name.clone(),
            date_of_birth: player.date_of_birth.clone(),
            nationality: player.nationality.clone(),
            football_nation: player.football_nation.clone(),
            birth_country: player.birth_country.clone(),
            media: player.media.clone(),
            natural_position: player.natural_position.clone(),
            alternate_positions: player.alternate_positions.clone(),
            footedness: player.footedness,
            weak_foot: player.weak_foot,
            attributes: player.attributes.clone(),
            ovr: player.ovr,
            potential: player.potential,
            career: player.career.clone(),
            movements,
            retired_on: retired_on.to_string(),
            retired_season,
            last_team_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::SquadRole;

    fn attributes(value: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: value,
            stamina: value,
            strength: value,
            agility: value,
            passing: value,
            shooting: value,
            tackling: value,
            dribbling: value,
            defending: value,
            positioning: value,
            vision: value,
            decisions: value,
            composure: value,
            aggression: value,
            teamwork: value,
            leadership: value,
            handling: value,
            reflexes: value,
            aerial: value,
        }
    }

    /// A veteran with every field the archive keeps set to a non-default value.
    fn veteran() -> Player {
        let mut player = Player::new(
            "p-veteran".into(),
            "J. Veteran".into(),
            "John Veteran".into(),
            "1988-04-02".into(),
            "Portugal".into(),
            Position::Midfielder,
            attributes(63),
        );
        player.natural_position = Position::CentralMidfielder;
        player.alternate_positions = vec![Position::DefensiveMidfielder];
        player.footedness = Footedness::Left;
        player.weak_foot = 4;
        player.ovr = 71;
        player.potential = 78;
        player.media.face = Some("faces/veteran.png".into());
        player.career.push(CareerEntry {
            season: 2031,
            team_id: "t-1".into(),
            team_name: "Porto".into(),
            appearances: 31,
            goals: 4,
            assists: 9,
        });
        player
            .record_movement(PlayerMovementEntry {
                from_team_id: Some("t-1".into()),
                ..PlayerMovementEntry::new("2035-06-30", PlayerMovementKind::Retired)
            })
            .expect("a retirement is always recordable");
        // Things only a playing career has.
        player.team_id = None;
        player.morale = 12;
        player.squad_role = SquadRole::Youth;
        player
    }

    /// Given a retired player, when archived, then who he is survives.
    #[test]
    fn the_archive_keeps_a_players_identity() {
        let player = veteran();
        let record = RetiredPlayer::from_player(&player, "2035-06-30", 2035);
        assert_eq!(record.id, "p-veteran");
        assert_eq!(record.match_name, "J. Veteran");
        assert_eq!(record.full_name, "John Veteran");
        assert_eq!(record.date_of_birth, "1988-04-02");
        assert_eq!(record.nationality, "Portugal");
        assert_eq!(record.football_nation, player.football_nation);
        assert_eq!(record.birth_country, player.birth_country);
        assert_eq!(record.media.face.as_deref(), Some("faces/veteran.png"));
    }

    /// Given a retired player, when archived, then his final ability and positions survive.
    #[test]
    fn the_archive_keeps_final_ability_and_positions() {
        let record = RetiredPlayer::from_player(&veteran(), "2035-06-30", 2035);
        assert_eq!(record.natural_position, Position::CentralMidfielder);
        assert_eq!(
            record.alternate_positions,
            vec![Position::DefensiveMidfielder]
        );
        assert_eq!(record.footedness, Footedness::Left);
        assert_eq!(record.weak_foot, 4);
        assert_eq!((record.ovr, record.potential), (71, 78));
        assert_eq!(record.attributes.pace, 63);
    }

    /// Given a career and movements, when archived, then both survive in order and the
    /// retirement itself is dated.
    #[test]
    fn the_archive_keeps_the_career_and_the_movements() {
        let record = RetiredPlayer::from_player(&veteran(), "2035-06-30", 2035);
        assert_eq!(record.career.len(), 1);
        assert_eq!(record.career[0].appearances, 31);
        assert_eq!(record.movements.len(), 1);
        assert_eq!(record.movements[0].kind, PlayerMovementKind::Retired);
        assert_eq!(record.retired_on, "2035-06-30");
        assert_eq!(record.retired_season, 2035);
    }

    /// Given a retirement recorded from a club, when archived, then that club is his last.
    #[test]
    fn the_archive_names_the_last_club() {
        let record = RetiredPlayer::from_player(&veteran(), "2035-06-30", 2035);
        assert_eq!(record.last_team_id.as_deref(), Some("t-1"));
    }

    /// Given a player who never had a club on record, when archived, then there is no last club.
    #[test]
    fn a_player_without_a_club_has_no_last_club() {
        let mut player = veteran();
        player.career.clear();
        player.movement_history = Default::default();
        let record = RetiredPlayer::from_player(&player, "2035-06-30", 2035);
        assert_eq!(record.last_team_id, None);
    }

    /// Given an archived record, when serialized and read back, then it is equal.
    #[test]
    fn the_record_round_trips_through_json() {
        let record = RetiredPlayer::from_player(&veteran(), "2035-06-30", 2035);
        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(
            serde_json::from_str::<RetiredPlayer>(&json).unwrap(),
            record
        );
    }

    /// Given a record written before a later field existed, when read, then the field defaults.
    #[test]
    fn an_older_record_without_optional_fields_still_loads() {
        let record = RetiredPlayer::from_player(&veteran(), "2035-06-30", 2035);
        let mut value = serde_json::to_value(&record).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("last_team_id");
        object.remove("football_nation");
        object.remove("alternate_positions");
        let loaded: RetiredPlayer = serde_json::from_value(value).unwrap();
        assert_eq!(loaded.last_team_id, None);
        assert!(loaded.alternate_positions.is_empty());
    }
}
