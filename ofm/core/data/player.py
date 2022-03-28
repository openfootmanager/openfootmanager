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
import inspect
from uuid import UUID
from dataclasses import dataclass
from datetime import datetime, date
from typing import Union


@dataclass
class PlayerStats:
    """
    The PlayerStats class records the player's all-time statistics.

    It counts all the relevant information from the player's stats.
    """
    num_games: int = 0
    goals: int = 0
    fouls_suffered: int = 0
    own_goals: int = 0
    shots: int = 0
    defenses: int = 0
    injuries: int = 0
    yellow: int = 0
    red: int = 0
    total_rating: float = 0.0
    minutes_played: float = 0.0

    @classmethod
    def get_from_dict(cls, data: dict):
        """
        Creates a PlayerStats from a dictionary.
        :param data: dictionary data to turn into a PlayerStats
        :return: returns a PlayerStats with the given attributes from the dictionary
        """
        return cls(
            **{
                key: (data[key] if val.default == val.empty else data.get(key, val.default))
                for key, val in inspect.signature(PlayerStats).parameters.items()
            }
        )

    @property
    def avg_rating(self) -> float:
        return self.total_rating / self.num_games if self.num_games > 0 else 0.0


@dataclass
class Player:
    player_id: Union[int, UUID]
    name: str
    short_name: str
    club_number: int
    date_of_birth: Union[datetime, date, str]
    nationality: str
    international_reputation: int
    overall: int
    positions: str
    __potential: int
    preferred_foot: str
    value: float
    wage: float

    def get_age(self, today: datetime = date.today()) -> int:
        """
        Calculates the player's age based on the date of birth and the given date of "today".
        :param today: represents the current date that to calculate the player's age.
        :return: returns the players age in years
        """
        if isinstance(self.date_of_birth, str):
            date_of_birth = datetime.strptime(self.date_of_birth, "%Y-%m-%d")
        elif isinstance(self.date_of_birth, (datetime, date)):
            date_of_birth = date
        else:
            return NotImplemented

        return today.year - date_of_birth.year - ((today.month, today.day) < (date_of_birth.month, date_of_birth.day))

    @classmethod
    def get_from_dict(cls, data: dict):
        """
        Creates a Player from a dictionary.
        :param data: dictionary data to turn into a Player
        :return: returns a Player with the given attributes from the dictionary
        """
        return cls(
            **{
                key: (data[key] if val.default == val.empty else data.get(key, val.default))
                for key, val in inspect.signature(Player).parameters.items()
            }
        )

    def get_potential(self, potential_multiplier: float, potential_rand: float) -> float:
        """
        A scout should have a potential multiplier, that is unable to tell the full potential of the player.
        Sometimes it can overestimate or underestimate the player's potential.
        :param potential_multiplier: the potential multiplier from the scout. If the scout is good, the multiplier will
        be closer to 1.
        :param potential_rand: the potential randomness from the scout. If the scout is good, the rand approaches 0.
        :return: returns the potential
        """
        return self.__potential * potential_multiplier + potential_rand


@dataclass
class PlayerLive:
    """
    Class for player that plays in a LiveGame.

    Contains the information related to the current live game.
    """
    player: Player
    player_stats: PlayerStats
    goals: int = 0
    own_goals: int = 0
    fouls_suffered: int = 0
    shots: int = 0
    defenses: int = 0
    injury: int = 0
    red: int = 0
    yellow: int = 0
    stamina: float = 0.0
    rating: float = 0.0

    def add_to_player_stats(self):
        self.player_stats.fouls_suffered += self.fouls_suffered
        self.player_stats.goals += self.goals
        self.player_stats.own_goals += self.own_goals
        self.player_stats.shots += self.shots
        self.player_stats.defenses += self.defenses
        self.player_stats.injuries += self.injury
        self.player_stats.yellow += self.yellow
        self.player_stats.red += self.red
        self.player_stats.total_rating += self.rating
        self.player_stats.num_games += 1
