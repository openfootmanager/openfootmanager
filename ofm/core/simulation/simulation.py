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
from ..football.club import TeamSimulation
from .fixture import Fixture


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
        self.possible_extra_time = possible_extra_time
        self.possible_penalties = possible_penalties

    def run(self):
        pass
