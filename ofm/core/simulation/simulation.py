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

from .fixture import Fixture
from ..football.club import TeamSimulation
from .event import SimulationEvent, EventType, PitchPosition, Possession


class LiveGame:
    def __init__(
        self,
        fixture: Fixture,
        home_team: TeamSimulation,
        away_team: TeamSimulation,
        possible_extra_time: bool,
        possible_penalties: bool,
    ):
        self.fixture = fixture
        self.home_team = home_team
        self.away_team = away_team
        self.attendance = self.calculate_attendance()
        self.engine = SimulationEngine(possible_penalties, possible_extra_time)

    def calculate_attendance(self) -> int:
        pass

    def run(self):
        while not self.engine.is_game_over and not self.engine.is_half_time:
            event = self.engine.generate_events()
            event.calculate_event(self.home_team, self.away_team)


class SimulationEngine:
    def __init__(
            self,
            possible_penalties: bool,
            possible_extra_time: bool,
    ):
        self.minutes = 0.0
        self.is_half_time = False
        self.is_game_over = False
        self.possible_penalties = possible_penalties
        self.possible_extra_time = possible_extra_time
        self.event_history = []
        self.possession = random.choice(list(Possession))
        self.pitch_position = PitchPosition.MIDFIELD_CENTER

    def generate_events(self) -> SimulationEvent:
        events = list(EventType)
        if not self.event_history:
            event_type = EventType.START_MATCH
        else:
            event_type = EventType.PASS

        return SimulationEvent(event_type, self.minutes, self.possession, self.pitch_position)
