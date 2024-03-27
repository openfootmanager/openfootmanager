#      Openfoot Manager - A free and open source soccer management simulation
#      Copyright (C) 2020-2024  Pedrenrique G. Guimarães
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
from datetime import timedelta

import pytest

from ofm.core.football.formation import FormationError
from ofm.core.football.team_simulation import (
    GameEventType,
    PlayerSimulation,
    SubbingError,
    SubstitutionEvent,
)
from ofm.core.simulation import PitchPosition
from ofm.core.simulation.event_type import EventType
from ofm.core.simulation.events import EventFactory, PassEvent
from ofm.core.simulation.fixture import Fixture
from ofm.core.simulation.game_state import GameState, SimulationStatus
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

    def get_event_duration(self):
        return timedelta(seconds=5)

    monkeypatch.setattr(SimulationEngine, "run", get_simulation_engine)
    monkeypatch.setattr(SimulationEngine, "get_event_duration", get_event_duration)

    live_game = LiveGame(fixture, home_team_sim, away_team_sim, False, False, True)
    live_game.running = True
    return live_game


def test_formations_are_complete(live_game: LiveGame):
    assert live_game.engine.home_team.formation.players is not None
    assert live_game.engine.away_team.formation.players is not None
    assert live_game.engine.home_team.formation.bench is not None
    assert live_game.engine.away_team.formation.bench is not None


def test_penalty_shootout_enabled_if_extra_time_is_enabled(live_game):
    live_game.possible_extra_time = True
    assert live_game.possible_penalties is True


def test_game_breaks_in_half_time(live_game):
    live_game.no_break = False
    assert live_game.state.status == SimulationStatus.NOT_STARTED
    live_game.run()
    assert live_game.minutes == timedelta(seconds=2700)
    assert live_game.state.status == SimulationStatus.FIRST_HALF_BREAK
    assert live_game.is_game_over is False


def test_game_ends_after_90_minutes(live_game):
    assert live_game.state.status == SimulationStatus.NOT_STARTED
    live_game.run()
    assert live_game.minutes == timedelta(minutes=90)
    assert live_game.is_game_over is True
    assert live_game.state.status == SimulationStatus.FINISHED


def test_game_breaks_in_extra_time(live_game):
    live_game.no_break = False
    live_game.possible_extra_time = True
    assert live_game.state.status == SimulationStatus.NOT_STARTED
    live_game.run()
    assert live_game.state.status == SimulationStatus.FIRST_HALF_BREAK
    live_game.run()
    assert live_game.state.status == SimulationStatus.SECOND_HALF_BREAK
    assert live_game.minutes == timedelta(minutes=90)
    assert live_game.is_game_over is False


def test_game_breaks_in_extra_time_half_time(live_game):
    live_game.no_break = False
    live_game.possible_extra_time = True
    live_game.run()  # first half
    assert live_game.state.status == SimulationStatus.FIRST_HALF_BREAK
    live_game.run()  # second half
    assert live_game.state.status == SimulationStatus.SECOND_HALF_BREAK
    live_game.run()  # first et half
    assert live_game.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK
    assert live_game.minutes == timedelta(minutes=105)
    assert live_game.is_game_over is False


def test_game_breaks_to_penalty_shootout(live_game):
    live_game.no_break = False
    live_game.possible_extra_time = True
    live_game.run()  # first half
    assert live_game.state.status == SimulationStatus.FIRST_HALF_BREAK
    live_game.run()  # second half
    assert live_game.state.status == SimulationStatus.SECOND_HALF_BREAK
    live_game.run()  # first et half
    assert live_game.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK
    live_game.run()  # second et half
    assert live_game.state.status == SimulationStatus.SECOND_HALF_EXTRA_TIME_BREAK
    assert live_game.minutes == timedelta(minutes=120)
    assert live_game.is_game_over is False


def test_game_breaks_and_does_not_go_to_extra_time(live_game, player_sim):
    live_game.no_break = False
    live_game.possible_extra_time = True
    live_game.run()  # first half
    assert live_game.state.status == SimulationStatus.FIRST_HALF_BREAK
    live_game.engine.home_team.add_goal(player_sim, timedelta(minutes=45))
    live_game.run()  # second half
    assert live_game.state.status == SimulationStatus.FINISHED
    assert live_game.minutes == timedelta(minutes=90)
    assert live_game.is_game_over is True


def test_game_ends_in_120_minutes(live_game, player_sim):
    live_game.no_break = False
    live_game.possible_extra_time = True
    live_game.run()  # first half
    assert live_game.state.status == SimulationStatus.FIRST_HALF_BREAK
    live_game.run()  # second half
    assert live_game.state.status == SimulationStatus.SECOND_HALF_BREAK
    live_game.run()  # first et half
    assert live_game.state.status == SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK
    live_game.engine.home_team.add_goal(player_sim, timedelta(minutes=90))
    live_game.run()  # second et half
    assert live_game.state.status == SimulationStatus.FINISHED
    assert live_game.minutes == timedelta(minutes=120)
    assert live_game.is_game_over is True


