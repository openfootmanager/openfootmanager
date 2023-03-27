#      Openfoot Manager - A free and open source soccer management simulation
#      Copyright (C) 2020-2023  Pedrenrique G. Guimarães
#
#      This program is free software: you can redistribute it and/or modify
#      it under the terms of the GNU General Public License as published by
#      the Free Software Foundation, either version 3 of the License, or
#      (at your option) any later version.
#
#      This program is distributed in the hope that it will be useful,
#      but WITHOUT ANY WARRANTY; without even the implied warranty of
#      MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#      GNU General Public License for more details.
#
#      You should have received a copy of the GNU General Public License
#      along with this program.  If not, see <https://www.gnu.org/licenses/>.
import datetime
from dataclasses import asdict, dataclass
from enum import Enum, IntEnum, auto
from typing import Union
from uuid import UUID

from .playercontract import PlayerContract


class Positions(IntEnum):
    GK = auto()
    DF = auto()
    MF = auto()
    FW = auto()


class PreferredFoot(IntEnum):
    LEFT = auto()
    RIGHT = auto()
    BOTH = auto()


def get_players_from_dict_list(players_dict: list) -> list:
    return [Player.get_from_dict(player) for player in players_dict]


def get_positions_from_dict(positions: list[int]):
    return [Positions(position) for position in positions]


@dataclass
class PlayerAttributes:
    offense: int
    defense: int
    passing: int
    gk: int

    @classmethod
    def get_from_dict(cls, attributes: dict[str, int]):
        return cls(**attributes)

    def serialize(self) -> dict[str, int]:
        return asdict(self)


@dataclass
class Player:
    """
    Parameters
    ----------
    player_id: UUID
        Player's unique ID in the database.
    nationality: str
        Player's nationality.
    dob: datetime.datetime or datetime.date
        Player's date of birth, used to get the player's age.
    first_name: str
    last_name: str
    short_name: str
        How the player is called commonly. Some players have nicknames that have nothing to do with their real names,
        such as Pelé, Kaká, Ronaldinho, Pepe, etc.
    positions: list[Positions]
        The positions that the player can play. Currently, we are only implementing the FW, MF, DF and GK. Players can
        only play as such.
    fitness: float
        Indicates how much the player is ready for a game. This also relates to how likely the player can get injured,
        and how fast his stamina drops. Values go from 0.0 to 100.0, with higher values meaning that the player
        is less likely to get injured, has less stamina issues and can recover quickly between games. This value can be
        improved with training.
    stamina: float
        Indicates the current stamina of the player in the game session. Values go from 0.0 to 100.0, with higher
        values meaning that the player can still perform well in a game. The longer the player stays in a game,
        the more the stamina drops. Lower stamina values in a game can lead to injuries and can lower player's fitness,
        and the player takes longer to recover after a game.
    form: float
        Indicates how confident the player is for a game. Values range from 0.0 to 1.0. Higher values indicate that
        the player is more confident, and can perform better in a game. It can be improved after winning important
        games, scoring streaks, getting a good performance/rating in a game or winning a title.
    skill: dict
        Set of skills that a player possesses, such as attacking, defending, midfield and goalkeeper skills. The higher
        the values, the better the player performs in such areas. Skill values range from 0 to 99.
    potential_skill: dict
        Indicates potential values that a player can reach with training. These are only metrics that are used
        to calculate player's prospects. The values have the same range as the skill values.
    international_reputation: int
        Indicates how the player is valued outside his country. Values range from 0 to 5. Players with higher
        values can perform better in matches, and are more valuable than others with lower scores.
    preferred_foot: PreferredFoot
        The player's preferred foot for shooting. Can have an impact in goal scoring.
    value: float
        How much the player is currently valued on the market. Takes into account the player's age, performance,
        form, skill, potential skill and international reputation.
    """

    player_id: UUID
    nationality: str
    dob: Union[datetime.datetime, datetime.date]
    first_name: str
    last_name: str
    short_name: str
    positions: list[Positions]
    fitness: float
    stamina: float
    form: float
    attributes: PlayerAttributes
    potential_attributes: PlayerAttributes
    international_reputation: int
    preferred_foot: PreferredFoot
    value: float

    @classmethod
    def get_from_dict(cls, player_dict: dict):
        return cls(
            UUID(int=player_dict["id"]),
            player_dict["nationality"],
            datetime.datetime.strptime(player_dict["dob"], "%Y-%m-%d").date(),
            player_dict["first_name"],
            player_dict["last_name"],
            player_dict["short_name"],
            get_positions_from_dict(player_dict["positions"]),
            player_dict["fitness"],
            player_dict["stamina"],
            player_dict["form"],
            PlayerAttributes.get_from_dict(player_dict["attributes"]),
            PlayerAttributes.get_from_dict(player_dict["potential_attributes"]),
            player_dict["international_reputation"],
            PreferredFoot(player_dict["preferred_foot"]),
            player_dict["value"],
        )

    def get_position_values(self):
        return [position.value for position in self.positions]

    def get_best_position(self) -> Positions:
        best_pos = max(self.attributes.serialize())
        match best_pos:
            case "atk":
                return Positions.FW
            case "mid":
                return Positions.MF
            case "def":
                return Positions.DF
            case "gk":
                return Positions.GK

    def serialize(self) -> dict:
        return {
            "id": self.player_id.int,
            "nationality": self.nationality,
            "dob": self.dob.strftime("%Y-%m-%d"),
            "first_name": self.first_name,
            "last_name": self.last_name,
            "short_name": self.short_name,
            "positions": self.get_position_values(),
            "fitness": self.fitness,
            "stamina": self.stamina,
            "form": self.form,
            "attributes": self.attributes.serialize(),
            "potential_attributes": self.potential_attributes.serialize(),
            "international_reputation": self.international_reputation,
            "preferred_foot": self.preferred_foot.value,
            "value": self.value,
        }


