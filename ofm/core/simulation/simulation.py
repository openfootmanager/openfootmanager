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
import time
from datetime import timedelta
from enum import Enum
from typing import Optional

from ..football.team_simulation import TeamSimulation
from . import PitchPosition
from .event import SimulationEvent
from .events import EventFactory
from .fixture import Fixture
from .game_state import GameState, SimulationStatus


class DelayValue(Enum):
    NONE = 0
    SHORT = 0.005
    MEDIUM = 0.01
    LONG = 0.1
    VERY_LONG = 1


class LiveGame:
    def __init__(
        self,
        fixture: Fixture,
        home_team: TeamSimulation,
        away_team: TeamSimulation,
        possible_extra_time: bool,
        possible_penalties: bool,
        no_break: bool,
        delay: DelayValue = DelayValue.NONE,
        max_substitutions: int = 5,
    ):
        self.fixture = fixture
        self.is_game_over = False
        self.possible_penalties = possible_penalties
        self._possible_extra_time = possible_extra_time
        self.no_break = no_break
        self.delay = delay
        self.penalty_shootout = False
        self.attendance = self.calculate_attendance()
        self.engine = SimulationEngine(home_team, away_team, max_substitutions)
        self.added_time: Optional[timedelta] = None
        self.total_elapsed_time: timedelta = timedelta(0)

    @property
    def possible_extra_time(self):
        return self._possible_extra_time

    @possible_extra_time.setter
    def possible_extra_time(self, value: bool):
        self._possible_extra_time = value
        if not self.possible_penalties:
            self.possible_penalties = True

    @property
    def minutes(self):
        return self.engine.state.minutes

    @minutes.setter
    def minutes(self, minutes):
        self.engine.state.minutes = minutes

    @property
    def state(self):
        return self.engine.state

    @state.setter
    def state(self, state: GameState):
        self.engine.state = state

    def get_added_time(self):
        if (
            (
                self.state.status == SimulationStatus.FIRST_HALF
                and self.state.in_additional_time is False
                and self.state.minutes == timedelta(minutes=45)
            )
            or (
                self.state.status == SimulationStatus.SECOND_HALF
                and self.state.in_additional_time is False
                and self.state.minutes == timedelta(minutes=90)
            )
            or (
                self.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME
                and self.state.in_additional_time is False
                and self.state.minutes == timedelta(minutes=105)
            )
            or (
                self.state.status == SimulationStatus.SECOND_HALF_EXTRA_TIME
                and self.state.in_additional_time is False
                and self.state.minutes == timedelta(minutes=120)
            )
        ):
            added_time = random.randint(0, 5)
            self.state.in_additional_time = True
            self.added_time = timedelta(minutes=float(added_time))

    def calculate_attendance(self) -> int:
        pass

    def reset_state_additional_time(self):
        self.state.in_additional_time = False
        self.state.additional_time_elapsed = timedelta(0)

    def transition_game_status(self):
        if self.state.status == SimulationStatus.NOT_STARTED:
            self.state.status = SimulationStatus.FIRST_HALF
        elif (
            self.state.status == SimulationStatus.FIRST_HALF
            and self.state.minutes == timedelta(minutes=45)
        ):
            if not self.state.in_additional_time:
                self.get_added_time()
            elif (
                self.state.additional_time_elapsed.total_seconds()
                >= self.added_time.total_seconds()
            ):
                self.state.status = SimulationStatus.FIRST_HALF_BREAK
                self.reset_state_additional_time()
        elif self.state.status == SimulationStatus.FIRST_HALF_BREAK:
            self.state.status = SimulationStatus.SECOND_HALF
        elif (
            self.state.status == SimulationStatus.SECOND_HALF
            and self.state.minutes == timedelta(minutes=90)
        ):
            if not self.state.in_additional_time:
                self.get_added_time()
            elif (
                self.state.additional_time_elapsed.total_seconds()
                >= self.added_time.total_seconds()
            ):
                if self.possible_extra_time and self.engine.is_game_a_draw():
                    self.state.status = SimulationStatus.SECOND_HALF_BREAK
                    self.reset_state_additional_time()
                else:
                    self.state.status = SimulationStatus.FINISHED
                    self.is_game_over = True
        elif self.state.status == SimulationStatus.SECOND_HALF_BREAK:
            self.state.status = SimulationStatus.FIRST_HALF_EXTRA_TIME
        elif (
            self.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME
            and self.state.minutes == timedelta(minutes=105)
        ):
            if not self.state.in_additional_time:
                self.get_added_time()
            elif (
                self.state.additional_time_elapsed.total_seconds()
                >= self.added_time.total_seconds()
            ):
                self.state.status = SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK
                self.reset_state_additional_time()
        elif self.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK:
            self.state.status = SimulationStatus.SECOND_HALF_EXTRA_TIME
        elif (
            self.state.status == SimulationStatus.SECOND_HALF_EXTRA_TIME
            and self.state.minutes == timedelta(minutes=120)
        ):
            if not self.state.in_additional_time:
                self.get_added_time()
            elif (
                self.state.additional_time_elapsed.total_seconds()
                >= self.added_time.total_seconds()
            ):
                if self.engine.is_game_a_draw():
                    self.state.status = SimulationStatus.SECOND_HALF_EXTRA_TIME_BREAK
                else:
                    self.state.status = SimulationStatus.FINISHED
                    self.is_game_over = True
        elif self.state.status == SimulationStatus.SECOND_HALF_EXTRA_TIME_BREAK:
            self.state.status = SimulationStatus.PENALTY_SHOOTOUT

    def add_minutes(self):
        duration = self.engine.get_event_duration()
        self.state.minutes += duration
        self.total_elapsed_time += duration

        if (
            self.state.minutes >= timedelta(minutes=45)
            and self.state.status == SimulationStatus.FIRST_HALF
        ):
            additional_time = self.state.minutes - timedelta(minutes=45)
            self.state.additional_time_elapsed += additional_time
            self.state.minutes = timedelta(minutes=45)
        elif (
            self.state.minutes >= timedelta(minutes=90)
            and self.state.status == SimulationStatus.SECOND_HALF
        ):
            additional_time = self.state.minutes - timedelta(minutes=90)
            self.state.additional_time_elapsed += additional_time
            self.state.minutes = timedelta(minutes=90)
        elif (
            self.state.minutes >= timedelta(minutes=105)
            and self.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME
        ):
            additional_time = self.state.minutes - timedelta(minutes=105)
            self.state.additional_time_elapsed += additional_time
            self.state.minutes = timedelta(minutes=105)
        elif (
            self.state.minutes >= timedelta(minutes=120)
            and self.state.status == SimulationStatus.SECOND_HALF_EXTRA_TIME
        ):
            additional_time = self.state.minutes - timedelta(minutes=120)
            self.state.additional_time_elapsed += additional_time
            self.state.minutes = timedelta(minutes=120)

    def is_game_on_break(self) -> bool:
        return (
            self.state.status == SimulationStatus.FIRST_HALF_BREAK
            or self.state.status == SimulationStatus.SECOND_HALF_BREAK
            or self.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK
            or self.state.status == SimulationStatus.SECOND_HALF_EXTRA_TIME_BREAK
        )

    def run(self):
        while not self.is_game_over:
            self.engine.run()
            self.add_minutes()
            self.transition_game_status()
            if not self.no_break and self.is_game_on_break():
                break
            if self.delay.value > 0:
                time.sleep(self.delay.value)


