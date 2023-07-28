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

from ofm.core.db.generators import TeamGenerator
from ofm.core.football.club import Formation, Goal, TeamSimulation
from ofm.core.football.player import PlayerSimulation
from ofm.core.simulation.fixture import Fixture
from ofm.core.simulation.simulation import LiveGame, SimulationEngine


class MockSimulationEngine:
    def run(self):
        pass


@pytest.fixture
def live_game(monkeypatch, squads_def, confederations_file) -> LiveGame:
    def get_simulation_engine(*args, **kwargs):
        return MockSimulationEngine()

    team_gen = TeamGenerator(squads_def, confederations_file)

    teams = team_gen.generate()
    home_team, away_team = teams[0], teams[1]
    fixture = Fixture(
        uuid.uuid4(),
        uuid.uuid4(),
        home_team.club_id,
        away_team.club_id,
        home_team.stadium,
    )

    home_team_formation = Formation(home_team.default_formation)
    home_team_formation.get_best_players(home_team.squad)
    home_team_sim = TeamSimulation(home_team, home_team_formation)
    away_team_formation = Formation(away_team.default_formation)
    away_team_sim = TeamSimulation(away_team, away_team_formation)

    monkeypatch.setattr(SimulationEngine, "run", get_simulation_engine)

    return LiveGame(
        fixture,
        home_team_sim,
        away_team_sim,
        False,
        False,
    )


def get_goal(player_sim: PlayerSimulation) -> Goal:
    return Goal(player_sim, 90.0)


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
    live_game.engine.home_team.add_goal(player_sim)
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
    live_game.engine.home_team.add_goal(get_goal(player_sim))
    live_game.run()
    assert live_game.minutes == 120.0
    assert live_game.is_game_over is True
    assert live_game.is_half_time is False
