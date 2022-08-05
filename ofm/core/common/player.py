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
from typing import Union
from uuid import UUID
from enum import StrEnum, auto

from .contract import Contract


class PreferredFoot(StrEnum):
    LEFT = auto()
    RIGHT = auto()
    BOTH = auto()


@dataclass
class Player:
    player_id: UUID
    current_team_id: Union[UUID, None]
    first_name: str
    last_name: str
    short_name: str
    positions: list[dict]
    fitness: float
    stamina: float
    form: float
    skill: int
    potential_skill: int
    shirt_number: int
    national_shirt_number: int
    international_reputation: int
    preferred_foot: PreferredFoot
    value: float
    contract: Contract


@dataclass
class PlayerStats:
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


class PlayerSimulation:
    def __init__(
            self,
            player: Player,
            current_position: dict,
            stamina: float,
    ):
        self.player = player
        self.current_position = current_position
        self.current_skill = self.calculate_current_skill()
        self.current_stamina = stamina
        self.statistics = PlayerStats()

    def calculate_current_skill(self):
        return self.player.skill * self.current_position["mult"]

    def update_stamina(self):
        pass
