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
import logging
from generators.player_gen import PlayerGenerator
from generators.team_gen import TeamGenerator


class Generator:
    def __init__(self):
        self.generator = None
        self.logger = logging.getLogger(__file__)

    def generate_players(self):
        if self.generator is None:
            self.generator = PlayerGenerator()

    def generate_teams(self):
        if self.generator is None:
            self.generator = TeamGenerator()

    def write_data_file(self):
        pass

    def update_database(self, database):
        pass

