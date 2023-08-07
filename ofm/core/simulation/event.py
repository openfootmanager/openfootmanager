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
from abc import abstractmethod
from copy import deepcopy
from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

from ..football.player import PlayerSimulation
from ..football.team_simulation import TeamSimulation
from . import OFF_POSITIONS, PITCH_EQUIVALENTS, PitchPosition
from .event_type import EventType
from .team_strategy import *


class EventOutcome(Enum):
    PASS_MISS = 0
    PASS_SUCCESS = auto()
    PASS_INTERCEPT = auto()
    PASS_OFFSIDE = auto()  # only if PASS_SUCCESS happened first
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
    SHOT_HIT_POST = auto()
    SHOT_BLOCKED = auto()
    SHOT_ON_GOAL = auto()
    SHOT_SAVED = auto()  # SHOT_SAVED only if SHOT_ON_GOAL happened first
    GOAL = auto()  # GOAL only if SHOT_ON_GOAL happened first
    OWN_GOAL = auto()


@dataclass
class SimulationEvent:
    event_type: EventType
    state: GameState
    outcome: Optional[EventOutcome] = None
    player_event: Optional[PlayerSimulation] = None
    player2_event: Optional[PlayerSimulation] = None

    @abstractmethod
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        pass


class EventFactory:
    def get_possible_events(
        self,
        teams: tuple[TeamSimulation, TeamSimulation],
        state: GameState,
        last_event: Optional[SimulationEvent],
    ) -> list[list[EventType] | list[float]]:
        if last_event is None:
            return [[EventType.PASS], [1.0]]

        if last_event.outcome == EventOutcome.GOAL:
            return [[EventType.PASS], [1.0]]

        attacking_team = teams[0]
        transition_matrix = team_general_strategy(attacking_team.team_strategy)

        return [
            [
                EventType(i)
                for i, _ in enumerate(transition_matrix[last_event.event_type.value])
            ],
            list(transition_matrix[last_event.event_type.value]),
        ]

    def get_event(self, _state: GameState, event_type: EventType) -> SimulationEvent:
        state = deepcopy(_state)
        if event_type == EventType.PASS:
            return PassEvent(EventType.PASS, state)
        elif event_type == EventType.DRIBBLE:
            return DribbleEvent(EventType.DRIBBLE, state)
        elif event_type == EventType.FOUL:
            return FoulEvent(EventType.FOUL, state)
        elif event_type == EventType.SHOT:
            return ShotEvent(EventType.SHOT, state)
        elif event_type == EventType.CROSS:
            return CrossEvent(EventType.CROSS, state)
        elif event_type == EventType.CORNER_KICK:
            return CornerKickEvent(EventType.CORNER_KICK, state)
        elif event_type == EventType.FREE_KICK:
            return FreeKickEvent(EventType.FREE_KICK, state)
        elif event_type == EventType.GOAL_KICK:
            return GoalKickEvent(EventType.GOAL_KICK, state)
        elif event_type == EventType.PENALTY_KICK:
            return ShotEvent(EventType.PENALTY_KICK, state)

        return NotImplemented