@dataclass
class PlayerStats:
    player_id: UUID
    shots: int = 0
    assists: int = 0
    fouls: int = 0
    goals: int = 0
    goals_conceded: int = 0  # only for GK
    own_goals: int = 0
    penalties: int = 0
    injuries: int = 0
    yellow_cards: int = 0
    red_cards: int = 0
    avg_rating: float = 0.0
    win_streak: int = 0


@dataclass
class PlayerTeam:
    details: Player
    team_id: Union[UUID, None]
    shirt_number: int
    contract: PlayerContract

    @classmethod
    def get_from_dict(cls, player: dict, players: list[Player]):
        pl = get_player_from_player_id(UUID(int=player["player_id"]), players)
        return cls(
            pl,
            UUID(int=player["team_id"]),
            player["shirt_number"],
            PlayerContract.get_from_dict(player["contract"]),
        )

    def serialize(self) -> dict:
        return {
            "player_id": self.details.player_id.int,
            "team_id": self.team_id.int,
            "shirt_number": self.shirt_number,
            "contract": self.contract.serialize(),
        }


class GetPlayerException(Exception):
    pass


def get_player_from_player_id(player_id: UUID, players: list[Player]) -> Player:
    for player in players:
        if player.player_id == player_id:
            return player

    raise GetPlayerException


class PlayerInjuries(str, Enum):
    ANKLE_SP = "Ankle sprain"
    KNEE_SP = "Knee sprain"
    CALF_ST = "Calf strain"
    KNEECAP_BURSITIS = "Kneecap bursitis"
    RIB_BROK = "Broken rib"
    CLAVICLE_FRAC = "Fractured clavicle"
    ARM_FRAC = "Fractured arm"
    FOOT_FRAC = "Fractured foot"
    WRIST_FRAC = "Fractured wrist"
    ANKLE_FRAC = "Fractured ankle"
    CONCUSSION = "Concussion"
    LIGAMENT_TORN = "Torn ligament"
    MENISCAL_TORN = "Torn meniscal"


class PlayerSimulation:
    def __init__(
        self,
        player: PlayerTeam,
        current_position: Positions,
        stamina: float,
    ):
        self.player = player
        self.current_position = current_position
        self.current_skill = self.calculate_current_skill()
        self.current_stamina = stamina
        self.statistics = PlayerStats(player.details.player_id)
        self.is_injured = False
        self.injury_type = None
        self.subbed = False

    def calculate_current_skill(self) -> int:
        match self.current_position:
            case Positions.FW:
                return self.player.details.attributes.offense
            case Positions.MF:
                return self.player.details.attributes.passing
            case Positions.DF:
                return self.player.details.attributes.defense
            case Positions.GK:
                return self.player.details.attributes.gk

    def update_stamina(self):
        pass
