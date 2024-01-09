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
import random
from abc import abstractmethod
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Optional

from ..football.player import PlayerSimulation
from ..football.team_simulation import TeamSimulation
from . import PITCH_EQUIVALENTS, PitchPosition
from .event_type import EventType
from .game_state import GameState


class EventOutcome(Enum):
    PASS_MISS = 0
    PASS_SUCCESS = auto()
    PASS_INTERCEPT = auto()
    PASS_OFFSIDE = auto()  # Only if PASS_SUCCESS happened first
    DRIBBLE_FAIL = auto()
    DRIBBLE_SUCCESS = auto()
    CROSS_MISS = auto()
    CROSS_SUCCESS = auto()
    CROSS_INTERCEPT = auto()
    CROSS_OFFSIDE = auto()
    FOUL_WARNING = auto()
    FOUL_YELLOW_CARD = auto()
    FOUL_RED_CARD = auto()
    SHOT_MISS = auto()
    SHOT_ON_GOAL = auto()
    SHOT_BLOCKED = auto()
    SHOT_BLOCKED_CHANGE_POSSESSION = auto()  # Only if SHOT_BLOCKED happened first
    SHOT_BLOCKED_BACK = auto()  # Only if SHOT_BLOCKED happened first
    SHOT_SAVED = auto()  # Only if SHOT_ON_GOAL happened first
    SHOT_SAVED_SECURED = (
        auto()
    )  # Only if SHOT_SAVED happend first. Keeps the ball after saving.
    SHOT_HIT_POST = auto()  # Only if SHOT_ON_GOAL happened first
    SHOT_HIT_POST_CHANGE_POSSESSION = (
        auto()
    )  # Only if SHOT_HIT_POST happened first. Changes possession after the event.
    SHOT_LEFT_CORNER_KICK = auto()  # Only if SHOT_MISS or SHOT_SAVED happened first
    SHOT_RIGHT_CORNER_KICK = auto()  # Only if SHOT_MISS or SHOT_SAVED happened first
    SHOT_GOAL_KICK = auto()  # Only if SHOT_MISS happened first
    GOAL = auto()  # Only if SHOT_ON_GOAL happened first
    OWN_GOAL = auto()


class CommentaryImportance(Enum):
    LOW = auto()
    MEDIUM = auto()
    HIGH = auto()


@dataclass
class SimulationEvent:
    event_type: EventType
    state: GameState
    commentary_importance = CommentaryImportance
    outcome: Optional[EventOutcome] = None
    attacking_player: Optional[PlayerSimulation] = None
    defending_player: Optional[PlayerSimulation] = None
    commentary: list[str] = field(default_factory=list)
    duration: float = 0.0

    def __post_init__(self):
        self.duration = float(random.randint(1, 8))

    @abstractmethod
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        pass

    def change_possession(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
        defending_player: PlayerSimulation,
        position: PitchPosition,
    ) -> GameState:
        attacking_team.in_possession = False
        attacking_team.player_in_possession = None
        defending_team.in_possession = True
        defending_team.player_in_possession = defending_player
        position = PITCH_EQUIVALENTS[position]
        return GameState(self.state.minutes, self.state.status, position)
