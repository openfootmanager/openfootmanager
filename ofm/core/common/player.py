#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2022  Pedrenrique G. Guimarães
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
from dataclasses import dataclass
import datetime
from typing import Union
from uuid import UUID
from enum import IntEnum, auto
from .playercontract import PlayerContract


class Positions(IntEnum):
    GK = auto()
    LW = auto()
    DF = auto()
    RW = auto()
    MF = auto()
    ST = auto()
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
class Player:
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
    skill: int
    potential_skill: int
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
            player_dict["skill"],
            player_dict["potential_skill"],
            player_dict["international_reputation"],
            player_dict["preferred_foot"],
            player_dict["value"]
        )

    def get_position_values(self):
        return [position.value for position in self.positions]

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
            "skill": self.skill,
            "potential_skill": self.potential_skill,
            "international_reputation": self.international_reputation,
            "preferred_foot": self.preferred_foot.value,
            "value": self.value
        }


@dataclass
class PlayerStats:
    player_id: UUID
    shots: int = 0
    assists: int = 0
    fouls: int = 0
    goals: int = 0
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
            "contract": self.contract.serialize()
        }


class GetPlayerException(Exception):
    pass


def get_player_from_player_id(player_id: UUID, players: list[Player]) -> Player:
    for player in players:
        if player.player_id == player_id:
            return player

    raise GetPlayerException


class PlayerSimulation:
    def __init__(
            self,
            player: PlayerTeam,
            current_position: dict,
            stamina: float,
    ):
        self.player = player
        self.current_position = current_position
        self.current_skill = self.calculate_current_skill()
        self.current_stamina = stamina
        self.statistics = PlayerStats(player.details.player_id)

    def calculate_current_skill(self):
        return self.player.details.skill * self.current_position["mult"]

    def update_stamina(self):
        pass
