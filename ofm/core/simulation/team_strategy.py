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

from ..simulation.event_type import EventType
from . import OFF_POSITIONS, PitchPosition
from .game_state import GameState


class TeamStrategy(Enum):
    NORMAL = 0
    KEEP_POSSESSION = auto()
    COUNTER_ATTACK = auto()


def get_team_foul_values(strategy: TeamStrategy) -> int:
    """
    Teams without the ball dictate foul values
    """
    match strategy:
        case TeamStrategy.NORMAL:
            return 4
        case TeamStrategy.KEEP_POSSESSION:
            return 2
        case TeamStrategy.COUNTER_ATTACK:
            return 5


def team_pass_strategy(strategy: TeamStrategy) -> dict[PitchPosition, list[int]]:
    """
    Returns the transition matrix of PitchPositions for passing.
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
                PitchPosition.DEF_BOX: [10, 20, 15, 20, 15, 10, 5, 3, 1, 1, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [5, 15, 5, 15, 15, 5, 10, 15, 5, 3, 2, 0, 1, 0, 0],
                PitchPosition.DEF_RIGHT: [5, 5, 15, 15, 5, 15, 5, 15, 10, 3, 0, 2, 0, 1, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [0, 2, 5, 15, 20, 20, 15, 15, 10, 3, 3, 3, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 0, 0, 15, 15, 5, 15, 20, 0, 10, 10, 2, 2, 1, 1],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 15, 5, 15, 0, 15, 20, 10, 1, 10, 1, 2, 1],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 10, 20, 1, 15, 15, 1, 5, 1, 3],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 10, 15, 10, 10, 10, 5, 3, 3, 3],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 1, 20, 15, 15, 1, 15, 1, 5, 3],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 10, 15, 15, 10, 15, 10],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 2, 3, 2, 10, 5, 0, 30, 0, 15],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 2, 2, 3, 10, 0, 5, 0, 30, 15],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 1, 3, 2, 5, 10, 0, 20, 0, 59],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 1, 2, 3, 5, 0, 10, 0, 20, 59],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 15, 15, 64],
            }
        case TeamStrategy.COUNTER_ATTACK:
            return {
                PitchPosition.DEF_BOX: [5, 10, 5, 15, 15, 10, 5, 5, 5, 5, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [5, 15, 5, 15, 15, 5, 10, 15, 5, 3, 2, 0, 1, 0, 0],
                PitchPosition.DEF_RIGHT: [5, 5, 15, 15, 5, 15, 5, 15, 10, 3, 0, 2, 0, 1, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [0, 2, 5, 15, 20, 20, 15, 15, 10, 3, 3, 3, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 0, 0, 15, 15, 5, 15, 20, 0, 10, 10, 2, 2, 1, 1],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 15, 5, 15, 0, 15, 20, 10, 1, 10, 1, 2, 1],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 10, 20, 1, 15, 15, 1, 5, 1, 3],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 10, 15, 10, 10, 10, 5, 3, 3, 3],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 1, 20, 15, 15, 1, 15, 1, 5, 3],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 15, 10, 10, 5, 5, 5],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 2, 3, 2, 10, 5, 0, 30, 0, 15],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 2, 2, 3, 10, 0, 5, 0, 30, 15],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 1, 3, 2, 5, 10, 0, 20, 0, 59],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 1, 2, 3, 5, 0, 10, 0, 20, 59],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 15, 15, 64],
            }


# fmt: on
def team_cross_strategy(strategy: TeamStrategy) -> dict[PitchPosition, list[int]]:
    # fmt: off
    match strategy:
        case TeamStrategy.NORMAL:
            return {
                PitchPosition.DEF_BOX: [5, 5, 5, 10, 15, 15, 10, 10, 10, 5, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [5, 15, 5, 15, 15, 5, 10, 15, 5, 3, 2, 0, 1, 0, 0],
                PitchPosition.DEF_RIGHT: [5, 5, 15, 15, 5, 15, 5, 15, 10, 3, 0, 2, 0, 1, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [0, 2, 5, 15, 20, 20, 15, 15, 10, 3, 3, 3, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 0, 0, 15, 15, 5, 15, 20, 0, 10, 10, 2, 2, 1, 1],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 15, 5, 15, 0, 15, 20, 10, 1, 10, 1, 2, 1],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 10, 20, 1, 15, 15, 1, 5, 1, 3],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 10, 15, 10, 10, 10, 5, 3, 3, 3],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 1, 20, 15, 15, 1, 15, 1, 5, 3],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 15, 10, 10, 5, 5, 5],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 2, 3, 2, 10, 5, 0, 30, 0, 15],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 2, 2, 3, 10, 0, 5, 0, 30, 15],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 1, 3, 2, 5, 10, 0, 20, 0, 59],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 1, 2, 3, 5, 0, 10, 0, 20, 59],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 15, 15, 64],
            }
        case TeamStrategy.KEEP_POSSESSION:
            return {
                PitchPosition.DEF_BOX: [15, 20, 15, 20, 15, 10, 5, 5, 3, 1, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [10, 15, 5, 15, 15, 5, 10, 15, 5, 3, 2, 0, 1, 0, 0],
                PitchPosition.DEF_RIGHT: [10, 5, 15, 15, 5, 15, 5, 15, 10, 3, 0, 2, 0, 1, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [5, 10, 15, 20, 20, 15, 10, 10, 5, 2, 2, 3, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_LEFT: [2, 5, 5, 15, 15, 10, 15, 20, 2, 5, 5, 1, 2, 1, 1],
                PitchPosition.DEF_MIDFIELD_RIGHT: [2, 1, 2, 15, 5, 15, 5, 15, 10, 5, 1, 5, 1, 2, 1],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 5, 10, 5, 10, 20, 2, 15, 15, 2, 5, 2, 7],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 1, 5, 2, 5, 10, 5, 10, 10, 5, 3, 3, 7],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 1, 0, 1, 2, 15, 10, 15, 2, 15, 2, 5, 7],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 3, 3, 3, 15, 10, 10, 5, 5, 5],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 1, 1, 1, 10, 5, 0, 30, 0, 15],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 1, 1, 1, 10, 0, 5, 0, 30, 15],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 1, 1, 1, 5, 10, 0, 15, 0, 67],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 1, 1, 1, 5, 0, 10, 0, 15, 67],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 15, 15, 64],
            }
        case TeamStrategy.COUNTER_ATTACK:
            return {
                PitchPosition.DEF_BOX: [5, 10, 5, 15, 15, 10, 5, 5, 5, 5, 0, 0, 0, 0, 0],
                PitchPosition.DEF_LEFT: [5, 15, 5, 15, 15, 5, 10, 15, 5, 3, 2, 0, 1, 0, 0],
                PitchPosition.DEF_RIGHT: [5, 5, 15, 15, 5, 15, 5, 15, 10, 3, 0, 2, 0, 1, 0],
                PitchPosition.DEF_MIDFIELD_CENTER: [0, 2, 5, 15, 20, 20, 15, 15, 10, 3, 3, 3, 1, 1, 1],
                PitchPosition.DEF_MIDFIELD_LEFT: [0, 0, 0, 15, 15, 5, 15, 20, 0, 10, 10, 2, 2, 1, 1],
                PitchPosition.DEF_MIDFIELD_RIGHT: [0, 0, 0, 15, 5, 15, 0, 15, 20, 10, 1, 10, 1, 2, 1],
                PitchPosition.MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 10, 20, 1, 15, 15, 1, 5, 1, 3],
                PitchPosition.MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 10, 15, 10, 10, 10, 5, 3, 3, 3],
                PitchPosition.MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 1, 20, 15, 15, 1, 15, 1, 5, 3],
                PitchPosition.OFF_MIDFIELD_CENTER: [0, 0, 0, 0, 0, 0, 5, 5, 5, 15, 10, 10, 5, 5, 5],
                PitchPosition.OFF_MIDFIELD_LEFT: [0, 0, 0, 0, 0, 0, 2, 3, 2, 10, 5, 0, 30, 0, 15],
                PitchPosition.OFF_MIDFIELD_RIGHT: [0, 0, 0, 0, 0, 0, 2, 2, 3, 10, 0, 5, 0, 30, 15],
                PitchPosition.OFF_LEFT: [0, 0, 0, 0, 0, 0, 1, 3, 2, 5, 10, 0, 15, 0, 67],
                PitchPosition.OFF_RIGHT: [0, 0, 0, 0, 0, 0, 1, 2, 3, 5, 0, 10, 0, 15, 67],
                PitchPosition.OFF_BOX: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 15, 15, 64],
            }


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
        case TeamStrategy.COUNTER_ATTACK:
            return [0.3, 0.7]


def team_general_strategy(
    attacking_team_strategy: TeamStrategy,
    def_team_strategy: TeamStrategy,
    state: GameState,
) -> list[int]:
    """
    Gets the probability of the events from the attacking team.

    Returns:
        [ Probability of passing, probability of crossing, probability of dribble, probability of foul, probability of shot ]
    """
    foul_value = get_team_foul_values(def_team_strategy)
    probability = {
        EventType.PASS: 20,
        EventType.CROSS: 20,
        EventType.DRIBBLE: 2,
        EventType.FOUL: foul_value,
        EventType.SHOT: 0,
    }
    match attacking_team_strategy:
        case TeamStrategy.NORMAL:
            probability = {
                EventType.PASS: 40,
                EventType.CROSS: 10,
                EventType.DRIBBLE: 1,
                EventType.FOUL: foul_value,
                EventType.SHOT: 0,
            }

            if state.position in OFF_POSITIONS:
                probability[EventType.DRIBBLE] = 4

            if state.position == PitchPosition.DEF_BOX:
                probability[EventType.FOUL] = 1

            if state.position == PitchPosition.OFF_BOX:
                probability[EventType.SHOT] = 5
                probability[EventType.FOUL] = 1
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[EventType.SHOT] = 2
                probability[EventType.CROSS] = 30
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[EventType.SHOT] = 1
        case TeamStrategy.KEEP_POSSESSION:
            probability = {
                EventType.PASS: 80,
                EventType.CROSS: 10,
                EventType.DRIBBLE: 1,
                EventType.FOUL: foul_value,
                EventType.SHOT: 0,
            }

            if state.position in OFF_POSITIONS:
                probability[EventType.DRIBBLE] = 2

            if state.position == PitchPosition.DEF_BOX:
                probability[EventType.FOUL] = 1

            if state.position == PitchPosition.OFF_BOX:
                probability[EventType.SHOT] = 5
                probability[EventType.FOUL] = 1
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[EventType.SHOT] = 2
                probability[EventType.CROSS] = 20
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[EventType.SHOT] = 1
        case TeamStrategy.COUNTER_ATTACK:
            probability = {
                EventType.PASS: 30,
                EventType.CROSS: 50,
                EventType.DRIBBLE: 2,
                EventType.FOUL: foul_value,
                EventType.SHOT: 0,
            }

            if state.position in OFF_POSITIONS:
                probability[EventType.DRIBBLE] = 3

            if state.position == PitchPosition.DEF_BOX:
                probability[EventType.FOUL] = 1

            if state.position == PitchPosition.OFF_BOX:
                probability[EventType.SHOT] = 5
                probability[EventType.FOUL] = 1
            if state.position in [
                PitchPosition.OFF_LEFT,
                PitchPosition.OFF_RIGHT,
            ]:
                probability[EventType.SHOT] = 2
                probability[EventType.CROSS] = 50
            if state.position == PitchPosition.OFF_MIDFIELD_CENTER:
                probability[EventType.SHOT] = 1

    return list(probability.values())
