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
from dataclasses import dataclass, field
from .player import Player


def get_formations():
    return [
        "3-5-2",
        "3-4-3",
        "3-4-1-2",
        "3-6-1",
        "4-4-2",
        "4-2-2-2",
        "4-4-1-1",
        "4-2-3-1",
        "4-3-1-2",
        "4-3-2-1",
        "4-4-1-1",
        "4-1-2-1-2",
        "4-3-3",
        "4-5-1",
        "5-4-1",
        "5-3-2",
    ]


@dataclass
class Formation:
    goalkeeper: Player
    defenders: list[Player] = field(default_factory=list)
    midfielders: list[Player] = field(default_factory=list)
    forwards: list[Player] = field(default_factory=list)

    def get_formation(self, formation: str):
        pass

    def set_formation(self, formation: str):
        pass

    def validate_formation(self):
        number_players = len(self.defenders) + len(self.midfielders) + len(self.forwards)
        if number_players == 10:
            return True

        return False
