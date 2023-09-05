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

from .game_state import GameState


class TeamStrategy(Enum):
    NORMAL = 0
    KEEP_POSSESSION = auto()
    COUNTER_ATTACK = auto()
    DEFEND = auto()
    ALL_ATTACK = auto()


def team_pass_strategy(strategy: TeamStrategy) -> list[list[int]]:
    # fmt: off
    match strategy:
        case TeamStrategy.NORMAL:
            return [
                [0.1, 0.2, 0.1, 0.05, 0.05, 0.05, 0.1, 0.1, 0.1, 0.05, 0.05, 0.05, 0.1, 0.1, 0.1],  # DEF_BOX
                [0.2, 0.1, 0.3, 0.1, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # DEF_LEFT
                [0.2, 0.3, 0.1, 0.1, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # DEF_RIGHT
                [0.15, 0.1, 0.1, 0.1, 0.2, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # DEF_MIDFIELD_CENTER
                [0.15, 0.1, 0.1, 0.2, 0.1, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # DEF_MIDFIELD_LEFT
                [0.15, 0.1, 0.1, 0.2, 0.1, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # DEF_MIDFIELD_RIGHT
                [0.1, 0.1, 0.1, 0.15, 0.15, 0.15, 0.1, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # MIDFIELD_LEFT
                [0.1, 0.1, 0.1, 0.15, 0.15, 0.15, 0.1, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # MIDFIELD_CENTER
                [0.1, 0.1, 0.1, 0.15, 0.15, 0.15, 0.15, 0.1, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05],  # MIDFIELD_RIGHT
                [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.15, 0.15, 0.15, 0.05, 0.05, 0.05, 0.05],  # OFF_MIDFIELD_CENTER
                [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.15, 0.15, 0.15, 0.05, 0.05, 0.05, 0.05] ,  # OFF_MIDFIELD_LEFT
                [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.15, 0.15, 0.15, 0.05, 0.05, 0.05, 0.05],  # OFF_MIDFIELD_RIGHT
                [0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.1, 0.1, 0.1, 0.1, 0.1, 0.15, 0.15],  # OFF_LEFT
                [0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.1, 0.1, 0.1, 0.1, 0.1, 0.15, 0.15],  # OFF_RIGHT
                [0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.1, 0.1, 0.1, 0.1, 0.1, 0.15, 0.15],  # OFF_BOX
            ]
        case TeamStrategy.KEEP_POSSESSION:
            pass
        case TeamStrategy.DEFEND:
            pass
        case TeamStrategy.COUNTER_ATTACK:
            pass
        case TeamStrategy.ALL_ATTACK:
            pass


# fmt: on
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
        case TeamStrategy.COUNTER_ATTACK:
            # TODO: implement transition matrix for COUNTER_ATTACK TeamStrategy
            pass
        case TeamStrategy.ALL_ATTACK:
            # TODO: implement transition matrix for ALL_ATTACK TeamStrategy
            pass


def team_goal_kick_strategy(
    strategy: TeamStrategy, state: GameState
) -> list[list[int]]:
    pass


def team_general_strategy(strategy: TeamStrategy, state: GameState) -> list[list[int]]:
    match strategy:
        # fmt: off
        case TeamStrategy.NORMAL:
            transition_matrix = [
                [4, 2, 0, 1, 1, 0, 0, 0, 0],  # PASS
            ]

        case TeamStrategy.KEEP_POSSESSION:
            transition_matrix = [
                [5, 2, 0, 1, 0, 0, 0, 0, 0],  # PASS
            ]

        case TeamStrategy.DEFEND:
            transition_matrix = [
                [3, 2, 0, 1, 1, 0, 0, 0, 0],  # PASS
            ]


        case TeamStrategy.COUNTER_ATTACK:
            transition_matrix = [
                [4, 2, 0, 1, 1, 0, 0, 0, 0],  # PASS
            ]

        case TeamStrategy.ALL_ATTACK:
            transition_matrix = [
                [4, 2, 0, 2, 1, 0, 0, 0, 0],  # PASS
            ]

    return transition_matrix


# fmt: on
