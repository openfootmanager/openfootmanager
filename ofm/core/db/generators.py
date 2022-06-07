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
from abc import ABC, abstractmethod


class Generator(ABC):
    @abstractmethod
    def generate(self):
        pass

    @abstractmethod
    def write_to_db(self):
        pass


class GeneratePlayer(Generator):
    def __init__(self):
        self.players_obj = []
        self.players_dict = []
        self.nationalities = self.get_nationalities()

    def get_nationalities(self):
        return []

    def generate_nationality(self):
        pass

    def generate_name(self):
        pass

    def generate(self):
        pass

    def write_to_db(self):
        pass
