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
from .. import OFF_POSITIONS, PitchPosition
from ..event import SimulationEvent, EventOutcome
from ..game_state import GameState
from ..team_strategy import team_cross_strategy


@dataclass
class CrossEvent(SimulationEvent):
    receiving_player: Optional[PlayerSimulation] = None

    def get_end_position(self, attacking_team) -> PitchPosition:
        team_strategy = attacking_team.team_strategy
        transition_matrix = team_cross_strategy(team_strategy)
        probabilities = transition_matrix[self.state.position.value]
        return random.choices(list(PitchPosition), probabilities)[0]

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        # Transition matrix for each position on the field
        end_position = self.get_end_position(attacking_team)
        distance = abs(
            end_position.value - self.state.position.value
        )  # distance from current position to end position

        if self.attacking_player is None:
            self.attacking_player = attacking_team.player_in_possession
        if self.defending_player is None:
            self.defending_player = defending_team.get_player_on_pitch(end_position)
        self.receiving_player = attacking_team.get_player_on_pitch(end_position)
        self.attacking_player.statistics.crosses += 1

        self.outcome = self.get_cross_primary_outcome(distance)

        return GameState(self.state.minutes, self.state.position)
