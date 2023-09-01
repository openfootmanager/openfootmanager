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
import json
import uuid

import pytest

from ..core.db.generators import PlayerGenerator, TeamGenerator
from ..core.football.club import PlayerTeam
from ..core.football.formation import Formation
from ..core.football.player import (Player, PlayerInjury, PlayerSimulation,
                                    Positions, PreferredFoot)
from ..core.football.player_attributes import *
from ..core.football.playercontract import PlayerContract
from ..core.football.team_simulation import TeamSimulation
from ..core.settings import Settings


@pytest.fixture
def player_gen() -> PlayerGenerator:
    return PlayerGenerator()


@pytest.fixture
def player_attributes() -> PlayerAttributes:
    return PlayerAttributes(
        OffensiveAttributes(85, 80, 75, 88, 90),
        PhysicalAttributes(80, 75, 40),
        DefensiveAttributes(54, 65, 50),
        IntelligenceAttributes(60, 88, 82, 87, 80, 83, 75),
        GkAttributes(20, 20, 10, 10),
    )


@pytest.fixture
def player_obj(player_attributes: PlayerAttributes) -> Player:
    return Player(
        uuid.UUID(int=1),
        "Brazil",
        datetime.date(1996, 12, 14),
        "John",
        "Doe",
        "J. Doe",
        [Positions.FW, Positions.MF],
        100.0,
        100.0,
        0.5,
        player_attributes,
        90,
        5,
        PreferredFoot.LEFT,
        10000.00,
    )


@pytest.fixture
def player_dict(player_attributes: PlayerAttributes) -> dict:
    positions = [Positions.FW, Positions.MF]
    preferred_foot = PreferredFoot.LEFT
    return {
        "id": 1,
        "nationality": "Brazil",
        "dob": "1996-12-14",
        "first_name": "John",
        "last_name": "Doe",
        "short_name": "J. Doe",
        "positions": [position.value for position in positions],
        "fitness": 100.0,
        "stamina": 100.0,
        "form": 0.5,
        "attributes": player_attributes.serialize(),
        "potential_skill": 90,
        "international_reputation": 5,
        "preferred_foot": PreferredFoot(preferred_foot),
        "value": 10000.00,
        "injured": False,
        "injury_type": PlayerInjury.NO_INJURY,
    }


@pytest.fixture
def player_team(player_obj) -> tuple[PlayerTeam, Player, dict]:
    player_id = uuid.UUID(int=1)
    team_id = uuid.UUID(int=1)
    shirt_number = 10
    contract_dict = {
        "wage": 10000.00,
        "started": "2020-01-01",
        "end": "2021-01-01",
        "bonus_for_goal": 500.00,
        "bonus_for_def": 500.00,
    }
    player_team_dict = {
        "player_id": player_id.int,
        "team_id": team_id.int,
        "shirt_number": shirt_number,
        "contract": contract_dict,
    }
    expected_contract = PlayerContract.get_from_dict(contract_dict)
    player_team = PlayerTeam(player_obj, team_id, shirt_number, expected_contract)
    return player_team, player_obj, player_team_dict


@pytest.fixture
def player_sim(player_team: tuple[PlayerTeam, Player, dict]) -> PlayerSimulation:
    position = player_team[0].details.get_best_position()
    return PlayerSimulation(player_team[0], position, 100.0)


@pytest.fixture
def squads_def() -> list[dict]:
    return [
        {
            "name": "Munchen",
            "stadium": "Munchen National Stadium",
            "stadium_capacity": 40100,
            "country": "GER",
            "location": "Munich",
            "default_formation": "4-4-2",
            "squads_def": {
                "mu": 80,
                "sigma": 20,
            },
        },
        {
            "name": "Barcelona",
            "stadium": "Barcelona National Stadium",
            "stadium_capacity": 50000,
            "country": "ESP",
            "location": "Barcelona",
            "default_formation": "4-3-3",
            "squads_def": {
                "mu": 80,
                "sigma": 20,
            },
        },
    ]


@pytest.fixture
def mock_file() -> list[dict]:
    return [
        {
            "id": 1,
            "name": "Munchen",
            "country": "GER",
            "location": "Munich",
            "default_formation": "4-4-2",
            "squad": [],
            "stadium": "Munchen National Stadium",
            "stadium_capacity": 40100,
        },
        {
            "id": 2,
            "name": "Barcelona",
            "country": "ESP",
            "location": "Barcelona",
            "default_formation": "4-3-3",
            "squad": [],
            "stadium": "Barcelona National Stadium",
            "stadium_capacity": 50000,
        },
    ]


@pytest.fixture
def confederations_file() -> list[dict]:
    settings = Settings()
    with open(settings.fifa_conf, "r") as fp:
        return json.load(fp)


@pytest.fixture
def simulation_teams(squads_def, confederations_file) -> tuple[TeamSimulation, TeamSimulation]:
    team_gen = TeamGenerator(squads_def, confederations_file)

    teams = team_gen.generate()
    home_team, away_team = teams[0], teams[1]

    home_team_formation = Formation(home_team.default_formation)
    home_team_formation.get_best_players(home_team.squad)
    home_team_sim = TeamSimulation(home_team, home_team_formation)
    away_team_formation = Formation(away_team.default_formation)
    away_team_formation.get_best_players(away_team.squad)
    away_team_sim = TeamSimulation(away_team, away_team_formation)

    return home_team_sim, away_team_sim
