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
from dataclasses import dataclass
from .player import Player


@dataclass
class Formation:
    goalkeeper: Player
    defenders: list[Player]
    midfielders: list[Player]
    forwards: list[Player]

    def change_formation(self):
        pass

    def validate_formation(self):
        number_players = len(self.defenders) + len(self.midfielders) + len(self.forwards)
        if number_players == 10:
            return True

        return False

