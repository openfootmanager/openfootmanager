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
import random
from dataclasses import dataclass
from typing import Optional
from enum import Enum, auto

from ...football.player import PlayerSimulation
from ...football.team_simulation import TeamSimulation
from ..event import SimulationEvent
from ..event_type import EventType
from ..game_state import GameState
from .pass_event import PassEvent
from .cross_event import CrossEvent


class GoalKickType(Enum):
    PASS = auto()
    CROSS = auto()


@dataclass
class GoalKickEvent(SimulationEvent):
    goal_kick_type: Optional[GoalKickType] = None
    receiving_player: Optional[PlayerSimulation] = None
    sub_event: Optional[PassEvent | CrossEvent] = None

    def get_goal_kick_type(self):
        goal_kick_types = list(GoalKickType)
        return random.choice(goal_kick_types)

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.attacking_player = attacking_team.formation.gk

        print(f"Goal Kick {self.state.position.name}")

        self.goal_kick_type = self.get_goal_kick_type()

        if self.goal_kick_type == GoalKickType.PASS:
            self.sub_event = PassEvent(
                EventType.GOAL_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
            )
        else:
            self.sub_event = CrossEvent(
                EventType.GOAL_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
            )

        self.state = self.sub_event.calculate_event(attacking_team, defending_team)
        self.outcome = self.sub_event.outcome
        self.defending_player = self.sub_event.defending_player

        return GameState(self.state.minutes, self.state.position)
