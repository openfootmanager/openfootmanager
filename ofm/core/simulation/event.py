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
from enum import Enum, auto
from dataclasses import dataclass
from ..football.club import TeamSimulation
from abc import abstractmethod
from copy import deepcopy
from . import PitchPosition, PITCH_EQUIVALENTS


class EventType(Enum):
    PASS = 0
    SHOT = auto()
    CROSS = auto()
    FOUL = auto()
    FREE_KICK = auto()
    CORNER_KICK = auto()
    GOAL_KICK = auto()
    PENALTY_KICK = auto()


class EventOutcome(Enum):
    PASS_MISS = 0
    PASS_SUCCESS = auto()
    PASS_INTERCEPT = auto()
    PASS_OFFSIDE = auto()
    CROSS_MISS = auto()
    CROSS_SUCCESS = auto()
    CROSS_INTERCEPT = auto()
    CROSS_OFFSIDE = auto()
    FOUL_WARNING = auto()
    FOUL_YELLOW_CARD = auto()
    FOUL_RED_CARD = auto()
    SHOT_MISS = auto()
    SHOT_HIT_POST = auto()
    SHOT_SAVED = auto()
    GOAL = auto()
    OWN_GOAL = auto()


@dataclass
class GameState:
    minutes: float
    position: PitchPosition


@dataclass
class SimulationEvent:
    event_type: EventType
    state: GameState
    outcome: EventOutcome = None

    @abstractmethod
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        pass

class EventFactory:
    def get_possible_events(self, state: GameState, last_event: EventType) -> list[list[EventType], list[float]]:
        if last_event is None:
            return [[EventType.PASS], [1.0]]

        transition_matrix = [
            [4, 0, 1, 1, 0, 0, 0, 0],  # PASS
            [2, 1, 1, 1, 0, 1, 1, 0],  # SHOT
            [1, 0, 1, 1, 0, 1, 1, 0],  # CROSS
            [0, 0, 0, 0, 1, 0, 0, 0],  # FOUL
            [1, 0, 1, 0, 0, 0, 0, 0],  # FREE KICK
            [1, 0, 1, 0, 0, 0, 0, 0],  # CORNER KICK
            [1, 0, 1, 0, 0, 0, 0, 0],  # GOAL KICK
            [0, 1, 0, 0, 0, 0, 0, 0],  # PENALTY KICK
        ]

        # Depending on the position, some events will be added to the matrix
        if state.position in [
            PitchPosition.OFF_BOX,
            PitchPosition.OFF_RIGHT,
            PitchPosition.OFF_LEFT,
            PitchPosition.OFF_MIDFIELD_LEFT,
            PitchPosition.OFF_MIDFIELD_RIGHT,
            PitchPosition.OFF_MIDFIELD_CENTER,
        ]:
            transition_matrix[EventType.PASS.value][EventType.SHOT.value] = 1
            transition_matrix[EventType.CROSS.value][EventType.SHOT.value] = 1
            transition_matrix[EventType.FREE_KICK.value][EventType.SHOT.value] = 1

        if state.position == PitchPosition.OFF_BOX:
            # Free kick still possible if it's an offensive foul
            transition_matrix[EventType.FOUL.value][EventType.PENALTY_KICK.value] = 1

        return [
            [
                EventType(i)
                for i, _ in enumerate(transition_matrix[last_event.value])
            ],
            list(transition_matrix[last_event.value]),
        ]

    def get_event(self, _state: GameState, event_type: EventType) -> SimulationEvent:
        state = deepcopy(_state)
        if event_type == EventType.PASS:
            return PassEvent(EventType.PASS, state)
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
        # Transition matrix for each position on the field
        # TODO: Transition matrix should depend on team's strategy
        transition_matrix = [
            [1, 4, 6, 8, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1], # BOX
            [1, 4, 6, 8, 5, 2, 4, 1, 1, 1, 1, 1, 1, 1, 1], # DEF_LEFT_BOX
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], # DEF_RIGHT_BOX
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], # DEF_MIDFIELD_CENTER
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], # DEF_MIDFIELD_LEFT
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], # DEF_RIGHT_MIDFIELD
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4], # MIDFIELD_LEFT
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4], # MIDFIELD_CENTER
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4], # MIDFIELD_RIGHT
            [1, 1, 1, 1, 1, 1, 4, 6, 4, 8, 6, 6, 8, 8, 8], # OFF_MIDFIELD_CENTER
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 6], # OFF_MIDFIELD_LEFT
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 8, 6], # OFF_MIDFIELD_RIGHT
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14], # OFF_LEFT
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14], # OFF_RIGHT
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 8, 14], # OFF_BOX
        ]
        probabilities = transition_matrix[self.state.position.value]
        end_position = random.choices(list(PitchPosition), probabilities)[0]
        distance = end_position.value - self.state.position.value  # distance from current position to end position
        attacking_player = attacking_team.get_player_in_possession(self.state.position)
        defending_player = defending_team.get_player_in_possession(self.state.position)
        outcomes = [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_OFFSIDE,
            EventOutcome.PASS_SUCCESS,
            EventOutcome.PASS_INTERCEPT
        ]

        luck_factor = random.random()

        outcome_probability = [
            int(abs(distance) / attacking_player.player.details.attributes.passing * luck_factor * 10),
            0,
            int(attacking_player.player.details.attributes.passing * luck_factor * 10),
            int(defending_player.player.details.attributes.defense * luck_factor * 10),
        ]

        if end_position in [
            PitchPosition.OFF_MIDFIELD_LEFT,
            PitchPosition.OFF_MIDFIELD_CENTER,
            PitchPosition.OFF_MIDFIELD_RIGHT,
            PitchPosition.OFF_LEFT,
            PitchPosition.OFF_RIGHT,
            PitchPosition.OFF_BOX,
        ]:
            outcome_probability[1] = int(distance / attacking_player.player.details.attributes.passing * luck_factor * 10)

        self.outcome = random.choices(outcomes, outcome_probability)[0]
        minutes = self.state.minutes + 0.1
        if self.outcome in [EventOutcome.PASS_MISS, EventOutcome.PASS_INTERCEPT, EventOutcome.PASS_OFFSIDE]:
            attacking_team.in_possession = False
            defending_team.in_possession = True
            end_position = PITCH_EQUIVALENTS[end_position]

        return GameState(minutes, end_position)


@dataclass
class FoulEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Foul {self.state.position}")
        return GameState(self.state.minutes + 0.1, self.state.position)


@dataclass
class CornerKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Corner {self.state.position}")
        return GameState(self.state.minutes + 0.1, self.state.position)


@dataclass
class ShotEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Shot {self.state.position}")
        return GameState(self.state.minutes + 0.1, self.state.position)


@dataclass
class CrossEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Cross {self.state.position}")
        return GameState(self.state.minutes + 0.1, self.state.position)


@dataclass
class GoalKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Goal Kick {self.state.position}")
        return GameState(self.state.minutes + 0.1, self.state.position)


@dataclass
class FreeKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"FreeKick {self.state.position}")
        return GameState(self.state.minutes + 0.1, self.state.position)