class SimulationEngine:
    def __init__(
        self,
        home_team: TeamSimulation,
        away_team: TeamSimulation,
        max_substitutions: int,
    ):
        self.home_team = home_team
        self.away_team = away_team
        self.home_team.max_substitutions = max_substitutions
        self.away_team.max_substitutions = max_substitutions
        self.event_history: list[SimulationEvent] = []
        self.state = GameState(
            timedelta(seconds=0),
            SimulationStatus.NOT_STARTED,
            PitchPosition.MIDFIELD_CENTER,
        )
        self.starting_the_game = random.choice([self.home_team, self.away_team])
        if self.starting_the_game == self.home_team:
            self.secondary_start = self.away_team
        else:
            self.secondary_start = self.home_team
        self.event_factory = EventFactory()

    def generate_event(self) -> SimulationEvent:
        last_event = self.event_history[-1] if self.event_history else None
        event_type = self.event_factory.get_event_type(
            self.get_team_in_possession(), self.state, last_event
        )
        event = self.event_factory.get_event(self.state, event_type)
        self.event_history.append(event)
        return event

    def is_game_a_draw(self) -> bool:
        return self.home_team.score == self.away_team.score

    def get_team_in_possession(self) -> tuple[TeamSimulation, TeamSimulation]:
        """
        Returns (Attacking Team, Defending Team)
        """
        if self.state.status in [
            SimulationStatus.NOT_STARTED,
            SimulationStatus.SECOND_HALF_BREAK,
        ]:
            self.starting_the_game.in_possession = True
            self.secondary_start.in_possession = False
            self.starting_the_game.player_in_possession = (
                self.starting_the_game.get_player_on_pitch(
                    PitchPosition.MIDFIELD_CENTER
                )
            )
        elif self.state.status in [
            SimulationStatus.FIRST_HALF_BREAK,
            SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK,
        ]:
            self.starting_the_game.in_possession = False
            self.secondary_start.in_possession = True
            self.secondary_start.player_in_possession = (
                self.secondary_start.get_player_on_pitch(PitchPosition.MIDFIELD_CENTER)
            )

        if self.home_team.in_possession:
            return self.home_team, self.away_team
        else:
            return self.away_team, self.home_team

    def get_event_duration(self):
        if self.event_history:
            return timedelta(seconds=self.event_history[-1].duration)

    def run(self):
        event = self.generate_event()
        attacking_team, defending_team = self.get_team_in_possession()
        self.state = event.calculate_event(attacking_team, defending_team)
        attacking_team.stats.possession += event.duration

        self.home_team.update_player_stamina(event.duration)
        self.away_team.update_player_stamina(event.duration)
