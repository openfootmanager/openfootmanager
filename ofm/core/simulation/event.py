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
from dataclasses import dataclass
from typing import Tuple
from ..football.club import TeamSimulation
from abc import abstractmethod


class Possession(Enum):
    HOME_TEAM = 0
    AWAY_TEAM = auto()


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
    MIDFIELD_CENTER = auto()
    MIDFIELD_LEFT = auto()
    MIDFIELD_RIGHT = auto()
    OFF_MIDFIELD_CENTER = auto()
    OFF_MIDFIELD_LEFT = auto()
    OFF_MIDFIELD_RIGHT = auto()
    OFF_LEFT_CORNER = auto()
    OFF_RIGHT_CORNER = auto()
    OFF_BOX = auto()


class EventType(Enum):
    """
    Enum of possible events. Pretty much self-explanatory.
    """
    PASS = 0
    SHOT = auto()
    CROSS = auto()
    FOUL = auto()
    FREE_KICK = auto()
    CORNER_KICK = auto()
    GOAL_KICK = auto()
    PENALTY_KICK = auto()


@dataclass
class GameState:
    minutes: float
    possession: Possession
    position: PitchPosition


@dataclass
class SimulationEvent:
    event_type: EventType
    state: GameState

    @abstractmethod
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> Tuple[Possession, PitchPosition]:
        pass


@dataclass
class PassEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> Tuple[Possession, PitchPosition]:
        pass
