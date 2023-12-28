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
from datetime import timedelta
from enum import Enum, auto

from . import PitchPosition


class SimulationStatus(Enum):
    NOT_STARTED = auto()
    FIRST_HALF = auto()
    FIRST_HALF_BREAK = auto()
    SECOND_HALF = auto()
    SECOND_HALF_BREAK = auto()
    FIRST_HALF_EXTRA_TIME = auto()
    FIRST_HALF_EXTRA_TIME_BREAK = auto()
    SECOND_HALF_EXTRA_TIME = auto()
    SECOND_HALF_EXTRA_TIME_BREAK = auto()
    PENALTY_SHOOTOUT = auto()
    FINISHED = auto()


@dataclass
class GameState:
    minutes: timedelta
    status: SimulationStatus
    position: PitchPosition
    in_additional_time: bool = False
    additional_time_elapsed: timedelta = timedelta(0)
