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
import datetime
from ..core.common.player import Player, PlayerSimulation, PlayerStats, PlayerTeam, Positions, PreferredFoot
from ..core.db.generators import GeneratePlayer


@pytest.fixture
def player_gen():
    return GeneratePlayer()


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
        "preferred_foot": preferred_foot.name,
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


def test_write_to_db(player_gen, players_file):
    player_gen.generate(1000)
    player_gen.write_to_db(players_file)
    # TODO: finish this test
