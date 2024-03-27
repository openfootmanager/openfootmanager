#      Openfoot Manager - A free and open source soccer management simulation
#      Copyright (C) 2020-2024  Pedrenrique G. Guimarães
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
from dataclasses import dataclass
from enum import IntEnum, auto
from typing import Optional, Union
from uuid import UUID

from .injury import PlayerInjury
from .player_attributes import PlayerAttributes
from .playercontract import PlayerContract
from .positions import Positions


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
    attributes: PlayerAttributes
        Set of skills that a player possesses, such as attacking, defending, midfield and goalkeeper skills. The higher
        the values, the better the player performs in such areas. Skill values range from 0 to 99.
    potential_skill: int
        Indicates potential overall that a player can reach. These are only metrics that are used
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
    potential_skill: int
    international_reputation: int
    preferred_foot: PreferredFoot
    value: float
    injury_type: PlayerInjury = PlayerInjury.NO_INJURY

    @property
    def is_injured(self) -> bool:
        return self.injury_type != PlayerInjury.NO_INJURY

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
            player_dict["potential_skill"],
            player_dict["international_reputation"],
            PreferredFoot(player_dict["preferred_foot"]),
            player_dict["value"],
            PlayerInjury(player_dict["injury_type"]),
        )

    def get_position_values(self):
        return [position.value for position in self.positions]

    def get_best_position(self) -> Positions:
        return self.positions[0]

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
            "potential_skill": self.potential_skill,
            "international_reputation": self.international_reputation,
            "preferred_foot": self.preferred_foot.value,
            "value": self.value,
            "injured": self.is_injured,
            "injury_type": self.injury_type.value,
        }


@dataclass
class PlayerStats:
    player_id: UUID
    minutes_played: float = 0.0
    passes: int = 0
    passes_missed: int = 0
    crosses: int = 0
    crosses_missed: int = 0
    dribbles: int = 0
    dribbles_failed: int = 0
    shots: int = 0
    shots_on_target: int = 0
    shots_missed: int = 0
    shots_blocked: int = 0
    interceptions: int = 0
    assists: int = 0
    fouls: int = 0
    goals: int = 0
    goals_conceded: int = 0  # only for GK
    shots_saved: int = 0
    own_goals: int = 0
    penalties: int = 0
    injuries: int = 0
    yellow_cards: int = 0
    red_cards: int = 0
    rating: float = 0.0


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
        team_id = self.team_id.int if self.team_id else None
        return {
            "player_id": self.details.player_id.int,
            "team_id": team_id,
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


class PlayerSimulation:
    def __init__(
        self,
        player: PlayerTeam,
        current_position: Positions,
    ):
        self.player = player
        self.current_position = current_position
        self._current_skill = 0.0
        self.statistics = PlayerStats(player.details.player_id)
        self.initial_stamina = player.details.stamina
        self.received_ball: Optional[PlayerSimulation] = None
        self.subbed = False
        self.able_to_play = True
        self.minutes_played = datetime.timedelta(seconds=0)

    @property
    def stamina(self) -> float:
        return self.player.details.stamina

    @stamina.setter
    def stamina(self, value: float):
        self.player.details.stamina = value

    @property
    def is_injured(self) -> bool:
        return self.player.details.is_injured

    @property
    def injury_type(self) -> PlayerInjury:
        return self.player.details.injury_type

    @injury_type.setter
    def injury_type(self, value: PlayerInjury):
        self.player.details.injury_type = value

    @property
    def sent_off(self) -> bool:
        return self.statistics.red_cards == 1

    @property
    def current_skill(self) -> int:
        self._current_skill = self.player.details.attributes.get_overall(
            self.current_position
        )
        return self._current_skill

    @property
    def attributes(self):
        return self.player.details.attributes

    @attributes.setter
    def attributes(self, attributes: PlayerAttributes):
        self.player.details.attributes = attributes

    def update_stamina(self, elapsed_time: float):
        fitness = self.player.details.fitness
        form = self.player.details.form

        self.minutes_played += datetime.timedelta(seconds=elapsed_time)

        # Using a half-life formula for stamina
        self.stamina = round(
            self.initial_stamina
            * (2 ** (-self.minutes_played.total_seconds() / (144 * fitness * form))),
            2,
        )

    def __eq__(self, other):
        if not isinstance(other, PlayerSimulation):
            return False
        return self.player.details.player_id == other.player.details.player_id

    def __str__(self):
        return self.player.details.short_name.encode("utf-8").decode("unicode_escape")

    def __repr__(self):
        return self.player.details.short_name.encode("utf-8").decode("unicode_escape")
