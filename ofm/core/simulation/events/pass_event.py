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
from ..event_type import EventType
from ..game_state import GameState
from ..team_strategy import team_pass_strategy


@dataclass
class PassEvent(SimulationEvent):
    receiving_player: Optional[PlayerSimulation] = None

    def get_end_position(self, attacking_team) -> PitchPosition:
        if self.event_type == EventType.CORNER_KICK:
            return self.state.position

        team_strategy = attacking_team.team_strategy
        transition_matrix = team_pass_strategy(team_strategy)
        probabilities = transition_matrix[self.state.position.value]
        return random.choices(list(PitchPosition), probabilities)[0]

    def get_pass_primary_outcome(self, distance) -> EventOutcome:
        outcomes = [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_SUCCESS,
            EventOutcome.PASS_INTERCEPT,
        ]

        if self.event_type == EventType.FREE_KICK:
            pass_success = (
                (
                    self.attacking_player.attributes.intelligence.passing
                    + self.attacking_player.attributes.intelligence.vision
                    + self.attacking_player.attributes.offensive.free_kick * 2
                )
                / 4
            ) - distance
        else:
            pass_success = (
                (
                    self.attacking_player.attributes.intelligence.passing
                    + self.attacking_player.attributes.intelligence.vision
                )
                / 2
            ) - distance

        outcome_probability = [
            100.0 - pass_success,  # PASS_MISS
            pass_success,  # PASS_SUCCESS
            (
                self.defending_player.attributes.defensive.positioning
                + self.defending_player.attributes.defensive.interception
            )
            / 2,  # PASS_INTERCEPT
        ]

        return random.choices(outcomes, outcome_probability)[0]

    def get_secondary_outcome(self) -> EventOutcome:
        outcomes = [EventOutcome.PASS_SUCCESS, EventOutcome.PASS_OFFSIDE]
        not_offside_probability = (
            self.receiving_player.attributes.offensive.positioning
            + self.receiving_player.attributes.intelligence.team_work
        ) / 2
        outcome_probability = [
            not_offside_probability,
            100 - not_offside_probability,
        ]
        return random.choices(outcomes, outcome_probability)[0]

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
        self.attacking_player.statistics.passes += 1

        self.outcome = self.get_pass_primary_outcome(distance)

        if (
            end_position in OFF_POSITIONS
            and self.outcome == EventOutcome.PASS_SUCCESS
            and self.event_type != EventType.CORNER_KICK
        ):
            self.outcome = self.get_secondary_outcome()

        if self.outcome in [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_INTERCEPT,
            EventOutcome.PASS_OFFSIDE,
        ]:
            self.attacking_player.statistics.passes_missed += 1
            print(f"{self.attacking_player} failed to pass the ball!")
            if self.outcome == EventOutcome.PASS_INTERCEPT:
                self.defending_player.statistics.interceptions += 1
            self.state = self.change_possession(
                attacking_team, defending_team, self.defending_player, end_position
            )
        else:
            print(f"{self.attacking_player} passed the ball to {self.receiving_player}")
            attacking_team.player_in_possession = self.receiving_player

        attacking_team.update_stats()
        defending_team.update_stats()
        return GameState(self.state.minutes, end_position)
