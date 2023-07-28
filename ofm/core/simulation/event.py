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

from ..football.club import PlayerSimulation, TeamSimulation
from ..football.team_strategy import TeamStrategyFactory
from . import DEF_POSITIONS, OFF_POSITIONS, PITCH_EQUIVALENTS, PitchPosition


class EventType(Enum):
    PASS = 0
    DRIBBLE = auto()
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
    SHOT_SAVED = auto()
    SHOT_BLOCKED = auto()
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
    outcome: Optional[EventOutcome] = None
    player_event: Optional[PlayerSimulation] = None
    player2_event: Optional[PlayerSimulation] = None
    strategy: TeamStrategyFactory = TeamStrategyFactory()

    @abstractmethod
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        pass


class EventFactory:
    def get_possible_events(
        self, state: GameState, last_event: EventType
    ) -> list[list[EventType] | list[float]]:
        if last_event is None:
            return [[EventType.PASS], [1.0]]

        team_strategy = self.state.attacking_team.team_strategy
        transition_matrix = self.strategy.get_game_transition_matrix(team_strategy)

        # Depending on the position, some events will be added to the matrix
        if state.position in OFF_POSITIONS:
            transition_matrix[EventType.PASS.value][EventType.SHOT.value] = 1
            transition_matrix[EventType.CROSS.value][EventType.SHOT.value] = 1
            transition_matrix[EventType.FREE_KICK.value][EventType.SHOT.value] = 1

        if state.position == PitchPosition.OFF_BOX:
            # Free kick still possible if it's an offensive foul
            transition_matrix[EventType.FOUL.value][EventType.PENALTY_KICK.value] = 1

        return [
            [EventType(i) for i, _ in enumerate(transition_matrix[last_event.value])],
            list(transition_matrix[last_event.value]),
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
        transition_matrix = self.strategy.get_pass_transition_matrix(team_strategy)
        probabilities = transition_matrix[self.state.position.value]
        end_position = random.choices(list(PitchPosition), probabilities)[0]
        distance = (
            end_position.value - self.state.position.value
        )  # distance from current position to end position
        attacking_player = attacking_team.get_player_on_pitch(self.state.position)
        defending_player = defending_team.get_player_on_pitch(self.state.position)
        outcomes = [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_OFFSIDE,
            EventOutcome.PASS_SUCCESS,
            EventOutcome.PASS_INTERCEPT,
        ]

        luck_factor = random.random()

        outcome_probability = [
            int(
                (attacking_player.attributes.intelligence.passing * luck_factor * 10)
                / abs(distance)
            ),
            0,
            int(attacking_player.attributes.intelligence.passing * luck_factor * 10),
            int(defending_player.attributes.defensive.interception * luck_factor * 10),
        ]

        if end_position in OFF_POSITIONS:
            outcome_probability[1] = int(
                (attacking_player.attributes.intelligence.passing * luck_factor * 10)
                / abs(distance)
            )

        self.outcome = random.choices(outcomes, outcome_probability)[0]
        if self.outcome in [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_INTERCEPT,
            EventOutcome.PASS_OFFSIDE,
        ]:
            attacking_team.in_possession = False
            defending_team.in_possession = True
            end_position = PITCH_EQUIVALENTS[end_position]

        return GameState(self.state.minutes, end_position)


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
        transition_matrix = [
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 8, 8, 14],  # OFF_LEFT
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 8, 8, 14],  # OFF_RIGHT
        ]
        print(f"Corner {self.state.position}")
        return GameState(self.state.minutes + 0.1, self.state.position)


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
            EventOutcome.SHOT_SAVED,
            EventOutcome.SHOT_BLOCKED,
            EventOutcome.GOAL,
            EventOutcome.OWN_GOAL,
        ]
        player_shot = attacking_team.get_player_on_pitch(self.state.position)
        print(f"{player_shot.player.details.short_name} shoots!")
        event_probabilities = []
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
        print(f"Goal Kick {self.state.position.name}")
        return GameState(self.state.minutes + 0.1, self.state.position)


@dataclass
class FreeKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"FreeKick {self.state.position.name}")
        return GameState(self.state.minutes + 0.1, self.state.position)


@dataclass
class DribbleEvent(SimulationEvent):
    def calculate_event(
        self, attacking_team: TeamSimulation, defending_team: TeamSimulation
    ) -> GameState:
        print(f"Dribble {self.state.position.name}")
        return GameState(self.state.minutes + 0.1, self.state.position)
