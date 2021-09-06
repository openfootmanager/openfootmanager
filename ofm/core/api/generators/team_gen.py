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

from .generator_interface import IGenerator
from ..game.team import Team


class TeamGenerator(IGenerator):
    def __init__(self):
        self.team_id = None
        self.name = None
        self.nationality = None
    
    def generate_id(self):
        pass

    def generate_name(self):
        pass

    def generate(self):
        pass

    def generate_obj(self):
        pass

    def generate_dict(self):
        pass

    def generate_file(self):
        pass
