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

from . import PitchPosition
from .game_state import GameState


class TeamStrategy(Enum):
    NORMAL = 0
    KEEP_POSSESSION = auto()
    COUNTER_ATTACK = auto()
    DEFEND = auto()
    ALL_ATTACK = auto()


def get_team_foul_values(strategy: TeamStrategy) -> int:
    """
    Teams without the ball dictate foul values
    """
    match strategy:
        case TeamStrategy.NORMAL:
            return 5
        case TeamStrategy.KEEP_POSSESSION:
            return 1
        case TeamStrategy.DEFEND:
            return 15
        case TeamStrategy.COUNTER_ATTACK:
            return 5
        case TeamStrategy.ALL_ATTACK:
            return 10


def team_pass_strategy(strategy: TeamStrategy) -> dict[PitchPosition, list[int]]:
    """
    Returns the transition matrix of PitchPositions for passing.

    Each column of the matrix is the target PitchPosition, while the row is the current PitchPosition.
    The columns and rows follow the order of the PitchPosition enum.
    """
    # fmt: off
    match strategy:
        case TeamStrategy.NORMAL:
            return {
                PitchPosition.DEF_BOX: [1, 10, 10, 30, 10, 10, 5, 15, 5, 4, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [1, 10, 1, 20, 20, 1, 15, 20, 1, 5, 5, 0, 1, 0, 0],
                PitchPosition.DEF_RIGHT: [1, 1, 10, 20, 1, 20, 1, 20, 15, 5, 0, 5, 0, 1, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [0, 1, 1, 10, 15, 15, 10, 20, 10, 5, 5, 5, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 0, 0, 20, 15, 1, 20, 20, 0, 10, 10, 1, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 20, 1, 15, 0, 20, 20, 10, 1, 10, 1, 1, 1],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 15, 30, 1, 20, 20, 1, 7, 1, 5],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 15, 20, 15, 15, 10, 10, 5, 5, 5],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 1, 30, 15, 20, 1, 20, 1, 7, 5],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 25, 15, 15, 10, 10, 10],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 10, 0, 50, 0, 20],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 10, 0, 50, 20],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 0, 60],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 60],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 20, 20, 50],
            }
        case TeamStrategy.KEEP_POSSESSION:
            return {
                PitchPosition.DEF_BOX: [5, 10, 10, 25, 10, 10, 10, 10, 10, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [5, 5, 0, 25, 30, 0, 15, 20, 0, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_RIGHT: [5, 0, 10, 25, 0, 25, 0, 20, 15, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [5, 5, 5, 15, 15, 15, 10, 20, 10, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 5, 0, 15, 10, 10, 20, 20, 0, 10, 10, 0, 0, 0, 0],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 20, 0, 15, 0, 25, 20, 10, 0, 10, 0, 0, 0],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 15, 30, 0, 20, 20, 0, 10, 0, 5],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 15, 20, 15, 15, 10, 10, 5, 5, 5],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 0, 30, 15, 20, 0, 20, 0, 10, 5],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 25, 15, 15, 10, 10, 10],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 10, 0, 50, 0, 20],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 10, 0, 50, 20],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 0, 60],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 60],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 10, 10, 70],
            }
        case TeamStrategy.DEFEND:
            pass
        case TeamStrategy.COUNTER_ATTACK:
            pass
        case TeamStrategy.ALL_ATTACK:
            pass


