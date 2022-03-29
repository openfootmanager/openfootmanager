import pytest
from ofm.core.data.team import Team


def test_player_from_dict():
    dictionary = {
        "team_id": 1,
        "name": "Manchester Utd",
        "nationality": "England",
        "stadium": "Old Trafford",
        "international_reputation": 5,
        "overall": 50,
        "financial_status": 5000.00,
    }
    team = Team.get_from_dict(dictionary)
    team_expected = Team(
        1,
        "Manchester Utd",
        "England",
        "Old Trafford",
        5,
        50,
        5000.00,
    )

    assert team == team_expected
