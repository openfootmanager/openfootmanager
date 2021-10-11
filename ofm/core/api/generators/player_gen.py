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
from datetime import date, timedelta

from ofm.core.api.file_management import write_to_file, load_list_from_file
from ofm.core.api.game.player import Player
from generator_interface import IGenerator


class PlayerGeneratorError(Exception):
    pass


class PlayerGenerator(IGenerator):
    def __init__(
        self,
        today: data = date.today(),
        min_age: int = 16,
        max_age: int = 40,
        file_name: str = "players.json",
    ):
        self.player_id = None
        self.name = None
        self.nationality = None
        
        self.names = None

        self.skill = None
        self.short_name = None
        self.mult = None
        self.nationality = None
        
        if min_age <= max_age:
            self.min_age = min_age
            self.max_age = max_age
        else:
            self.min_age = 16
            self.max_age = 40

        self.dob = None
        self.pos_skill = None
        self.mult = None
        self.names = None
        self.player_obj = None
        self.player_dict = None
        self.player_obj_list = []
        self.player_dict_list = []
        self.filename = file_name
    
    def generate_id(self) -> None:
        self.player_id = uuid.uuid4()
        
    def get_names(self) -> None:
        self.names = load_list_from_file("names.json")
    
    def generate_name(self) -> None:
        pass

    def get_nationalitites(self) -> None:
        pass

    def generate_nationality(self) -> None:
        pass

    def generate_short_name(self) -> None:
        pass

    def generate_dob(self) -> None:
        pass
    
    def generate_skill(self) -> None:
        pass

    def generate_mult(self) -> None:
        pass

    def generate(self) -> None:
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
    
    def generate_amount(self, amount = 11) -> None:
        for _ in range(amount):
            self.generate()

    def generate_obj(self) -> None:
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

    def generate_dict(self) -> None:
        self.player_dict = {
            "player_id": self.player_id.int,
            "name": self.name,
            "nationality": self.nationality,
            "short_name": self.short_name,
            "pos_skill": self.pos_skill,
            "mult": self.mult,
        }

    def generate_file(self) -> None:
        write_to_file(self.player_dict_list)
