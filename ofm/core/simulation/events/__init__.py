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
from copy import deepcopy
from typing import Optional

from ...football.team_simulation import TeamSimulation
from .. import PitchPosition
from ..event import EventOutcome, SimulationEvent
from ..event_type import EventType, FoulType
from ..game_state import GameState, SimulationStatus
from ..team_strategy import team_general_strategy
from .corner_kick_event import CornerKickEvent
from .cross_event import CrossEvent
from .dribble_event import DribbleEvent
from .foul_event import FoulEvent
from .free_kick_event import FreeKickEvent
from .goal_kick_event import GoalKickEvent
from .pass_event import PassEvent
from .penalty_kick_event import PenaltyKickEvent
from .shot_event import ShotEvent


class EventFactory:
    def get_event_type(
        self,
        teams: tuple[TeamSimulation, TeamSimulation],
        state: GameState,
        last_event: Optional[SimulationEvent],
    ) -> EventType:
        if state.status in [
            SimulationStatus.NOT_STARTED,
            SimulationStatus.FIRST_HALF_BREAK,
            SimulationStatus.SECOND_HALF_BREAK,
            SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK,
        ]:
            return EventType.PASS

        if last_event.outcome == EventOutcome.GOAL:
            return EventType.PASS
        elif last_event.outcome == EventOutcome.SHOT_GOAL_KICK:
            return EventType.GOAL_KICK
        elif last_event.outcome in [
            EventOutcome.SHOT_LEFT_CORNER_KICK,
            EventOutcome.SHOT_RIGHT_CORNER_KICK,
        ]:
            return EventType.CORNER_KICK
        elif isinstance(last_event, FoulEvent):
            if (
                last_event.foul_type == FoulType.DEFENSIVE_FOUL
                and state.position == PitchPosition.OFF_BOX
            ):
                return EventType.PENALTY_KICK
            else:
                return EventType.FREE_KICK

        attacking_team, defensive_team = teams
        transition_matrix = team_general_strategy(
            attacking_team.team_strategy, defensive_team.team_strategy, state
        )

        events = [
            EventType.PASS,
            EventType.CROSS,
            EventType.DRIBBLE,
            EventType.FOUL,
            EventType.SHOT,
        ]
        return random.choices(events, transition_matrix)[0]

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
            return PenaltyKickEvent(EventType.PENALTY_KICK, state)

        return NotImplemented
