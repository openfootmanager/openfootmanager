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
import uuid


from ..core.db.generators import PlayerGenerator
from ..core.football.player import (
    Player,
    PlayerTeam,
    get_player_from_player_id,
)


def test_get_from_dictionary(player_dict, player_obj):
    expected_player = player_obj
    player = Player.get_from_dict(player_dict)
    assert expected_player == player


def test_player_expected_keys_dictionary(player_gen: PlayerGenerator):
    expected_keys = (
        "id",
        "nationality",
        "dob",
        "first_name",
        "last_name",
        "short_name",
        "positions",
        "fitness",
        "stamina",
        "form",
        "attributes",
        "potential_attributes",
        "international_reputation",
        "preferred_foot",
        "value",
    )
    player = player_gen.generate_player()
    player_dict = player.serialize()
    assert all(k in player_dict for k in expected_keys)


def test_player_team_get_from_dictionary(player_team):
    expected_player_team, player, player_team_dict = player_team
    assert PlayerTeam.get_from_dict(player_team_dict, [player]) == expected_player_team


def test_serialize_player_team(player_team):
    player_team, player, expected_player_team_dict = player_team
    assert player_team.serialize() == expected_player_team_dict


def test_get_player_from_player_id(player_gen: PlayerGenerator):
    player_gen.generate(100)
    players = player_gen.players_obj.copy()
    player = players[0]
    assert get_player_from_player_id(player.player_id, players) == player


def test_serialized_player_equals_to_obj(player_gen: PlayerGenerator):
    player = player_gen.generate_player()
    player_dict = player.serialize()
    assert Player.get_from_dict(player_dict) == player


def test_generate_player_from_unknown_region(player_gen: PlayerGenerator):
    player = player_gen.generate_player(region="Senegal")
    assert player.first_name is not None
    assert player.last_name is not None


def test_serialized_player_has_no_none(player_gen: PlayerGenerator):
    player_gen.generate(10000)
    for player in player_gen.players_obj:
        player_dict = player.serialize()
        for key in player_dict.keys():
            assert player_dict[key] is not None
