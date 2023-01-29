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
import uuid
from unittest.mock import Mock
from ofm.core.db.generators import PlayerGenerator
from ofm.core.db.database import DB, DatabaseLoadError, PlayerTeamLoadError
from ofm.core.settings import Settings


@pytest.fixture
def db(tmp_path) -> DB:
    settings_file = tmp_path / "settings.yaml"
    settings = Settings(tmp_path, settings_file)
    res = tmp_path / "res"
    res.mkdir()
    db = res / "db"
    db.mkdir()
    settings.res = res
    settings.db = db
    return DB(settings)


def get_squads_def() -> list[dict]:
    return [
        {
            "id": 1,
            "nationalities": [
                {
                    "name": "Germany",
                    "probability": 0.90,
                },
                {
                    "name": "France",
                    "probability": 0.05,
                },
                {
                    "name": "Spain",
                    "probability": 0.05,
                }
            ],
            "mu": 80,
            "sigma": 20,
        },
        {
            "id": 2,
            "nationalities": [
                {
                    "name": "Spain",
                    "probability": 0.90
                },
                {
                    "name": "Germany",
                    "probability": 0.05
                },
                {
                    "name": "France",
                    "probability": 0.05
                }
            ],
            "mu": 80,
            "sigma": 20,
        }
    ]


def get_club_mock_file() -> list[dict]:
    return [
        {
            "id": 1,
            "name": "Munchen",
            "stadium_name": "Munchen National Stadium",
            "stadium_capacity": 40100,
        },
        {
            "id": 2,
            "name": "Barcelona",
            "stadium_name": "Barcelona National Stadium",
            "stadium_capacity": 50000,
        },
    ]


def test_generate_players(db: DB):
    db.generate_players()
    file_contents = db.load_players()
    assert file_contents is not None


def test_get_non_existant_player_from_database(db: DB):
    with pytest.raises(DatabaseLoadError):
        db.get_player_object_from_id(uuid.uuid4(), [{"id": 3333333333333333333333333333333333333}])


def test_get_player_from_empty_players_list(db: DB):
    with pytest.raises(DatabaseLoadError):
        db.get_player_object_from_id(uuid.uuid4(), [])


def test_get_player_from_player_list(db: DB):
    player = PlayerGenerator().generate_player()
    player_dict = player.serialize()
    pl_id = player.player_id
    assert db.get_player_object_from_id(pl_id, [player_dict]) == player


def test_load_player_from_dict(db: DB):
    player = PlayerGenerator().generate_player()
    player_dict = player.serialize()
    assert db.load_player_objects([player_dict]) == [player]


def test_raises_database_load_error_get_player_team_from_dict(db: DB):
    players = [Mock(player_id=uuid.uuid4())]
    squad_ids = [{"id": 1}]
    with pytest.raises(DatabaseLoadError):
        db.get_player_team_from_dicts(squad_ids, players)


def test_raises_error_get_player_team_from_dict(db: DB):
    players = []
    squad_ids = [{"id": 1}]
    with pytest.raises(PlayerTeamLoadError):
        db.get_player_team_from_dicts(squad_ids, players)


# def test_generate_teams(db: DB):
#     open_mock = mock_open()
#     with patch("db.generate_teams", open_mock, create=True):
#
#     db.generate_teams(get_club_mock_file(), get_squads_def())
#     players_list = db.load_players()
#     players = db.load_player_objects(players_list)