def test_game_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    game_state = GameState(
        timedelta(minutes=0),
        SimulationStatus.NOT_STARTED,
        PitchPosition.MIDFIELD_CENTER,
    )
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        game_state,
        None,
    )
    assert event == EventType.PASS


def test_half_time_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    game_state = GameState(
        timedelta(minutes=45),
        SimulationStatus.FIRST_HALF_BREAK,
        PitchPosition.MIDFIELD_CENTER,
    )
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        game_state,
        PassEvent(EventType.PASS, game_state),
    )
    assert event == EventType.PASS


def test_extra_time_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    game_state = GameState(
        timedelta(minutes=90),
        SimulationStatus.SECOND_HALF_BREAK,
        PitchPosition.MIDFIELD_CENTER,
    )
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        game_state,
        PassEvent(EventType.PASS, game_state),
    )
    assert event == EventType.PASS


def test_extra_half_time_starts_with_pass_event(live_game):
    event_factory = EventFactory()
    game_state = GameState(
        timedelta(minutes=105),
        SimulationStatus.FIRST_HALF_EXTRA_TIME_BREAK,
        PitchPosition.MIDFIELD_CENTER,
    )
    event = event_factory.get_event_type(
        (live_game.engine.home_team, live_game.engine.away_team),
        game_state,
        PassEvent(EventType.PASS, game_state),
    )
    assert event == EventType.PASS


def test_get_added_time_in_45_minutes(live_game):
    game_state = GameState(
        timedelta(minutes=45),
        SimulationStatus.FIRST_HALF,
        PitchPosition.MIDFIELD_CENTER,
    )
    live_game.state = game_state
    live_game.get_added_time()
    assert live_game.state.in_additional_time is True
    assert live_game.state.minutes == timedelta(minutes=45)
    assert live_game.added_time is not None


def test_get_added_time_before_45_minutes(live_game):
    game_state = GameState(
        timedelta(minutes=44),
        SimulationStatus.FIRST_HALF,
        PitchPosition.MIDFIELD_CENTER,
    )
    live_game.state = game_state
    live_game.get_added_time()
    assert live_game.state.in_additional_time is False
    assert live_game.state.minutes == timedelta(minutes=44)
    assert live_game.added_time is None


def test_substitute_same_player(live_game):
    home_team = live_game.engine.home_team
    with pytest.raises(ValueError):
        home_team.sub_player(
            home_team.formation.fw[0],
            home_team.formation.fw[0],
            timedelta(minutes=45),
            timedelta(minutes=0),
        )


def test_substitute_invalid_player(live_game):
    home_team = live_game.engine.home_team
    away_team = live_game.engine.away_team
    with pytest.raises(FormationError):
        home_team.sub_player(
            home_team.formation.fw[0],
            away_team.formation.fw[0],
            timedelta(minutes=45),
            timedelta(minutes=0),
        )


def test_substitute_player(live_game):
    home_team = live_game.engine.home_team
    player_in = home_team.formation.bench[0]
    player_out = home_team.formation.fw[0]
    expected_sub = SubstitutionEvent(
        player_out,
        timedelta(minutes=45),
        GameEventType.SUBSTITUTION,
        player_in,
        timedelta(minutes=0),
    )
    home_team.sub_player(
        player_out, player_in, timedelta(minutes=45), timedelta(minutes=0)
    )
    assert player_in == home_team.formation.fw[0]
    assert player_out in home_team.formation.bench
    assert home_team.sub_history[0] == expected_sub


def test_substitute_invalid_order(live_game):
    home_team = live_game.engine.home_team
    player_in = home_team.formation.bench[0]
    player_out = home_team.formation.fw[0]
    with pytest.raises(ValueError):
        home_team.sub_player(
            player_in, player_out, timedelta(minutes=45), timedelta(minutes=0)
        )


def test_substitute_no_available_substitutions(live_game):
    home_team = live_game.engine.home_team
    home_team.substitutions = 5
    player_in = home_team.formation.bench[0]
    player_out = home_team.formation.fw[0]
    with pytest.raises(SubbingError):
        home_team.sub_player(
            player_out, player_in, timedelta(minutes=45), timedelta(minutes=0)
        )


def test_substitute_sent_off_player(live_game):
    home_team = live_game.engine.home_team
    player_in = home_team.formation.bench[0]
    player_out = home_team.formation.fw[0]
    player_out.statistics.red_cards = 1
    with pytest.raises(SubbingError):
        home_team.sub_player(
            player_out, player_in, timedelta(minutes=45), timedelta(minutes=0)
        )


def test_substitute_player_in_was_sent_off(live_game):
    home_team = live_game.engine.home_team
    player_in = home_team.formation.bench[0]
    player_in.statistics.red_cards = 1
    player_out = home_team.formation.fw[0]
    with pytest.raises(SubbingError):
        home_team.sub_player(
            player_out, player_in, timedelta(minutes=45), timedelta(minutes=0)
        )


def test_get_player_on_pitch(live_game):
    home_team = live_game.engine.home_team
    unable_player = home_team.formation.players[-1]
    unable_player.able_to_play = False
    for _ in range(1500):
        for position in list(PitchPosition):
            player = home_team.get_player_on_pitch(position)
            assert isinstance(player, PlayerSimulation) is True
            assert player != unable_player
