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

from ...football.player import PlayerSimulation
from ...football.team_simulation import TeamSimulation
from .. import OFF_POSITIONS, PitchPosition
from ..event import CommentaryImportance, EventOutcome, SimulationEvent
from ..event_type import EventType
from ..game_state import GameState
from ..team_strategy import team_cross_strategy


@dataclass
class CrossEvent(SimulationEvent):
    commentary_importance = CommentaryImportance.LOW
    receiving_player: Optional[PlayerSimulation] = None

    def get_end_position(self, attacking_team) -> PitchPosition:
        if self.event_type == EventType.CORNER_KICK:
            return PitchPosition.OFF_BOX
        if self.event_type == EventType.GOAL_KICK:
            positions = [
                PitchPosition.MIDFIELD_CENTER,
                PitchPosition.MIDFIELD_RIGHT,
                PitchPosition.MIDFIELD_LEFT,
                PitchPosition.OFF_MIDFIELD_LEFT,
                PitchPosition.OFF_MIDFIELD_RIGHT,
                PitchPosition.OFF_MIDFIELD_CENTER,
            ]
            return random.choice(positions)

        team_strategy = attacking_team.team_strategy
        transition_matrix = team_cross_strategy(team_strategy)
        probabilities = transition_matrix[self.state.position]
        return random.choices(list(PitchPosition), probabilities)[0]

    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        outcomes = [
            EventOutcome.CROSS_MISS,
            EventOutcome.CROSS_SUCCESS,
        ]

        if self.event_type == EventType.FREE_KICK:
            cross_miss = (50 + distance) / (
                100
                + self.attacking_player.attributes.intelligence.crossing
                + self.attacking_player.attributes.intelligence.vision
                + self.attacking_player.attributes.offensive.free_kick
            )
        else:
            cross_miss = (25 + distance) / (
                100
                + self.attacking_player.attributes.intelligence.crossing
                + self.attacking_player.attributes.intelligence.vision
            )

        cross_success = 1 - cross_miss

        outcome_probability = [
            cross_miss,  # CROSS_MISS
            cross_success,  # CROSS_SUCCESS
        ]

        return random.choices(outcomes, outcome_probability)[0]

    def get_intercept_prob(self) -> EventOutcome:
        outcomes = [EventOutcome.CROSS_MISS, EventOutcome.CROSS_INTERCEPT]
        cross_intercept = (
            self.defending_player.attributes.defensive.positioning
            + self.defending_player.attributes.defensive.interception
        )
        outcome_probability = [
            200 - cross_intercept,
            cross_intercept,
        ]

        return random.choices(outcomes, outcome_probability)[0]

    def get_secondary_outcome(self) -> EventOutcome:
        outcomes = [EventOutcome.CROSS_SUCCESS, EventOutcome.CROSS_OFFSIDE]

        offside_probability = 5 / (
            200
            + self.attacking_player.attributes.offensive.positioning
            + self.attacking_player.attributes.intelligence.team_work
        )

        outcome_probability = [
            1 - offside_probability,
            offside_probability,
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
        self.attacking_player.statistics.crosses += 1

        self.outcome = self.get_cross_primary_outcome(distance)

        if self.outcome == EventOutcome.CROSS_MISS:
            self.outcome = self.get_intercept_prob()

        if (
            end_position in OFF_POSITIONS
            and self.state.position.value < end_position.value
            and self.outcome == EventOutcome.CROSS_SUCCESS
            and self.event_type != EventType.CORNER_KICK
        ):
            self.outcome = self.get_secondary_outcome()

        if self.outcome in [
            EventOutcome.CROSS_MISS,
            EventOutcome.CROSS_INTERCEPT,
            EventOutcome.CROSS_OFFSIDE,
        ]:
            self.attacking_player.statistics.crosses_missed += 1
            self.commentary.append(f"{self.attacking_player} failed to cross the ball!")
            if self.outcome == EventOutcome.CROSS_INTERCEPT:
                self.defending_player.statistics.interceptions += 1
            if self.outcome == EventOutcome.CROSS_OFFSIDE:
                attacking_team.stats.offsides += 1
                self.commentary.append(f"{self.attacking_player} was offside!")
            self.attacking_player.received_ball = None
            self.defending_player.received_ball = None
            self.receiving_player.received_ball = None
            self.state = self.change_possession(
                attacking_team, defending_team, self.defending_player, end_position
            )
        else:
            self.state.position = end_position
            self.commentary.append(
                f"{self.attacking_player} crossed the ball to {self.receiving_player}"
            )
            self.attacking_player.received_ball = None
            self.defending_player.received_ball = None
            self.receiving_player.received_ball = self.attacking_player
            attacking_team.player_in_possession = self.receiving_player

        attacking_team.update_stats()
        defending_team.update_stats()
        return self.state
