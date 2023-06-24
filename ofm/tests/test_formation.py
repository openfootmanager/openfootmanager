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
from ofm.core.football.player import Positions
from ofm.core.football.formation import Formation, FormationError
from ofm.core.db.generators import TeamGenerator


def test_invalid_formation():
    with pytest.raises(FormationError):
        Formation("4-4-3")


def test_add_gk_to_formation(player_team):
    formation = Formation("4-4-2")
    formation.add_player(0, player_team[0])
    assert formation.gk.player == player_team[0]
    assert formation.gk.current_position == Positions.GK


def test_add_df_to_formation(player_team):
    formation = Formation("4-4-2")
    for i in range(4):
        formation.add_player(i + 1, player_team[0])
        assert formation.df[i].player == player_team[0]
        assert formation.df[i].current_position == Positions.DF


def test_add_mf_to_formation(player_team):
    formation = Formation("4-4-2")
    for i in range(4):
        formation.add_player(i + 5, player_team[0])
        assert formation.mf[i].player == player_team[0]
        assert formation.mf[i].current_position == Positions.MF


def test_add_fw_to_formation(player_team):
    formation = Formation("4-4-2")
    for i in range(2):
        formation.add_player(i + 9, player_team[0])
        assert formation.fw[i].player == player_team[0]
        assert formation.fw[i].current_position == Positions.FW


def test_add_players_to_formation(player_team):
    formation = Formation("4-4-2")
    for i in range(11):
        formation.add_player(i, player_team[0])

    assert len(formation.df) == 4
    assert len(formation.mf) == 4
    assert len(formation.fw) == 2


def test_add_players_to_formation_and_bench(player_team):
    formation = Formation("4-4-2")
    for i in range(16):
        formation.add_player(i, player_team[0])

    assert len(formation.df) == 4
    assert len(formation.mf) == 4
    assert len(formation.fw) == 2
    assert len(formation.bench) == 5


def test_formation_get_best_players(squads_def, confederations_file):
    team_gen = TeamGenerator(squads_def, confederations_file)
    clubs = team_gen.generate()

    formation = Formation("4-4-2")
    formation.get_best_players(clubs[0].squad)

    assert len(formation.players) == 11
    assert len(formation.bench) > 0
    assert len(formation.df) == 4
    assert len(formation.mf) == 4
    assert len(formation.fw) == 2
