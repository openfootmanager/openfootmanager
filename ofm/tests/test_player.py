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
import datetime
import uuid

import pytest

from ..core.db.generators import PlayerGenerator
from ..core.football.player import (
    Player,
    PlayerAttributes,
    PlayerTeam,
    Positions,
    PreferredFoot,
    get_player_from_player_id,
)
from ..core.football.playercontract import PlayerContract


@pytest.fixture
def player_gen() -> PlayerGenerator:
    return PlayerGenerator()


def get_player_obj(player_id: uuid.UUID) -> Player:
    return Player(
        player_id,
        "Brazil",
        datetime.date(1996, 12, 14),
        "John",
        "Doe",
        "J. Doe",
        [Positions.FW, Positions.MF],
        100.0,
        100.0,
        0.5,
        PlayerAttributes(80, 75, 30, 20),
        PlayerAttributes(90, 75, 30, 20),
        5,
        PreferredFoot.LEFT,
        10000.00,
    )


def get_player_dict(player_id: uuid.UUID) -> dict:
    positions = [Positions.FW, Positions.MF]
    preferred_foot = PreferredFoot.LEFT
    return {
        "id": player_id.int,
        "nationality": "Brazil",
        "dob": "1996-12-14",
        "first_name": "John",
        "last_name": "Doe",
        "short_name": "J. Doe",
        "positions": [position.value for position in positions],
        "fitness": 100.0,
        "stamina": 100.0,
        "form": 0.5,
        "attributes": {
            "offense": 80,
            "defense": 75,
            "passing": 30,
            "gk": 20,
        },
        "potential_attributes": {
            "offense": 90,
            "defense": 75,
            "passing": 30,
            "gk": 20,
        },
        "international_reputation": 5,
        "preferred_foot": PreferredFoot(preferred_foot),
        "value": 10000.00,
    }


def get_player_team() -> tuple[PlayerTeam, Player, dict]:
    player_id = uuid.uuid4()
    team_id = uuid.uuid4()
    shirt_number = 10
    contract_dict = {
        "wage": 10000.00,
        "started": "2020-01-01",
        "end": "2021-01-01",
        "bonus_for_goal": 500.00,
        "bonus_for_def": 500.00,
    }
    player = get_player_obj(player_id)
    player_team_dict = {
        "player_id": player_id.int,
        "team_id": team_id.int,
        "shirt_number": shirt_number,
        "contract": contract_dict,
    }
    expected_contract = PlayerContract.get_from_dict(contract_dict)
    player_team = PlayerTeam(player, team_id, shirt_number, expected_contract)
    return player_team, player, player_team_dict


def test_get_from_dictionary():
    player_id = uuid.uuid4()
    player_dict = get_player_dict(player_id)
    expected_player = get_player_obj(player_id)
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


def test_player_team_get_from_dictionary():
    expected_player_team, player, player_team_dict = get_player_team()
    assert PlayerTeam.get_from_dict(player_team_dict, [player]) == expected_player_team


def test_serialize_player_team():
    player_team, player, expected_player_team_dict = get_player_team()
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
