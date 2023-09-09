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

from ...football.player import PlayerSimulation
from ...football.team_simulation import TeamSimulation
from .. import PitchPosition
from ..event import SimulationEvent, EventOutcome
from ..game_state import GameState


@dataclass
class GoalKickEvent(SimulationEvent):
    receiving_player: Optional[PlayerSimulation] = None

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.attacking_player = attacking_team.formation.gk

        print(f"Goal Kick {self.state.position.name}")
        return GameState(self.state.minutes, self.state.position)
