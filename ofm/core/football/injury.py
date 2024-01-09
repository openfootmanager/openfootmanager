#      Openfoot Manager - A free and open source soccer management simulation
#      Copyright (C) 2020-2024  Pedrenrique G. Guimarães
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
from enum import Enum, auto


class PlayerInjury(Enum):
    NO_INJURY = auto()
    LIGHT_INJURY = auto()
    MEDIUM_INJURY = auto()
    SEVERE_INJURY = auto()
    CAREER_ENDING_INJURY = auto()


# TODO: Implement types of injury and an InjuryManager class to determine how long the player
#   will be out
