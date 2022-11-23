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
import json
import uuid
import datetime
from ..core.common.player import Player, PlayerTeam, Positions, PreferredFoot, get_player_from_player_id
from ..core.common.playercontract import PlayerContract
from ..core.db.generators import PlayerGenerator


@pytest.fixture
def player_gen():
    return PlayerGenerator()


@pytest.fixture
def players_file(tmp_path):
    d = tmp_path / "db"
    d.mkdir()
    return d / "players.json"


def test_get_from_dictionary():
    player_id = uuid.uuid4()
    nationality = "Brazil"
    dob = "1996-12-14"
    first_name = "John"
    last_name = "Doe"
    short_name = "J. Doe"
    positions = [Positions.FW, Positions.ST]
    fitness = 100.0
    stamina = 100.0
    form = 0.5
    skill = 80
    potential_skill = 90
    international_reputation = 5
    preferred_foot = PreferredFoot.LEFT
    value = 10000.00
    player_dict = {
        "id": player_id.int,
        "nationality": nationality,
        "dob": dob,
        "first_name": first_name,
        "last_name": last_name,
        "short_name": short_name,
        "positions": [position.value for position in positions],
        "fitness": fitness,
        "stamina": stamina,
        "form": form,
        "skill": skill,
        "potential_skill": potential_skill,
        "international_reputation": international_reputation,
        "preferred_foot": PreferredFoot(preferred_foot),
        "value": value
    }
    expected_player = Player(
        player_id,
        nationality,
        datetime.datetime.strptime(dob, "%Y-%m-%d").date(),
        first_name,
        last_name,
        short_name,
        positions,
        fitness,
        stamina,
        form,
        skill,
        potential_skill,
        international_reputation,
        preferred_foot,
        value
    )
    player = Player.get_from_dict(player_dict)
    assert expected_player == player


def test_player_expected_keys_dictionary(player_gen):
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
        "skill",
        "potential_skill",
        "international_reputation",
        "preferred_foot",
        "value"
    )
    player = player_gen.generate_player()
    player_dict = player.serialize()
    assert all(k in player_dict for k in expected_keys)


def test_player_team_get_from_dictionary():
    player_id = uuid.uuid4()
    team_id = uuid.uuid4()
    shirt_number = 10
    nationality = "Brazil"
    dob = "1996-12-14"
    first_name = "John"
    last_name = "Doe"
    short_name = "J. Doe"
    positions = [Positions.FW, Positions.ST]
    fitness = 100.0
    stamina = 100.0
    form = 0.5
    skill = 80
    potential_skill = 90
    international_reputation = 5
    preferred_foot = PreferredFoot.LEFT
    value = 10000.00
    contract_dict = {
        "wage": 10000.00,
        "started": "2020-01-01",
        "end": "2021-01-01",
        "bonus_for_goal": 500.00,
        "bonus_for_def": 500.00,
    }
    player = Player(
        player_id,
        nationality,
        datetime.datetime.strptime(dob, "%Y-%m-%d").date(),
        first_name,
        last_name,
        short_name,
        positions,
        fitness,
        stamina,
        form,
        skill,
        potential_skill,
        international_reputation,
        preferred_foot,
        value
    )
    player_team_dict = {
        "player_id": player_id.int,
        "team_id": team_id.int,
        "shirt_number": shirt_number,
        "contract": contract_dict,
    }
    expected_contract = PlayerContract.get_from_dict(contract_dict)
    expected_player_team = PlayerTeam(
        player,
        team_id,
        shirt_number,
        expected_contract
    )
    assert PlayerTeam.get_from_dict(player_team_dict, [player]) == expected_player_team


def test_serialize_player_team():
    player_id = uuid.uuid4()
    team_id = uuid.uuid4()
    shirt_number = 10
    nationality = "Brazil"
    dob = "1996-12-14"
    first_name = "John"
    last_name = "Doe"
    short_name = "J. Doe"
    positions = [Positions.FW, Positions.ST]
    fitness = 100.0
    stamina = 100.0
    form = 0.5
    skill = 80
    potential_skill = 90
    international_reputation = 5
    preferred_foot = PreferredFoot.LEFT
    value = 10000.00
    contract_dict = {
        "wage": 10000.00,
        "started": "2020-01-01",
        "end": "2021-01-01",
        "bonus_for_goal": 500.00,
        "bonus_for_def": 500.00,
    }
    player = Player(
        player_id,
        nationality,
        datetime.datetime.strptime(dob, "%Y-%m-%d").date(),
        first_name,
        last_name,
        short_name,
        positions,
        fitness,
        stamina,
        form,
        skill,
        potential_skill,
        international_reputation,
        preferred_foot,
        value
    )
    expected_player_team_dict = {
        "player_id": player_id.int,
        "team_id": team_id.int,
        "shirt_number": shirt_number,
        "contract": contract_dict,
    }
    expected_contract = PlayerContract.get_from_dict(contract_dict)
    player_team = PlayerTeam(
        player,
        team_id,
        shirt_number,
        expected_contract
    )
    assert player_team.serialize() == expected_player_team_dict


def test_get_player_from_player_id(player_gen):
    player_gen.generate(100)
    players = player_gen.players_obj.copy()
    player = players[0]

    assert get_player_from_player_id(player.player_id, players) == player


def test_write_to_db(player_gen, players_file):
    player_gen.generate(1000)
    expected_players_dict = player_gen.get_players_dictionaries()
    player_gen.write_to_db(players_file)
    with open(players_file, "r") as fp:
        players_dict = json.load(fp)

    assert expected_players_dict == players_dict
