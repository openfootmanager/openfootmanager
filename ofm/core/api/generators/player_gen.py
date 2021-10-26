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
import random
import uuid
from datetime import date, timedelta

from ofm.core.api.file_management import write_to_file, load_list_from_file
from ofm.core.api.game.player import Player
from ofm.core.api.game.positions import Positions
from generator_interface import IGenerator


class PlayerGeneratorError(Exception):
    pass


class PlayerGenerator(IGenerator):
    def __init__(
        self,
        today: date = date.today(),
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
        self.dob = None
        self.pos_skill = None
        self.mult = None
        self.names = None
        self.today = today
        if min_age <= max_age:
            self.max_age = max_age
            self.min_age = min_age
        else:
            raise PlayerGeneratorError(
                "Minimum age cannot be higher than maximum age!"
            )
        self.file_name = file_name
        self.player_obj = None
        self.player_dict = None
        self.player_obj_list = []
        self.player_dict_list = []
        self.filename = file_name

    def generate_id(self) -> None:
        self.player_id = uuid.uuid4()
        
    def generate_name(self):
        """
        Generates the player's name
        :return:
        """
        pass

    def get_names(self) -> None:
        self.names = load_list_from_file("names.json")

    def generate_short_name(self):
        """
        Generates the player's shortname (the name on the player's shirt, or a nickname)
        :return:
        """
        pass

    def generate_dob(self) -> None:
        """
        Generates a random date of birth considering the minimum required age and the maximum required age
        :return:
        """
        year = timedelta(seconds=31556952)  # defining a Gregorian calendar year

        max_age = (
            self.max_age * year
        )

        min_age = (
            self.min_age * year
        )

        min_year = self.today - max_age  # min date for birthday
        max_year = self.today - min_age  # max date for birthday

        days_interval = max_year - min_year
        random_date = random.randrange(
            days_interval.days
        )
        self.dob = min_year + timedelta(days=random_date)  # assigns a random date of birth

    def generate_skill(self):
        """
        Generates the player's skill level
        :return:
        """
        pass

    def generate_mult(self):
        """
        Generates multipliers, so that players playing outside their ideal positions don't have ideal
        skill levels
        :return:
        """
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
    
    def generate_list(self, amount=11) -> list:
        return [self.generate() for _ in range(amount)]

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

    def generate_dict(self):
        """
        Generates the player dictionary. This is used to save to a file.
        :return:
        """
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
