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
import pytest
import datetime

from ..core.db.generators import TeamGenerator
from ..core.football.club import Club, PlayerTeam


def test_generate_team_squads(squads_def, confederations_file):
    team_gen = TeamGenerator(squads_def, confederations_file, datetime.date.today())
    clubs = team_gen.generate()
    for club in clubs:
        assert len(club.squad) >= 11
        assert isinstance(club, Club)
        for player in club.squad:
            assert isinstance(player, PlayerTeam)
