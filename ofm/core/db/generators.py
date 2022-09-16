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
from datetime import datetime, date, timedelta
import json
import random
import uuid
from abc import ABC, abstractmethod
from typing import Tuple, List, Optional, Union
from ofm.core.common.player import Player, Positions, PreferredFoot
from ofm.defaults import NAMES_FILE


class Generator(ABC):
    @abstractmethod
    def generate(self):
        pass

    @abstractmethod
    def write_to_db(self):
        pass


class GeneratePlayerError(Exception):
    pass


class GeneratePlayer(Generator):
    def __init__(self, today: Union[datetime, date] = date.today(), max_age = 35, min_age = 16):
        if min_age > max_age:
            raise GeneratePlayerError("Minimum age must not be greater than maximum age!")
        
        self.players_obj = []
        self.players_dict = []
        self.nationalities = self._get_nationalities()
        self.names = self._get_names()
        
        year = timedelta(seconds=31556952)  # definition of a Gregorian calendar date
        self.today = today
        self.max_age = max_age * year
        self.min_age = min_age * year

    def _get_nationalities(self):
        with open(NAMES_FILE, "r") as fp:
            data = json.load(fp)
            return [d["region"] for d in data]

    def _get_names(self):
        with open(NAMES_FILE, "r") as fp:
            return json.load(fp)

    def _get_names_from_region(self, region: str) -> dict:  
        for reg in self.names:
            if reg["region"] == region:
                return reg
    
    def generate_id(self):
        return uuid.uuid4()
    
    def generate_nationality(self, nat: Optional[str]) -> str:
        return nat or random.choice(self.nationalities)

    def generate_dob(self) -> datetime:
        """
        Generates the player's date of birth
        """
        min_year = self.today - self.max_age  # minimum date for birthday
        max_year = self.today - self.min_age  # max date for birthday

        days_interval = max_year - min_year
        rand_date = random.randrange(
            days_interval.days
        )  # chooses a random date from the max days interval
        return min_year + timedelta(days=rand_date)  # assigns date of birth
    
    def generate_name(self, region: Optional[str]) -> Tuple[str, str]:
        if not region:
            region = random.choice(self.nationalities)
        names = self.get_names_from_region(region)
        first_name = random.choice(names["male"])
        last_name = random.choice(names["surnames"])
        return first_name, last_name

    def generate_skill(self) -> int:
        """
        Generates the player's skill lvl. Region-tuned skill-lvl might come later,
        but for now, just generates players with skill lvls from 30 to 90.

        I'm capping skill lvls to not return negative values or values above 90.

        The planned skill rating should go from 0 to 99 in this game, just like other soccer games do. 
        """
        mu = 50
        sigma = 20

        skill = int(random.gauss(mu, sigma))

        skill = min(skill, 90)
        skill = max(30, skill)

        return skill
    
    def generate_positions(self, desired_pos: Optional[List[Positions]]) -> Positions:
        if desired_pos:     # might be useful if we want to generate teams later, so we don't get entirely random positions
            return desired_pos
        positions = list(Positions)
        return random.choices(positions)  # very naive implementation, I will improve it later

    def generate_preferred_foot(self) -> PreferredFoot:
        return random.choice(list(PreferredFoot))

    def get_player_value(self, skill: int, age: int) -> float:
        """
        Gets how much the player is worth based on the player's rating.

        Standard currency is EUR, might be convertable later.
        """
        distance_to_max_age = self.max_age - timedelta(days=age.days)

    
    def generate(self, region: Optional[str]) -> Player:
        pass

    def write_to_db(self):
        pass


class GenerateTeams(Generator):
    def __init__(self):
        pass
