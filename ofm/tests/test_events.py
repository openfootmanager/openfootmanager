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
from unittest.mock import Mock
from ofm.core.game.event_handler import EventHandler
from ofm.core.game.events import *


def assert_event_correct_type(event_type, event_class):
    event_handler = EventHandler(possible_extra_time=False, possible_penalties=False)
    event = event_handler.get_event(event_type, Mock(), Mock())
    assert isinstance(event, event_class)


def test_get_penalty_event():
    assert_event_correct_type("penalty", PenaltyEvent)


def test_get_foul_event():
    assert_event_correct_type("foul", FoulEvent)


def test_get_free_kick_event():
    assert_event_correct_type("free_kick", FreeKickEvent)


def test_get_goal_opportunity_event():
    assert_event_correct_type("goal_opportunity", GoalOpportunityEvent)


def test_get_start_match_event():
    assert_event_correct_type("start_match", StartMatchEvent)


def test_get_corner_kick_event():
    assert_event_correct_type("corner_kick", CornerKickEvent)


def test_get_injury_event():
    assert_event_correct_type("injury", InjuryEvent)


def test_get_substitution_event():
    assert_event_correct_type("substitution", SubstitutionEvent)


def test_get_yellow_card_event():
    assert_event_correct_type("yellow_card", YellowCardEvent)


def test_get_red_card_event():
    assert_event_correct_type("red_card", RedCardEvent)


def test_get_minutes_from_event():
    goal_opp = GoalOpportunityEvent(25, Mock(), Mock())
    assert goal_opp.minutes == 25


def test_event_duel():
    pass
