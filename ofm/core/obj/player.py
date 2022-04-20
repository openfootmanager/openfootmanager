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
from typing import Union
from uuid import UUID
from dataclasses import dataclass


@dataclass
class Player:
    player_id: UUID
    current_team_id: Union[UUID, None]
    first_name: str
    last_name: str
    short_name: str
    positions: Union[list, str]
    skill: int
    potential_skill: int


class PlayerSimulation:
    player: Player
    current_skill: int


    def calculate_current_skill(self, skill):
        pass
