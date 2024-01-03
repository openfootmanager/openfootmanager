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
from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

from ...football.team_simulation import TeamSimulation
from ..event import CommentaryImportance, SimulationEvent
from ..event_type import EventType
from ..game_state import GameState
from ..team_strategy import team_corner_kick_strategy
from .cross_event import CrossEvent
from .pass_event import PassEvent


class CornerKickType(Enum):
    PASS = auto()
    CROSS = auto()


@dataclass
class CornerKickEvent(SimulationEvent):
    commentary_importance = CommentaryImportance.MEDIUM
    corner_kick_type: Optional[CornerKickType] = None
    sub_event: Optional[PassEvent | CrossEvent] = None

    def get_corner_kick_type(self, attacking_team: TeamSimulation) -> CornerKickType:
        team_strategy = attacking_team.team_strategy
        probabilities = team_corner_kick_strategy(team_strategy)
        corner_types = list(CornerKickType)
        return random.choices(corner_types, probabilities)

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.corner_kick_type = self.get_corner_kick_type(attacking_team)
        is_pass = self.corner_kick_type == CornerKickType.PASS
        self.attacking_player = attacking_team.get_best_corner_kick_taker(is_pass)

        self.commentary.append(
            f"{self.attacking_player} goes to the ball to take the corner kick!"
        )

        if self.corner_kick_type == CornerKickType.PASS:
            self.sub_event = PassEvent(
                EventType.CORNER_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
            )
        else:
            self.sub_event = CrossEvent(
                EventType.CORNER_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
            )

        self.state = self.sub_event.calculate_event(attacking_team, defending_team)
        self.defending_player = self.sub_event.defending_player
        self.outcome = self.sub_event.outcome
        self.commentary.extend(self.sub_event.commentary)

        return self.state
