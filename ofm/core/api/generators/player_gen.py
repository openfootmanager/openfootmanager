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
from .generator_interface import IGenerator
from ...player import Player


class PlayerGenerator(IGenerator):
    def __init__(self):
        self.player_id = None
        self.name = None
        self.skill = None
        self.short_name = None
        self.mult = None
        self.nationality = None
        self.dob = None
        self.pos_skill = None
        self.mult = None
        self.names = None
        self.player_obj = None
        self.player_dict = None
        self.player_obj_list = []
        self.player_dict_list = []
    
    def generate_id(self):
        self.player_id = uuid.uuid4()
        
    def generate_name(self):
        pass

    def generate_short_name(self):
        pass

    def generate_dob(self):
        pass
    
    def generate_skill(self):
        pass

    def generate_mult(self):
        pass

    def generate(self):
        self.generate_id()
        self.generate_name()
        self.generate_short_name()
        self.generate_dob()
        self.generate_skill()
        self.generate_mult()
        self.generate_obj()
        self.generate_dict()
        self.player_obj_list.append(self.player_obj)
        self.player_dict_list.append(self.player_dict)
    
    def generate_amount(self, amount = 11):
        for _ in range(amount):
            self.generate()

    def generate_obj(self):
        self.player_obj = Player(
            self.name,
            self.short_name,
            self.nationality,
            self.dob,
            self.skill,
            self.pos_skill,
            self.mult,
            self.player_id,
        )

    def generate_dict(self):
        self.player_dict = {
            "player_id": self.player_id.int,
            "name": self.name,
            "nationality": self.nationality,
            "short_name": self.short_name,
            "pos_skill": self.pos_skill,
            "mult": self.mult,
        }

    def generate_file(self):
        pass
