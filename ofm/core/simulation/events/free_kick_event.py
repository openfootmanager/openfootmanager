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
from typing import Optional

from ...football.team_simulation import TeamSimulation
from .. import OFF_POSITIONS
from ..event import CommentaryImportance, SimulationEvent
from ..event_type import EventType, FreeKickType
from ..game_state import GameState
from .cross_event import CrossEvent
from .pass_event import PassEvent
from .shot_event import ShotEvent


@dataclass
class FreeKickEvent(SimulationEvent):
    commentary_importance = CommentaryImportance.MEDIUM
    free_kick_type: Optional[FreeKickType] = None
    sub_event: Optional[PassEvent | ShotEvent | CrossEvent] = None

    def get_free_kick_type(self):
        free_kick_types = [
            FreeKickType.PASS,
            FreeKickType.CROSS,
        ]

        if self.state.position in OFF_POSITIONS:
            free_kick_types.append(FreeKickType.DIRECT_SHOT)

        return random.choice(free_kick_types)

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        if self.state.position in OFF_POSITIONS:
            self.attacking_player = attacking_team.get_best_free_kick_taker()
        else:
            self.attacking_player = attacking_team.get_player_on_pitch(
                self.state.position
            )

        self.free_kick_type = self.get_free_kick_type()

        self.commentary.append(
            f"{self.attacking_player} goes to the ball to take the free kick!"
        )

        if self.free_kick_type == FreeKickType.PASS:
            self.sub_event = PassEvent(
                EventType.FREE_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
            )
        elif self.free_kick_type == FreeKickType.DIRECT_SHOT:
            self.sub_event = ShotEvent(
                EventType.FREE_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
            )
        else:
            self.sub_event = CrossEvent(
                EventType.FREE_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
            )

        self.state = self.sub_event.calculate_event(attacking_team, defending_team)
        self.defending_player = self.sub_event.defending_player
        self.outcome = self.sub_event.outcome
        self.commentary.extend(self.sub_event.commentary)

        return self.state