# fmt: on
def team_cross_strategy(strategy: TeamStrategy) -> dict[PitchPosition, list[int]]:
    # fmt: off
    match strategy:
        case TeamStrategy.NORMAL:
            return {
                PitchPosition.DEF_BOX: [1, 10, 10, 30, 10, 10, 5, 15, 5, 4, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [1, 10, 1, 20, 20, 1, 15, 20, 1, 5, 5, 0, 1, 0, 0],
                PitchPosition.DEF_RIGHT: [1, 1, 10, 20, 1, 20, 1, 20, 15, 5, 0, 5, 0, 1, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [0, 1, 1, 10, 15, 15, 10, 20, 10, 5, 5, 5, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 0, 0, 20, 15, 1, 20, 20, 0, 10, 10, 1, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 20, 1, 15, 0, 20, 20, 10, 1, 10, 1, 1, 1],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 15, 30, 1, 20, 20, 1, 7, 1, 5],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 15, 20, 15, 15, 10, 10, 5, 5, 5],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 1, 30, 15, 20, 1, 20, 1, 7, 5],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 25, 15, 15, 10, 10, 10],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 10, 0, 50, 0, 20],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 10, 0, 50, 20],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 0, 60],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 60],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 20, 20, 50],
            }
        case TeamStrategy.KEEP_POSSESSION:
            return {
                PitchPosition.DEF_BOX: [5, 10, 10, 25, 10, 10, 10, 10, 10, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [5, 5, 0, 25, 30, 0, 15, 20, 0, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_RIGHT: [5, 0, 10, 25, 0, 25, 0, 20, 15, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [5, 5, 5, 15, 15, 15, 10, 20, 10, 0, 0, 0, 0, 0, 0],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 5, 0, 15, 10, 10, 20, 20, 0, 10, 10, 0, 0, 0, 0],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 20, 0, 15, 0, 25, 20, 10, 0, 10, 0, 0, 0],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 15, 30, 0, 20, 20, 0, 10, 0, 5],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 15, 20, 15, 15, 10, 10, 5, 5, 5],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 0, 30, 15, 20, 0, 20, 0, 10, 5],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 25, 15, 15, 10, 10, 10],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 10, 0, 50, 0, 20],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 10, 0, 50, 20],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 0, 60],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 25, 60],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 5, 10, 10, 70],
            }
        case TeamStrategy.DEFEND:
            # TODO: implement transition matrix for DEFEND TeamStrategy
            pass
        case TeamStrategy.COUNTER_ATTACK:
            # TODO: implement transition matrix for COUNTER_ATTACK TeamStrategy
            pass
        case TeamStrategy.ALL_ATTACK:
            # TODO: implement transition matrix for ALL_ATTACK TeamStrategy
            pass


# fmt: on
def team_corner_kick_strategy(strategy: TeamStrategy) -> list[float]:
    """
    Returns the probability to either pass the ball or to cross the ball in a corner kick

    Returns: [PASS_PROB, CROSS_PROB]
    """
    match strategy:
        case TeamStrategy.NORMAL:
            return [0.5, 0.5]
        case TeamStrategy.KEEP_POSSESSION:
            return [0.8, 0.2]
        case TeamStrategy.DEFEND:
            return [0.4, 0.6]
        case TeamStrategy.COUNTER_ATTACK:
            return [0.3, 0.7]
        case TeamStrategy.ALL_ATTACK:
            return [0.1, 0.9]


def team_general_strategy(
    attacking_team_strategy: TeamStrategy,
    def_team_strategy: TeamStrategy,
    state: GameState,
) -> list[int]:
    """
    Gets the probability of the events from the attacking team.

    Returns:
        [ Probability of passing, probability of crossing, probability of foul, probability of shot ]
    """
    foul_value = get_team_foul_values(def_team_strategy)
    probability = [20, 20, foul_value, 0]
    match attacking_team_strategy:
        case TeamStrategy.NORMAL:
            probability = [20, 20, foul_value, 0]

            if state.position == PitchPosition.OFF_BOX:
                probability[3] = 30
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[3] = 10
                probability[1] = 30
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[3] = 20
        case TeamStrategy.KEEP_POSSESSION:
            probability = [80, 15, foul_value, 0]

            if state.position == PitchPosition.OFF_BOX:
                probability[3] = 40
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[3] = 10
                probability[1] = 40
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[3] = 20
        case TeamStrategy.DEFEND:
            probability = [20, 60, foul_value, 0]

            if state.position == PitchPosition.OFF_BOX:
                probability[3] = 10
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[3] = 5
                probability[1] = 40
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[3] = 5
        case TeamStrategy.COUNTER_ATTACK:
            probability = [20, 70, foul_value, 0]

            if state.position == PitchPosition.OFF_BOX:
                probability[3] = 35
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[3] = 20
                probability[1] = 50
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[3] = 30
        case TeamStrategy.ALL_ATTACK:
            probability = [15, 60, foul_value, 0]

            if state.position == PitchPosition.OFF_BOX:
                probability[3] = 50
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[3] = 20
                probability[1] = 70
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[3] = 30

    return probability
