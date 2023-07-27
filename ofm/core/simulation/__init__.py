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
from enum import Enum, auto


class PitchPosition(Enum):
    """
    Positions on the pitch. According to what position the ball is on, the game may calculate a different outcome
    for each event.
    """

    DEF_BOX = 0
    DEF_LEFT = auto()
    DEF_RIGHT = auto()
    DEF_MIDFIELD_CENTER = auto()
    DEF_MIDFIELD_LEFT = auto()
    DEF_MIDFIELD_RIGHT = auto()
    MIDFIELD_LEFT = auto()
    MIDFIELD_CENTER = auto()
    MIDFIELD_RIGHT = auto()
    OFF_MIDFIELD_CENTER = auto()
    OFF_MIDFIELD_LEFT = auto()
    OFF_MIDFIELD_RIGHT = auto()
    OFF_LEFT = auto()
    OFF_RIGHT = auto()
    OFF_BOX = auto()


# Equivalent positions when changing possession
PITCH_EQUIVALENTS = dict(zip(PitchPosition, reversed(list(PitchPosition))))

OFF_POSITIONS = [
    PitchPosition.OFF_MIDFIELD_CENTER,
    PitchPosition.OFF_MIDFIELD_LEFT,
    PitchPosition.OFF_MIDFIELD_RIGHT,
    PitchPosition.OFF_LEFT,
    PitchPosition.OFF_RIGHT,
    PitchPosition.OFF_BOX,
]

DEF_POSITIONS = [
    PitchPosition.DEF_BOX,
    PitchPosition.DEF_LEFT,
    PitchPosition.DEF_RIGHT,
    PitchPosition.DEF_MIDFIELD_CENTER,
    PitchPosition.DEF_MIDFIELD_LEFT,
    PitchPosition.DEF_MIDFIELD_RIGHT,
]