@dataclass
class PassEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        """
        Calculate the outcome of an event in a soccer game.

        This function takes in the attacking team and defending team as parameters and returns the resulting game state after the event.
        The event is simulated using a transition matrix that represents the probabilities of transitioning from one position on the field to another.
        The transition matrix is based on the team's strategy.

        Parameters:
        - attacking_team (TeamSimulation): The attacking team in the game.
        - defending_team (TeamSimulation): The defending team in the game.

        Returns:
        - GameState: The resulting game state after the event.
        """
        # Transition matrix for each position on the field
        team_strategy = attacking_team.team_strategy
        transition_matrix = team_pass_strategy(team_strategy)
        probabilities = transition_matrix[self.state.position.value]
        end_position = random.choices(list(PitchPosition), probabilities)[0]
        distance = (
            end_position.value - self.state.position.value
        )  # distance from current position to end position

        if attacking_team.player_in_possession is None:
            attacking_team.player_in_possession = attacking_team.get_player_on_pitch(
                self.state.position
            )
            defending_team.player_in_possession = None

        attacking_player = attacking_team.player_in_possession
        receiving_player = attacking_team.get_player_on_pitch(
            end_position, attacking_player
        )
        defending_player = defending_team.get_player_on_pitch(self.state.position)
        outcomes = [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_SUCCESS,
            EventOutcome.PASS_INTERCEPT,
        ]

        pass_success = (
            (
                attacking_player.attributes.intelligence.passing
                + attacking_player.attributes.intelligence.vision
            )
            / 2
        ) - distance

        outcome_probability = [
            100.0 - pass_success,  # PASS_MISS
            pass_success,  # PASS_SUCCESS
            (
                defending_player.attributes.defensive.positioning
                + defending_player.attributes.defensive.interception
            )
            / 2,  # PASS_INTERCEPT
        ]

        self.outcome = random.choices(outcomes, outcome_probability)[0]

        if end_position in OFF_POSITIONS and self.outcome == EventOutcome.PASS_SUCCESS:
            outcomes = [EventOutcome.PASS_SUCCESS, EventOutcome.PASS_OFFSIDE]
            not_offside_probability = (
                receiving_player.attributes.offensive.positioning
                + receiving_player.attributes.intelligence.team_work
            ) / 2
            outcome_probability = [
                not_offside_probability,
                100 - not_offside_probability,
            ]
            self.outcome = random.choices(outcomes, outcome_probability)[0]

        if self.outcome in [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_INTERCEPT,
            EventOutcome.PASS_OFFSIDE,
        ]:
            attacking_team.in_possession = False
            defending_team.in_possession = True
            attacking_team.player_in_possession = None
            defending_team.player_in_possession = defending_player
            end_position = PITCH_EQUIVALENTS[end_position]
        else:
            attacking_team.player_in_possession = receiving_player

        return GameState(self.state.minutes, end_position)


@dataclass
class FoulEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        player_fouled = attacking_team.get_player_on_pitch(self.state.position)
        player_fouling = defending_team.get_player_on_pitch(self.state.position)

        foul_seriousness = player_fouled
        return GameState(self.state.minutes, self.state.position)


@dataclass
class CornerKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Corner {self.state.position}")
        return GameState(self.state.minutes, self.state.position)


@dataclass
class ShotEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        event_outcomes = [
            EventOutcome.SHOT_HIT_POST,
            EventOutcome.SHOT_MISS,
            EventOutcome.SHOT_BLOCKED,
            EventOutcome.SHOT_ON_GOAL,
        ]
        player_shot = attacking_team.get_player_on_pitch(self.state.position)
        print(f"{player_shot.player.details.short_name} shoots!")
        event_probabilities = []
        return GameState(self.state.minutes, self.state.position)


@dataclass
class CrossEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Cross {self.state.position}")
        return GameState(self.state.minutes, self.state.position)


@dataclass
class GoalKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Goal Kick {self.state.position.name}")
        return GameState(self.state.minutes, self.state.position)


@dataclass
class FreeKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"FreeKick {self.state.position.name}")
        return GameState(self.state.minutes, self.state.position)


@dataclass
class DribbleEvent(SimulationEvent):
    def calculate_event(
        self, attacking_team: TeamSimulation, defending_team: TeamSimulation
    ) -> GameState:
        attacking_player = attacking_team.player_in_possession
        defending_player = defending_team.get_player_on_pitch(self.state.position)

        outcomes = [
            EventOutcome.DRIBBLE_SUCCESS,
            EventOutcome.DRIBBLE_FAIL,
        ]

        outcome_probability = [
            (
                attacking_player.attributes.intelligence.ball_control
                + attacking_player.attributes.intelligence.dribbling
                + attacking_player.attributes.intelligence.skills
            )
            / 3,
            (
                defending_player.attributes.defensive.positioning
                + defending_player.attributes.defensive.tackling
                + defending_player.attributes.physical.strength
            )
            / 3,
        ]

        self.outcome = random.choices(outcomes, outcome_probability)[0]

        if self.outcome == EventOutcome.DRIBBLE_FAIL:
            self.attacking_team.in_possession = False
            self.attacking_team.player_in_possession = None
            self.defending_team.player_in_possession = defending_player
            self.defending_team.in_possession = True
        return GameState(self.state.minutes, self.state.position)
