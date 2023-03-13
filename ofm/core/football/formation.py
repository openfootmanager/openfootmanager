#      Openfoot Manager - A free and open source soccer management simulation
#      Copyright (C) 2020-2023  Pedrenrique G. Guimarães
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

from .player import PlayerSimulation


@dataclass
class Formation:
    players: list[PlayerSimulation]

    def validate_formation(self) -> bool:
        """
        Validates if the given formation is valid.

        This method calculates the number of players, and returns if the formation is a valid formation.
        This should point out if players are missing from the formation.
        :return:
        """
        pass

    def get_formation(self) -> str:
        """
        This returns the formation string, such as 4-4-2, 3-5-2, 4-3-3, etc.
        :return:
        """
        if self.validate_formation():
            return ""

    def change_formation(self):
        """
        Changes the player formation.
        :return:
        """
        pass

    def update_formation(self):
        pass
