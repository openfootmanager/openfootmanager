#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2022  Pedrenrique G. Guimarães
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
import uuid

import pytest

from ofm.core.obj.team import Team
from ofm.core.game.match import Match
from ofm.core.game.simulation import MatchSimulation


@pytest.fixture
def team1():
    return Team()


@pytest.fixture
def team2():
    return Team()


@pytest.fixture
def match(team1, team2):
    return Match(uuid.uuid4(), uuid.uuid4(), team1, team2)


@pytest.fixture
def simulation(match, team1, team2):
    return MatchSimulation(match)


def test_match_simulation(simulation):
    pass


def test_event_handler(simulation):
    pass


def test_get_event():
    pass
