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

from ...football.team_simulation import TeamSimulation
from .. import PITCH_EQUIVALENTS, PitchPosition
from ..event import CommentaryImportance, EventOutcome, SimulationEvent
from ..game_state import GameState
from ..team_strategy import team_pass_strategy


@dataclass
class DribbleEvent(SimulationEvent):
    commentary_importance = CommentaryImportance.LOW

    def calculate_event(
        self, attacking_team: TeamSimulation, defending_team: TeamSimulation
    ) -> GameState:
        self.attacking_player = attacking_team.player_in_possession
        self.defending_player = defending_team.get_player_on_pitch(self.state.position)

        # Player moves the ball according to the transition matrix
        # I'm using the pass transition matrix, we can rename it accordingly
        transition_matrix = team_pass_strategy(attacking_team.team_strategy)
        probability = transition_matrix[self.state.position]
        end_position = random.choices(list(PitchPosition), probability)[0]

        # Dribble distance
        distance = abs(end_position.value - self.state.position.value)

        outcomes = [
            EventOutcome.DRIBBLE_SUCCESS,
            EventOutcome.DRIBBLE_FAIL,
        ]

        dribble_success = (
            self.attacking_player.attributes.intelligence.ball_control
            + self.attacking_player.attributes.intelligence.dribbling
            + self.attacking_player.attributes.intelligence.skills
        ) / 3

        if self.attacking_player.attributes.intelligence.skills >= 80:
            dribble_success = (
                self.attacking_player.attributes.intelligence.ball_control
                + self.attacking_player.attributes.intelligence.dribbling
                + self.attacking_player.attributes.intelligence.skills * 3
            ) / 5

        outcome_probability = [
            dribble_success - distance,
            (
                self.defending_player.attributes.defensive.positioning
                + self.defending_player.attributes.defensive.tackling
                + self.defending_player.attributes.physical.strength
            )
            / 3,
        ]

        self.outcome = random.choices(outcomes, outcome_probability)[0]

        if self.outcome == EventOutcome.DRIBBLE_FAIL:
            self.commentary.append(f"{self.defending_player} steals the ball!")
            attacking_team.in_possession = False
            attacking_team.player_in_possession = None
            defending_team.player_in_possession = self.defending_player
            defending_team.in_possession = True
            self.state.position = PITCH_EQUIVALENTS[end_position]
        else:
            self.state.position = end_position
            self.commentary.append(f"{self.attacking_player} dribbles the defender!")

        return self.state
