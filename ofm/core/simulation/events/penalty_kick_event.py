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

from ...football.team_simulation import TeamSimulation
from .. import PitchPosition
from ..event import SimulationEvent, EventOutcome
from ..game_state import GameState


@dataclass
class PenaltyKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.defending_player = defending_team.formation.gk
        self.attacking_player = attacking_team.get_best_penalty_taker()

        outcomes = [EventOutcome.SHOT_ON_GOAL, EventOutcome.SHOT_MISS]

        shot_success = (
            self.attacking_player.attributes.offensive.penalty * 2
            + self.attacking_player.attributes.offensive.shot_accuracy
        ) / 3

        outcome_probability = [
            shot_success,
            110 - shot_success,
        ]

        self.attacking_player.statistics.shots += 1

        outcome = random.choices(outcomes, outcome_probability)[0]

        if outcome == EventOutcome.SHOT_ON_GOAL:
            self.attacking_player.statistics.shots_on_target += 1
            outcomes = [EventOutcome.SHOT_SAVED, EventOutcome.GOAL]
            gk_skills = (
                self.defending_player.attributes.gk.penalty * 2
                + self.defending_player.attributes.gk.jumping
            ) / 3
            outcome_probability = [
                gk_skills,
                125 - gk_skills,
            ]
            self.outcome = random.choices(outcomes, outcome_probability)[0]

        if self.outcome == EventOutcome.SHOT_SAVED:
            outcomes = [
                EventOutcome.SHOT_LEFT_CORNER_KICK,
                EventOutcome.SHOT_RIGHT_CORNER_KICK,
                EventOutcome.SHOT_SAVED_SECURED,
            ]
            self.defending_player.statistics.shots_saved += 1
            self.outcome = random.choice(outcomes)
        if self.outcome == EventOutcome.SHOT_MISS:
            outcomes = [EventOutcome.SHOT_HIT_POST, EventOutcome.SHOT_GOAL_KICK]
            self.attacking_player.statistics.shots_missed += 1
            self.outcome = random.choice(outcomes)
        elif self.outcome == EventOutcome.SHOT_SAVED_SECURED:
            self.change_possession(
                attacking_team,
                defending_team,
                self.defending_player,
                self.state.position,
            )
        elif self.outcome == EventOutcome.SHOT_RIGHT_CORNER_KICK:
            self.state.position = PitchPosition.OFF_RIGHT
        elif self.outcome == EventOutcome.SHOT_LEFT_CORNER_KICK:
            self.state.position = PitchPosition.OFF_LEFT

        return GameState(self.state.minutes, self.state.position)
