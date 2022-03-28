import pytest
from ofm.core.data.player import Player

def test_player_from_dict():
    dictionary = {
        "player_id": 1,
        "name": "Joseph Doe",
        "short_name": "J. Doe",
        "club_number": 10,
        "date_of_birth": "1970-01-01",
        "nationality": "England",
        "international_reputation": 5,
        "overall": 50,
        "positions": "FW, RW, GK",
        "potential": 55,
        "preferred_foot": "left",
        "value": 3300.00,
        "wage": 2000.00,
    }
    player = Player.get_from_dict(dictionary)
    player_expected = Player(
        1,
        "Joseph Doe",
        "J. Doe",
        10,
        "1970-01-01",
        "England",
        5,
        50,
        "FW, RW, GK",
        55,
        "left",
        3300.00,
        2000.00,
    )


    assert player == player_expected
