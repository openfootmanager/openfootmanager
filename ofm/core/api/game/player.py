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
from .positions import Positions

class Player:
    def __init__(
        self,
        fullname: str,
        short_name: str,
        nationality: str,
        age: int, 
        skill: dict,
        pos_skill: dict,
        player_id: uuid.UUID = None,
        stamina: int = 100,
    ):
        self.player_id = uuid.uuid4() if player_id is None else player_id
        self.fullname = fullname
        self.short_name = short_name
        self.nationality = nationality
        self.age = age
        self.skill = skill
        self.pos_skill = pos_skill
        self.stamina = stamina
        self.curr_pos = None

    def set_curr_pos(self, pos: Positions):
        self.curr_pos = pos

    def get_curr_pos(self):
        return self.curr_pos
    
    def get_best_pos(self):
        return max(self.pos_skill, key=self.pos_skill.get)

    def get_int_id(self):
        return self.player_id.int
