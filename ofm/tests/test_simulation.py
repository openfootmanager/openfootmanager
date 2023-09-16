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
import uuid

import pytest

from ofm.core.football.team_simulation import Goal
from ofm.core.simulation import PitchPosition
from ofm.core.simulation.event_type import EventType
from ofm.core.simulation.events import EventFactory, PassEvent
from ofm.core.simulation.fixture import Fixture
from ofm.core.simulation.game_state import GameState
from ofm.core.simulation.simulation import LiveGame, SimulationEngine


class MockSimulationEngine:
    def run(self):
        pass


@pytest.fixture
def live_game(monkeypatch, simulation_teams) -> LiveGame:
    def get_simulation_engine(*args, **kwargs):
        return MockSimulationEngine()

    home_team_sim, away_team_sim = simulation_teams
    home_team = home_team_sim.club
    away_team = away_team_sim.club
    fixture = Fixture(
        uuid.uuid4(),
        uuid.uuid4(),
        home_team.club_id,
        away_team.club_id,
        home_team.stadium,
    )

    monkeypatch.setattr(SimulationEngine, "run", get_simulation_engine)

    return LiveGame(
        fixture,
        home_team_sim,
        away_team_sim,
        False,
        False,
    )


def test_formations_are_complete(live_game: LiveGame):
    assert live_game.engine.home_team.formation.players is not None
    assert live_game.engine.away_team.formation.players is not None
    assert live_game.engine.home_team.formation.bench is not None
    assert live_game.engine.away_team.formation.bench is not None


def test_game_breaks_in_half_time(live_game):
    live_game.run()
    assert live_game.minutes == 45.0
    assert live_game.is_half_time is True


def test_game_breaks_in_90_min(live_game):
    live_game.run()
    live_game.reset_after_half_time()
    live_game.run()
    assert live_game.minutes == 90.0
    assert live_game.is_game_over is True
    assert live_game.is_half_time is False


def test_game_breaks_in_extra_time(live_game):
    live_game.possible_extra_time = True
    live_game.run()
    live_game.reset_after_half_time()
    live_game.run()
    assert live_game.minutes == 90.0
    assert live_game.is_game_over is False
    assert live_game.is_half_time is True


def test_game_breaks_in_extra_time_half_time(live_game):
    live_game.possible_extra_time = True
    live_game.run()  # first half
    live_game.reset_after_half_time()
    live_game.run()  # second half
    live_game.reset_after_half_time()
    live_game.run()
    assert live_game.minutes == 105.0
    assert live_game.is_game_over is False
    assert live_game.is_half_time is True


def test_game_breaks_after_extra_time(live_game):
    live_game.possible_extra_time = True
    live_game.run()  # first half
    live_game.reset_after_half_time()
    live_game.run()  # second half
    live_game.reset_after_half_time()
    live_game.run()  # first et half
    live_game.reset_after_half_time()
    live_game.run()
    assert live_game.minutes == 120.0
    assert live_game.is_game_over is True
    assert live_game.is_half_time is False


def test_game_breaks_to_penalty_shootout(live_game):
    live_game.possible_extra_time = True
    live_game.possible_penalties = True
    live_game.run()  # first half
    live_game.reset_after_half_time()
    live_game.run()  # second half
    live_game.reset_after_half_time()
    live_game.run()  # first et half
    live_game.reset_after_half_time()
    live_game.run()
    assert live_game.minutes == 120.0
    assert live_game.is_game_over is False
    assert live_game.is_half_time is True


def test_game_breaks_and_does_not_go_to_extra_time(live_game, player_sim):
    live_game.possible_extra_time = True
    live_game.run()  # first half
    live_game.reset_after_half_time()
    live_game.engine.home_team.add_goal(Goal(player_sim, 45.0))
    live_game.run()  # second half
    assert live_game.minutes == 90.0
    assert live_game.is_game_over is True
    assert live_game.is_half_time is False


def test_game_breaks_and_does_not_go_to_penalties(live_game, player_sim):
    live_game.possible_extra_time = True
    live_game.possible_penalties = True
    live_game.run()  # first half
    live_game.reset_after_half_time()
    live_game.run()  # second half
    live_game.reset_after_half_time()
    live_game.run()  # first et half
    live_game.reset_after_half_time()
    live_game.engine.home_team.add_goal(Goal(player_sim, 90.0))
    live_game.run()
    assert live_game.minutes == 120.0
    assert live_game.is_game_over is True
    assert live_game.is_half_time is False


def test_game_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        GameState(0.0, PitchPosition.MIDFIELD_CENTER),
        None,
    )
    assert event == EventType.PASS


def test_half_time_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        GameState(45.1, PitchPosition.MIDFIELD_CENTER),
        PassEvent(EventType.PASS, GameState(45.1, PitchPosition.MIDFIELD_CENTER)),
    )
    assert event == EventType.PASS


def test_extra_time_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        GameState(90.1, PitchPosition.MIDFIELD_CENTER),
        PassEvent(EventType.PASS, GameState(90.1, PitchPosition.MIDFIELD_CENTER)),
    )
    assert event == EventType.PASS


def test_extra_half_time_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        GameState(105.1, PitchPosition.MIDFIELD_CENTER),
        PassEvent(EventType.PASS, GameState(105.1, PitchPosition.MIDFIELD_CENTER)),
    )
    assert event == EventType.PASS
