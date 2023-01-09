#      Openfoot Manager - A free and open source soccer management game
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
from ..core.common.team import Team, TeamStats, TeamSimulation, PlayerTeam
from ..core.db.generators import TeamGenerator


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
            ]
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
            ]
        }
    ]

def get_team_mock_file() -> dict:
    return {
        "regions": [
            {
                "name": "UEFA",
                "countries": [
                    {
                        "name": "Germany",
                        "teams": [
                            {
                                "id": 1,
                                "name": "Munchen",
                                "stadium_name": "Munchen National Stadium",
                                "stadium_capacity": 40100,
                                "financial_rating": 80,
                            }
                        ]
                    },
                    {
                        "name": "Spain",
                        "teams": [
                            {
                                "id": 2,
                                "name": "Barcelona",
                                "stadium_name": "Barcelona National Stadium",
                                "stadium_capacity": 50000,
                                "financial_rating": 85,
                            },
                        ]
                    }
                ]
            }
        ]
    }


def test_get_team_from_mock_file():
    mock_definition_file = get_team_mock_file()
    expected_teams = [
        Team(
            uuid.UUID(int=1),
            "Munchen",
            "Munchen National Stadium",
            40100,
            80,
        ),
        Team(
            uuid.UUID(int=2),
            "Barcelona",
            "Barcelona National Stadium",
            50000,
            85,
        )
    ]
    teams = []

    for region in mock_definition_file["regions"]:
        for countries in region["countries"]:
            for team in countries["teams"]:
                teams.append(Team.get_from_dict(team))

    assert teams == expected_teams


def test_generate_team_squads():
    pass
