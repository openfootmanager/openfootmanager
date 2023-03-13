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

from ..core.db.generators import TeamGenerator
from ..core.football.club import Club, PlayerTeam


def get_squads_def() -> list[dict]:
    return [
        {
            "name": "Munchen",
            "stadium_name": "Munchen National Stadium",
            "stadium_capacity": 40100,
            "squad_def": {
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
            }
        },
        {
            "name": "Barcelona",
            "stadium_name": "Barcelona National Stadium",
            "stadium_capacity": 50000,
            "squad_def": {
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
        }
    ]


def get_club_mock_file() -> dict:
    return {
        "regions": [
            {
                "name": "UEFA",
                "countries": [
                    {
                        "name": "Germany",
                        "clubs": [
                            {
                                "id": 1,
                                "name": "Munchen",
                                "squad": [],
                                "stadium_name": "Munchen National Stadium",
                                "stadium_capacity": 40100,
                            }
                        ]
                    },
                    {
                        "name": "Spain",
                        "clubs": [
                            {
                                "id": 2,
                                "name": "Barcelona",
                                "squad": [],
                                "stadium_name": "Barcelona National Stadium",
                                "stadium_capacity": 50000,
                            },
                        ]
                    }
                ]
            }
        ]
    }


def test_get_club_from_mock_file():
    mock_definition_file = get_club_mock_file()
    expected_teams = [
        Club(
            uuid.UUID(int=1),
            "Munchen",
            [],
            "Munchen National Stadium",
            40100,
        ),
        Club(
            uuid.UUID(int=2),
            "Barcelona",
            [],
            "Barcelona National Stadium",
            50000,
        )
    ]
    teams = []

    for region in mock_definition_file["regions"]:
        for countries in region["countries"]:
            teams.extend(Club.get_from_dict(team, []) for team in countries["clubs"])
    assert teams == expected_teams


def test_generate_team_squads():
    clubs_def = get_squads_def()
    team_gen = TeamGenerator(clubs_def, datetime.date.today())
    team_squads = team_gen.generate()
    for t_squad in team_squads:
        assert len(t_squad.squad) == 22
        assert isinstance(t_squad, Club)
        for player in t_squad.squad:
            assert isinstance(player, PlayerTeam)
