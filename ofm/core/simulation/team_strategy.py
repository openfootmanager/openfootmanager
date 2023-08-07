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

from . import OFF_POSITIONS, PitchPosition
from .event_type import EventType
from .game_state import GameState


class TeamStrategy(Enum):
    NORMAL = 0
    KEEP_POSSESSION = auto()
    COUNTER_ATTACK = auto()
    DEFEND = auto()
    ALL_ATTACK = auto()


def team_pass_strategy(strategy: TeamStrategy) -> list[list[int]]:
    match strategy:
        case TeamStrategy.NORMAL:
            return [
                [
                    0.1,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                ],  # DEF_BOX
                [
                    0.2,
                    0.1,
                    0.3,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_LEFT
                [
                    0.2,
                    0.3,
                    0.1,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_RIGHT
                [
                    0.15,
                    0.1,
                    0.1,
                    0.1,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_CENTER
                [
                    0.15,
                    0.1,
                    0.1,
                    0.2,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_LEFT
                [
                    0.15,
                    0.1,
                    0.1,
                    0.2,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_CENTER
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_CENTER
                [
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                ],  # OFF_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                ],  # OFF_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                ],  # OFF_BOX
            ]
        case TeamStrategy.KEEP_POSSESSION:
            return [
                [
                    0.3,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_BOX
                [
                    0.2,
                    0.3,
                    0.1,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_LEFT
                [
                    0.2,
                    0.1,
                    0.3,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_RIGHT
                [
                    0.15,
                    0.1,
                    0.1,
                    0.25,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_CENTER
                [
                    0.15,
                    0.1,
                    0.1,
                    0.1,
                    0.25,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_LEFT
                [
                    0.15,
                    0.1,
                    0.1,
                    0.1,
                    0.1,
                    0.25,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.2,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.2,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_CENTER
                [
                    0.05,
                    0.2,
                    0.05,
                    0.2,
                    0.05,
                    0.05,
                    0.2,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.4,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_CENTER
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.2,
                    0.4,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.2,
                    0.2,
                    0.4,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.2,
                    0.2,
                    0.2,
                    0.2,
                    0.2,
                    0.4,
                ],  # OFF_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.2,
                    0.2,
                    0.2,
                    0.2,
                    0.2,
                    0.4,
                ],  # OFF_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.2,
                    0.2,
                    0.2,
                    0.2,
                    0.2,
                    0.4,
                ],  # OFF_BOX
            ]
        case TeamStrategy.DEFEND:
            return [
                [
                    0.4,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_BOX
                [
                    0.2,
                    0.3,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_LEFT
                [
                    0.2,
                    0.2,
                    0.3,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_RIGHT
                [
                    0.2,
                    0.1,
                    0.2,
                    0.3,
                    0.2,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_CENTER
                [
                    0.15,
                    0.1,
                    0.15,
                    0.2,
                    0.35,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_LEFT
                [
                    0.15,
                    0.1,
                    0.15,
                    0.2,
                    0.15,
                    0.35,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.3,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.3,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_CENTER
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.3,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.3,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_CENTER
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.3,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.3,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.3,
                    0.05,
                    0.05,
                ],  # OFF_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.3,
                    0.05,
                ],  # OFF_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.3,
                ],  # OFF_BOX
            ]
        case TeamStrategy.COUNTER_ATTACK:
            return [
                [
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_BOX
                [
                    0.2,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_LEFT
                [
                    0.2,
                    0.2,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_RIGHT
                [
                    0.2,
                    0.1,
                    0.2,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_CENTER
                [
                    0.15,
                    0.1,
                    0.15,
                    0.2,
                    0.35,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_LEFT
                [
                    0.15,
                    0.1,
                    0.15,
                    0.2,
                    0.15,
                    0.35,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_CENTER
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.15,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                ],  # MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.15,
                    0.05,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_CENTER
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                ],  # OFF_MIDFIELD_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                ],  # OFF_MIDFIELD_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.3,
                    0.15,
                    0.1,
                ],  # OFF_LEFT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.3,
                    0.1,
                ],  # OFF_RIGHT
                [
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.15,
                    0.3,
                ],  # OFF_BOX
            ]
        case TeamStrategy.ALL_ATTACK:
            return [
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                    0.05,
                ],  # DEF_BOX
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.2,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                    0.05,
                ],  # DEF_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.2,
                    0.2,
                    0.3,
                    0.15,
                    0.1,
                    0.05,
                ],  # DEF_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.2,
                    0.1,
                    0.2,
                    0.3,
                    0.15,
                    0.1,
                ],  # DEF_MIDFIELD_CENTER
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.15,
                    0.1,
                    0.15,
                    0.35,
                    0.15,
                    0.1,
                ],  # DEF_MIDFIELD_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.15,
                    0.1,
                    0.15,
                    0.15,
                    0.35,
                    0.15,
                ],  # DEF_MIDFIELD_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.3,
                ],  # MIDFIELD_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.3,
                ],  # MIDFIELD_CENTER
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.3,
                ],  # MIDFIELD_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.3,
                ],  # OFF_MIDFIELD_CENTER
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.3,
                ],  # OFF_MIDFIELD_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.15,
                    0.15,
                    0.3,
                ],  # OFF_MIDFIELD_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.10,
                    0.10,
                    0.4,
                ],  # OFF_LEFT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.10,
                    0.10,
                    0.4,
                ],  # OFF_RIGHT
                [
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.05,
                    0.1,
                    0.1,
                    0.1,
                    0.10,
                    0.10,
                    0.4,
                ],  # OFF_BOX
            ]


