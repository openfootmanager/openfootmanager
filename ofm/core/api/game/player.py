#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2021  Pedrenrique G. Guimarães
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

import uuid
from typing import Union
from datetime import datetime
from .positions import Positions


class Player:
    def __init__(
        self,
        fullname: str,
        short_name: str,
        nationality: str,
        age: int,
        dob: Union[str, datetime],
        skill: int,
        pos: str,
        international_rep: int,
        preferred_foot: str,
        player_id: uuid.UUID = None,
        stamina: int = 100,
        value: float = 0,
        wage: float = 0,
    ):
        self.player_id = uuid.uuid4() if player_id is None else player_id
        self.fullname = fullname
        self.short_name = short_name
        self.nationality = nationality
        self.age = int(age)
        if not isinstance(dob, datetime):
            self.dob = datetime.strptime(dob, "%Y-%m-%d")
        else:
            self.dob = dob
        self.skill = int(skill)
        self.intenational_rep = international_rep
        self.preferred_foot = preferred_foot
        self.value = float(value)
        self.wage = float(wage)
        self.pos = pos
        self.stamina = stamina
        self.curr_pos = None

    def set_curr_pos(self, pos: Positions):
        self.curr_pos = pos

    def get_curr_pos(self):
        return self.curr_pos
    
    def get_best_pos(self):
        return self.pos[0]

    def get_int_id(self):
        return self.player_id.int

    def __str__(self):
        return self.short_name

    def __repr__(self):
        return self.short_name
