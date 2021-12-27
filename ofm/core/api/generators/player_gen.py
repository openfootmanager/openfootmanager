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
import datetime
import random
import uuid
from datetime import date, timedelta
from typing import Union
from pathlib import Path

from ofm.core.api.file_management import write_to_file, load_list_from_file
from ofm.core.api.game.player import Player
from ofm.core.api.game.positions import Positions
from generator_interface import IGenerator


class PlayerGeneratorError(Exception):
    pass


class PlayerGenerator(IGenerator):
    """
    The PlayerGenerator class creates and populates a database of players based on some parameters.

    It is able to create random players for a team. In the PlayerParser class, the PlayerGenerator is used
    to create players based on existing information from a spreadsheet or json file.
    """

    def __init__(
            self,
            today: date = date.today(),
            min_age: int = 16,
            max_age: int = 40,
            file_name: str = "players.json",
    ):
        self.preferred_foot = None
        self.player_id = None
        self.name = None
        self.nationality = None

        self.names = None

        self.skill = None
        self.short_name = None
        self.mult = None
        self.nationality = None
        self.dob = None
        self.positions = None
        self.international_rep = None
        self.nationalities = []
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

    def generate_id(self) -> None:
        self.player_id = uuid.uuid4()

    def generate_name(self):
        """
        Generates the player's name
        :return:
        """
        if self.names is None:
            self.get_names()
        self.name = random.choice

    @staticmethod
    def get_names() -> list:
        return load_list_from_file("names.json")

    def get_nationalities(self):
        if self.names is None:
            nationalities = load_list_from_file("names.json")
        else:
            nationalities = self.names.copy()

        self.nationalities = []
        for nat in nationalities:
            nationality = nat["region"]
            self.nationalities.append(nationality)

    def generate_nationality(self):
        """
        Generates the player's nationality
        :return:
        """
        if self.nationalities is None:
            self.get_nationalities()
        self.nationality = random.choice(self.nationalities)

    def generate_short_name(self):
        """
        Generates the player's shortname (the name on the player's shirt, or a nickname)
        :return:
        """
        name = self.name.split()
        self.short_name = name[0][0] + '. ' + name[-1]

    def calculate_age(self, today: date = date.today()):
        """
        Calculates the age based on today's date.
        :param today:
        :return:
        """
        return today.year - self.dob.year - ((today.month, today.day) < (self.dob.month, self.dob.day))

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

    def generate_positions(self):
        """
        Generates multipliers, so that players playing outside their ideal positions don't have ideal
        skill levels
        :return:
        """
        pos = [pos.name for pos in Positions]
        self.positions = random.choices(pos)

    def generate_international_rep(self):
        """
        Generates the player's international reputation.

        Players with good international reputation have a higher weight on the game simulation.

        International reputation ranges from 0 to 5
        :return:
        """
        self.international_rep = max(int(random.gauss(5, 1)), 0)

    def generate_preferred_foot(self):
        """
        Generate the player's preferred foot: left or right.

        This can have an impact in the shot simulation. The player tends to shoot more accurately with its preferred foot.
        In the future we can attribute weights to player's preferred foot to make it a tuple, and more
        accurately describe the player's ability with each foot.
        :return:
        """
        feet = ['Left', 'Right']
        self.preferred_foot = random.choice(feet)

    def generate(self):
        """
        Generates a random player with random data
        :return:
        """
        self.generate_id()
        self.generate_nationality()
        self.generate_name()
        self.generate_short_name()
        self.generate_dob()
        self.generate_skill()
        self.generate_positions()
        self.generate_international_rep()
        self.generate_preferred_foot()
        self.generate_obj()
        self.generate_dict()
        self.player_obj_list.append(self.player_obj)
        self.player_dict_list.append(self.player_dict)

    def generate_list(self, amount=11) -> list:
        """
        Generates a list of players based on the amount of players given.
        :param amount:
        :return:
        """
        return [self.generate() for _ in range(amount)]

    def generate_obj(self) -> None:
        """
        Generates a player object. It is useful for match simulation.
        :return:
        """
        self.player_obj = Player(
            self.name,
            self.short_name,
            self.nationality,
            self.dob,
            self.skill,
            self.positions,
            self.international_rep,
            self.preferred_foot,
            self.player_id,
        )

    def generate_dict(self):
        """
        Generates the player dictionary. This is used to save to a file.
        :return:
        """
        self.player_dict = {
            "id": self.player_id.int,
            "name": self.name,
            "dob": self.dob,
            "age": self.calculate_age(),
            "overall": self.skill,
            "positions": self.positions,
            "international_reputation": self.international_rep,
            "preferred_foot": self.preferred_foot,
            "nationality": self.nationality,
            "short_name": self.short_name,
        }

    def __check_attributes(self):
        """
        Checks for None attributes, to generate them in case one of them is not found.
        :return:
        """
        if self.player_id is None:
            self.generate_id()
        if self.name is None:
            self.generate_name()
        if self.dob is None:
            self.generate_dob()
        if self.nationality is None:
            self.generate_nationality()
        if self.skill is None:
            self.generate_skill()
        if self.positions is None:
            self.generate_positions()
        if self.international_rep is None:
            self.generate_international_rep()
        if self.preferred_foot is None:
            self.generate_preferred_foot()
        if self.short_name is None:
            self.generate_short_name()

    def get_dict(self, dictionary: dict):
        """
        Gets a player_dict from the given dictionary.
        :param dictionary:
        :return:
        """
        self.player_id = dictionary.get("id")
        self.name = dictionary.get("name")
        self.dob = dictionary.get("dob")
        self.skill = dictionary.get("overall")
        self.positions = dictionary.get("positions")
        self.international_rep = dictionary.get("international_reputation")
        self.preferred_foot = dictionary.get("preferred_foot")
        self.nationality = dictionary.get("nationality")
        self.short_name = dictionary.get("short_name")

        # Checking if any value is None to fill them in based on the other values
        self.__check_attributes()

        # Get dictionary and append it to the list of dictionaries
        self.generate_dict()
        self.player_dict_list.append(self.player_dict)

    def generate_file(self) -> None:
        write_to_file(self.player_dict_list, self.file_name)


class PlayerParser:
    """
    Reads and parses a json file to create a players.json file using the PlayerGenerator.

    This is useful if a players.json file already exists, or if we need to parse a file with
    data coming from a spreadsheet, and turning this file into a format that OFM can understand.

    In the future, I think I'll try to make this also recognize XLSX, XML and other types of files to make
    it compatible with Bygfoot.
    """

    def __init__(
            self,
            filename: Union[str, Path],
            read_file: Union[str, Path],
            player_file: Union[str, Path] = "players.json"
    ):
        self.filename = filename
        self.read_file = read_file
        self.player_list = None
        self.destination_file = player_file
        self.player_generator = PlayerGenerator()

    def get_players(self):
        self.player_list = load_list_from_file(self.read_file)
        self.player_generator.player_dict_list.clear()
        for data in self.read_file:
            self.player_generator.get_dict(data)

    def write_players_file(self):
        self.player_generator.generate_file()