def team_cross_strategy(strategy: TeamStrategy) -> list[list[int]]:
    match strategy:
        case TeamStrategy.NORMAL:
            # TODO: implement transition matrix for NORMAL TeamStrategy
            pass
        case TeamStrategy.KEEP_POSSESSION:
            # TODO: implement transition matrix for KEEP_POSSESSION TeamStrategy
            pass
        case TeamStrategy.DEFEND:
            # TODO: implement transition matrix for DEFEND TeamStrategy
            pass
        case TeamStrategy.COUNTER_ATTACk:
            # TODO: implement transition matrix for COUNTER_ATTACK TeamStrategy
            pass
        case TeamStrategy.ALL_ATTACK:
            # TODO: implement transition matrix for ALL_ATTACK TeamStrategy
            pass


def team_general_strategy(strategy: TeamStrategy, state: GameState) -> list[list[int]]:
    match strategy:
        case TeamStrategy.NORMAL:
            transition_matrix = [
                [4, 2, 0, 1, 1, 0, 0, 0, 0],  # PASS
                [2, 2, 0, 1, 1, 0, 0, 0, 0],  # DRIBBLE
                [1, 1, 0, 1, 1, 1, 1, 0, 0],  # SHOT
                [1, 1, 0, 0, 1, 1, 0, 1, 0],  # CROSS
                [0, 0, 0, 0, 0, 1, 0, 0, 0],  # FOUL
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # FREE KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # CORNER KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # GOAL KICK
                [0, 0, 1, 0, 0, 0, 0, 0, 0],  # PENALTY KICK
            ]
            # Depending on the position, some events will be added to the matrix
            if state.position in OFF_POSITIONS:
                transition_matrix[EventType.PASS.value][EventType.SHOT.value] = 1
                transition_matrix[EventType.CROSS.value][EventType.SHOT.value] = 1
                transition_matrix[EventType.FREE_KICK.value][EventType.SHOT.value] = 1

                # If a change of possession occurs after a shot, a defensive team could still try to shoot
                # In a defensive pitch position. Let's make sure it only tries it in the offensive positions
                transition_matrix[EventType.SHOT.value][EventType.SHOT.value] = 1
        case TeamStrategy.KEEP_POSSESSION:
            transition_matrix = [
                [5, 1, 0, 1, 0, 0, 0, 0, 0],  # PASS
                [1, 2, 0, 1, 1, 0, 0, 0, 0],  # DRIBBLE
                [1, 1, 0, 1, 1, 1, 1, 0, 0],  # SHOT
                [1, 1, 0, 0, 1, 1, 0, 1, 0],  # CROSS
                [0, 0, 0, 0, 1, 0, 0, 0, 0],  # FOUL
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # FREE KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # CORNER KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # GOAL KICK
                [0, 0, 1, 0, 0, 0, 0, 0, 0],  # PENALTY KICK
            ]

            # Depending on the position, some events will be added to the matrix
            if state.position in OFF_POSITIONS:
                transition_matrix[EventType.PASS.value][EventType.SHOT.value] = 2
                transition_matrix[EventType.CROSS.value][EventType.SHOT.value] = 2
                transition_matrix[EventType.FREE_KICK.value][EventType.SHOT.value] = 2

                # If a change of possession occurs after a shot, a defensive team could still try to shoot
                # In a defensive pitch position. Let's make sure it only tries it in the offensive positions
                transition_matrix[EventType.SHOT.value][EventType.SHOT.value] = 2
        case TeamStrategy.DEFEND:
            transition_matrix = [
                [3, 2, 0, 1, 0, 0, 0, 0, 0],  # PASS
                [2, 2, 0, 1, 1, 0, 0, 0, 0],  # DRIBBLE
                [1, 1, 0, 1, 1, 1, 1, 0, 0],  # SHOT
                [1, 1, 0, 0, 1, 1, 0, 1, 0],  # CROSS
                [0, 0, 0, 0, 1, 0, 0, 0, 0],  # FOUL
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # FREE KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # CORNER KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # GOAL KICK
                [0, 0, 1, 0, 0, 0, 0, 0, 0],  # PENALTY KICK
            ]

            # Depending on the position, some events will be added to the matrix
            if state.position in OFF_POSITIONS:
                transition_matrix[EventType.PASS.value][EventType.SHOT.value] = 1
                transition_matrix[EventType.CROSS.value][EventType.SHOT.value] = 1
                transition_matrix[EventType.FREE_KICK.value][EventType.SHOT.value] = 1

                # If a change of possession occurs after a shot, a defensive team could still try to shoot
                # In a defensive pitch position. Let's make sure it only tries it in the offensive positions
                transition_matrix[EventType.SHOT.value][EventType.SHOT.value] = 1

        case TeamStrategy.COUNTER_ATTACk:
            transition_matrix = [
                [1, 2, 0, 1, 0, 0, 0, 0, 0],  # PASS
                [2, 2, 0, 1, 1, 0, 0, 0, 0],  # DRIBBLE
                [1, 1, 0, 1, 1, 1, 1, 0, 0],  # SHOT
                [1, 1, 0, 0, 1, 1, 0, 1, 0],  # CROSS
                [0, 0, 0, 0, 1, 0, 0, 0, 0],  # FOUL
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # FREE KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # CORNER KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # GOAL KICK
                [0, 0, 1, 0, 0, 0, 0, 0, 0],  # PENALTY KICK
            ]

            # Depending on the position, some events will be added to the matrix
            if state.position in OFF_POSITIONS:
                transition_matrix[EventType.PASS.value][EventType.SHOT.value] = 2
                transition_matrix[EventType.CROSS.value][EventType.SHOT.value] = 2
                transition_matrix[EventType.FREE_KICK.value][EventType.SHOT.value] = 2

                # If a change of possession occurs after a shot, a defensive team could still try to shoot
                # In a defensive pitch position. Let's make sure it only tries it in the offensive positions
                transition_matrix[EventType.SHOT.value][EventType.SHOT.value] = 2
        case TeamStrategy.ALL_ATTACK:
            transition_matrix = [
                [3, 2, 0, 2, 0, 0, 0, 0, 0],  # PASS
                [2, 3, 0, 2, 2, 0, 0, 0, 0],  # DRIBBLE
                [2, 2, 0, 2, 2, 2, 2, 0, 0],  # SHOT
                [1, 1, 0, 0, 1, 1, 0, 1, 0],  # CROSS
                [0, 0, 0, 0, 1, 0, 0, 0, 0],  # FOUL
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # FREE KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # CORNER KICK
                [1, 0, 0, 1, 0, 0, 0, 0, 0],  # GOAL KICK
                [0, 0, 1, 0, 0, 0, 0, 0, 0],  # PENALTY KICK
            ]

            # Depending on the position, some events will be added to the matrix
            if state.position in OFF_POSITIONS:
                transition_matrix[EventType.PASS.value][EventType.SHOT.value] = 3
                transition_matrix[EventType.CROSS.value][EventType.SHOT.value] = 3
                transition_matrix[EventType.FREE_KICK.value][EventType.SHOT.value] = 3

                # If a change of possession occurs after a shot, a defensive team could still try to shoot
                # In a defensive pitch position. Let's make sure it only tries it in the offensive positions
                transition_matrix[EventType.SHOT.value][EventType.SHOT.value] = 3

    if state.position == PitchPosition.OFF_BOX:
        transition_matrix[EventType.FOUL.value][EventType.FREE_KICK.value] = 0
        transition_matrix[EventType.FOUL.value][EventType.PENALTY_KICK.value] = 1

    return transition_matrix
