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
from datetime import timedelta

from ofm.core.simulation.event import EventOutcome, EventType, PitchPosition
from ofm.core.simulation.events import CrossEvent
from ofm.core.simulation.game_state import GameState, SimulationStatus


def get_cross_event() -> CrossEvent:
    return CrossEvent(
        EventType.CROSS,
        GameState(
            timedelta(minutes=10), SimulationStatus.FIRST_HALF, PitchPosition.OFF_LEFT
        ),
    )


def test_normal_cross_event(simulation_teams):
    event = get_cross_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    first_player = home_team.player_in_possession
    event.calculate_event(home_team, away_team)
    assert event.receiving_player != first_player
    if event.outcome == EventOutcome.CROSS_SUCCESS:
        assert event.receiving_player.received_ball == first_player
    else:
        assert event.receiving_player.received_ball is None
    assert event.attacking_player.received_ball is None
    assert event.defending_player.received_ball is None


def test_player_crosses_twice(simulation_teams):
    event = get_cross_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    first_player = home_team.player_in_possession
    event.calculate_event(home_team, away_team)
    assert event.receiving_player != first_player
    if event.outcome == EventOutcome.CROSS_SUCCESS:
        assert event.receiving_player.received_ball == first_player
    else:
        assert event.receiving_player.received_ball is None
    assert event.attacking_player.received_ball is None
    assert event.defending_player.received_ball is None
    event = get_cross_event()
    home_team.player_in_possession = first_player
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert first_player.statistics.crosses == 2
    assert event.receiving_player != first_player
    if event.outcome == EventOutcome.CROSS_SUCCESS:
        assert event.receiving_player.received_ball == first_player
    else:
        assert event.receiving_player.received_ball is None
    assert event.attacking_player.received_ball is None
    assert event.defending_player.received_ball is None


def test_cross_miss_event(simulation_teams, monkeypatch):
    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.CROSS_MISS

    def get_intercept_prob(self) -> EventOutcome:
        return EventOutcome.CROSS_MISS

    monkeypatch.setattr(
        CrossEvent, "get_cross_primary_outcome", get_cross_primary_outcome
    )
    monkeypatch.setattr(CrossEvent, "get_intercept_prob", get_intercept_prob)
    event = get_cross_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.CROSS_MISS
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert event.attacking_player.statistics.crosses_missed == 1
    assert event.attacking_player.statistics.crosses == 1
    assert home_team.stats.crosses_missed == 1
    assert home_team.stats.crosses == 1


def test_cross_intercept_event(simulation_teams, monkeypatch):
    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.CROSS_INTERCEPT

    def get_intercept_prob(self) -> EventOutcome:
        return EventOutcome.CROSS_INTERCEPT

    monkeypatch.setattr(
        CrossEvent, "get_cross_primary_outcome", get_cross_primary_outcome
    )
    monkeypatch.setattr(CrossEvent, "get_intercept_prob", get_intercept_prob)
    event = get_cross_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.CROSS_INTERCEPT
    assert event.receiving_player.received_ball is None
    assert event.attacking_player.received_ball is None
    assert event.defending_player.received_ball is None
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert event.defending_player.statistics.interceptions == 1
    assert event.attacking_player.statistics.crosses == 1
    assert event.attacking_player.statistics.crosses_missed == 1
    assert away_team.stats.interceptions == 1
    assert home_team.stats.crosses == 1


def test_cross_offside_event(simulation_teams, monkeypatch):
    def get_end_position(self, attacking_team) -> PitchPosition:
        return PitchPosition.OFF_BOX

    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.CROSS_SUCCESS

    def get_secondary_outcome(self) -> EventOutcome:
        return EventOutcome.CROSS_OFFSIDE

    monkeypatch.setattr(CrossEvent, "get_end_position", get_end_position)
    monkeypatch.setattr(
        CrossEvent, "get_cross_primary_outcome", get_cross_primary_outcome
    )
    monkeypatch.setattr(CrossEvent, "get_secondary_outcome", get_secondary_outcome)
    event = get_cross_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.CROSS_OFFSIDE
    assert event.receiving_player.received_ball is None
    assert event.attacking_player.received_ball is None
    assert event.defending_player.received_ball is None
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert event.defending_player.statistics.interceptions == 0
    assert event.attacking_player.statistics.crosses == 1
    assert event.attacking_player.statistics.crosses_missed == 1
    assert away_team.stats.interceptions == 0
    assert home_team.stats.crosses == 1


def test_cross_success_event(simulation_teams, monkeypatch):
    def get_end_position(self, attacking_team) -> PitchPosition:
        return PitchPosition.OFF_BOX

    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.CROSS_SUCCESS

    def get_secondary_outcome(self) -> EventOutcome:
        return EventOutcome.CROSS_SUCCESS

    monkeypatch.setattr(CrossEvent, "get_end_position", get_end_position)
    monkeypatch.setattr(
        CrossEvent, "get_cross_primary_outcome", get_cross_primary_outcome
    )
    monkeypatch.setattr(CrossEvent, "get_secondary_outcome", get_secondary_outcome)
    event = get_cross_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.CROSS_SUCCESS
    assert event.receiving_player.received_ball == event.attacking_player
    assert event.attacking_player.received_ball is None
    assert event.defending_player.received_ball is None
    assert home_team.in_possession is True
    assert away_team.in_possession is False
    assert event.receiving_player == home_team.player_in_possession
    assert event.defending_player.statistics.interceptions == 0
    assert event.attacking_player.statistics.crosses == 1
    assert event.attacking_player.statistics.crosses_missed == 0
    assert away_team.stats.interceptions == 0
    assert home_team.stats.crosses == 1
