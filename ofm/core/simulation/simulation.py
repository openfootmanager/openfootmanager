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
from decimal import Decimal, getcontext

from .fixture import Fixture
from ..football.club import TeamSimulation
from . import PitchPosition
from .event import SimulationEvent, GameState, EventFactory


class LiveGame:
    def __init__(
        self,
        fixture: Fixture,
        home_team: TeamSimulation,
        away_team: TeamSimulation,
        possible_extra_time: bool,
        possible_penalties: bool,
    ):
        self.minutes = Decimal(0.0)
        self.fixture = fixture
        self.is_half_time = False
        self.is_game_over = False
        self.possible_penalties = possible_penalties
        self.possible_extra_time = possible_extra_time
        self.penalty_shootout = False
        self.attendance = self.calculate_attendance()
        self.engine = SimulationEngine(home_team, away_team)

    def calculate_attendance(self) -> int:
        pass

    def reset_after_half_time(self):
        if not self.penalty_shootout:
            self.is_half_time = False
            self.minutes += Decimal(0.1)

    def game_is_not_in_break(self) -> bool:
        if self.minutes == 120.0:
            if self.possible_penalties and self.engine.is_game_a_draw():
                self.is_half_time = True
                self.penalty_shootout = True
            else:
                self.is_game_over = True
            return False
        elif self.minutes in [45.0, 105.0]:
            self.is_half_time = True
            return False
        elif self.minutes == 90.0:
            if self.possible_extra_time and self.engine.is_game_a_draw():
                self.is_half_time = True
            else:
                self.is_game_over = True
            return False
        return True

    def run(self):
        while not self.is_game_over and not self.is_half_time:
            if self.game_is_not_in_break():
                self.engine.run()
                self.minutes += Decimal(0.1)


class SimulationEngine:
    def __init__(
        self,
        home_team: TeamSimulation,
        away_team: TeamSimulation,
    ):
        getcontext().prec = 5
        self.home_team = home_team
        self.away_team = away_team
        self.event_history = []
        self.state = GameState(0.0, PitchPosition.MIDFIELD_CENTER)
        self.starting_the_game = random.choice([self.home_team, self.away_team])

    def generate_event(self) -> SimulationEvent:
        event_factory = EventFactory()
        last_event = self.event_history[-1].event_type if self.event_history else None
        possible_events, event_probabilities = event_factory.get_possible_events(self.state, last_event)
        event_type = random.choices(possible_events, event_probabilities)[0]
        event = event_factory.get_event(self.state, event_type)
        self.event_history.append(event)
        return event

    def is_game_a_draw(self) -> bool:
        return self.home_team.score == self.away_team.score

    def get_team_in_possession(self) -> tuple[TeamSimulation, TeamSimulation]:
        """
        Returns (Attacking Team, Defending Team)
        """
        if self.state.minutes in [0.0, 90.1]:
            if self.starting_the_game == self.home_team:
                self.away_team.in_possession = False
                self.home_team.in_possession = True
            else:
                self.home_team.in_possession = False
                self.away_team.in_possession = True
        if self.state.minutes in [45.1, 105.1]:
            if self.starting_the_game == self.home_team:
                self.away_team.in_possession = True
                self.home_team.in_possession = False
            else:
                self.home_team.in_possession = True
                self.away_team.in_possession = False

        if self.home_team.in_possession:
            return self.home_team, self.away_team
        else:
            return self.away_team, self.home_team

    def run(self):
        event = self.generate_event()
        attacking_team, defending_team = self.get_team_in_possession()
        print(f"{attacking_team.club.name} has the ball")
        self.state = event.calculate_event(attacking_team, defending_team)
