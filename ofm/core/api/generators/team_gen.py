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

from ofm.core.api.file_management import load_list_from_file, write_to_file, get_list_from_file
from ofm.core.api.generators.generator_interface import IGenerator
from ofm.core.api.game.team import Team
from player_gen import PlayerGenerator


class TeamGenerator(IGenerator):
    def __init__(self):
        self.team_id = None
        self.name = None
        self.names = None
        self.nationality = None
        self.file_name = "teams.json"
        
        self.countries = None
        self.country = None

        self.team_obj = None
        self.team_dict = None

        self.player_gen = PlayerGenerator()
        self.roster = []
        self.team_dict_list = []
        self.team_obj_list = []
    
    def generate_id(self) -> None:
        self.team_id = uuid.uuid4()

    def get_team_names(self) -> None:
        self.names = get_list_from_file("team_names.txt")

    def generate_name(self) -> None:
        self.name = random.choice(self.names)
        self.names.remove(self.name)

    def get_countries(self) -> None:
        return load_list_from_file("countries.txt")

    def generate_country(self) -> None:
        if self.countries is None:
            self.countries = self.get_countries()
        self.country = random.choice(self.countries)
    
    def generate_players(self, amount) -> None:
        self.roster = [
            player.player_id
            for player in self.player_gen.generate_list(amount)
        ]

    def generate(self) -> None:
        self.generate_id()
        self.generate_name()
        self.generate_country()
        self.generate_players()
        self.generate_obj()
        self.generate_dict()
        self.team_obj_list.append(self.team_obj)
        self.team_dict_list.append(self.team_dict)

    def generate_list(self, amount) -> None:
        for _ in range(amount):
            self.generate()
    
    def generate_obj(self) -> None:
        self.team_obj = Team(
            self.name,
            self.team_id,
            self.roster
        )

    def generate_dict(self) -> None:
        self.team_dict = {
            "id": self.id.int,
            "name": self.name,
            "roster": self.roster.copy()
        }

    def generate_file(self) -> None:
        write_to_file(self.file_name)
