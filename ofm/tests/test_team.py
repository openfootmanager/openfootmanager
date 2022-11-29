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
import pytest
import uuid
from ..core.common.team import Team, TeamStats, TeamSimulation, PlayerTeam
from ..core.db.generators import TeamGenerator, PlayerGenerator


def get_squad_ids():
    return [uuid.uuid4().int for _ in range(11)]


@pytest.fixture
def team_gen():
    return TeamGenerator()


@pytest.fixture
def player_gen():
    return PlayerGenerator()


@pytest.fixture
def teams_file(tmp_path):
    d = tmp_path / "db"
    d.mkdir()
    return d / "teams.json"


def test_get_team_from_dictionary():
    team_id = uuid.uuid4()
    name = "MyClub FC"
    squad = get_squad_ids()
    stadium = "MyStadium"
