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
from decimal import Decimal
from ofm.core.simulation.event import EventOutcome, EventType, GameState, PitchPosition
from ofm.core.simulation.events import PassEvent, CrossEvent
from ofm.core.simulation.events.corner_kick_event import CornerKickType, CornerKickEvent


def get_corner_kick_event() -> CornerKickEvent:
    return CornerKickEvent(
        EventType.CORNER_KICK, GameState(Decimal(0.0), PitchPosition.OFF_LEFT)
    )


def test_normal_corner_kick_event(simulation_teams):
    event = get_corner_kick_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    first_player = home_team.player_in_possession
    event.calculate_event(home_team, away_team)
    assert event.sub_event.receiving_player != first_player


def test_corner_kick_pass_miss_event(simulation_teams, monkeypatch):
    def get_corner_kick_type(self, attacking_team) -> CornerKickType:
        return CornerKickType.PASS

    def get_pass_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.PASS_MISS

    monkeypatch.setattr(CornerKickEvent, "get_corner_kick_type", get_corner_kick_type)
    monkeypatch.setattr(PassEvent, "get_pass_primary_outcome", get_pass_primary_outcome)
    event = get_corner_kick_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.PASS_MISS
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert event.attacking_player.statistics.passes_missed == 1
    assert event.attacking_player.statistics.passes == 1
    assert home_team.stats.passes_missed == 1
    assert home_team.stats.passes == 1


def test_corner_kick_pass_intercept_event(simulation_teams, monkeypatch):
    def get_corner_kick_type(self, attacking_team) -> CornerKickType:
        return CornerKickType.PASS

    def get_pass_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.PASS_INTERCEPT

    monkeypatch.setattr(CornerKickEvent, "get_corner_kick_type", get_corner_kick_type)
    monkeypatch.setattr(PassEvent, "get_pass_primary_outcome", get_pass_primary_outcome)
    event = get_corner_kick_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.PASS_INTERCEPT
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert event.defending_player.statistics.interceptions == 1
    assert event.attacking_player.statistics.passes == 1
    assert event.attacking_player.statistics.passes_missed == 1
    assert away_team.stats.interceptions == 1
    assert home_team.stats.passes == 1


def test_corner_kick_pass_success_event(simulation_teams, monkeypatch):
    def get_corner_kick_type(self, attacking_team) -> CornerKickType:
        return CornerKickType.PASS

    def get_pass_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.PASS_SUCCESS

    monkeypatch.setattr(CornerKickEvent, "get_corner_kick_type", get_corner_kick_type)
    monkeypatch.setattr(PassEvent, "get_pass_primary_outcome", get_pass_primary_outcome)
    event = get_corner_kick_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.PASS_SUCCESS
    assert home_team.in_possession is True
    assert away_team.in_possession is False
    assert event.sub_event.receiving_player == home_team.player_in_possession
    assert event.defending_player.statistics.interceptions == 0
    assert event.attacking_player.statistics.passes == 1
    assert event.attacking_player.statistics.passes_missed == 0
    assert away_team.stats.interceptions == 0
    assert home_team.stats.passes == 1


def test_corner_kick_cross_miss_event(simulation_teams, monkeypatch):
    def get_corner_kick_type(self, attacking_team) -> CornerKickType:
        return CornerKickType.CROSS

    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.CROSS_MISS

    monkeypatch.setattr(CornerKickEvent, "get_corner_kick_type", get_corner_kick_type)
    monkeypatch.setattr(
        CrossEvent, "get_cross_primary_outcome", get_cross_primary_outcome
    )
    event = get_corner_kick_event()
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


def test_corner_kick_cross_intercept_event(simulation_teams, monkeypatch):
    def get_corner_kick_type(self, attacking_team) -> CornerKickType:
        return CornerKickType.CROSS

    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.CROSS_INTERCEPT

    monkeypatch.setattr(CornerKickEvent, "get_corner_kick_type", get_corner_kick_type)
    monkeypatch.setattr(
        CrossEvent, "get_cross_primary_outcome", get_cross_primary_outcome
    )
    event = get_corner_kick_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.CROSS_INTERCEPT
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert event.defending_player.statistics.interceptions == 1
    assert event.attacking_player.statistics.crosses == 1
    assert event.attacking_player.statistics.crosses_missed == 1
    assert away_team.stats.interceptions == 1
    assert home_team.stats.crosses == 1


def test_corner_kick_cross_success_event(simulation_teams, monkeypatch):
    def get_corner_kick_type(self, attacking_team) -> CornerKickType:
        return CornerKickType.CROSS

    def get_cross_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.CROSS_SUCCESS

    monkeypatch.setattr(CornerKickEvent, "get_corner_kick_type", get_corner_kick_type)
    monkeypatch.setattr(
        CrossEvent, "get_cross_primary_outcome", get_cross_primary_outcome
    )
    event = get_corner_kick_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.CROSS_SUCCESS
    assert home_team.in_possession is True
    assert away_team.in_possession is False
    assert event.sub_event.receiving_player == home_team.player_in_possession
    assert event.defending_player.statistics.interceptions == 0
    assert event.attacking_player.statistics.crosses == 1
    assert event.attacking_player.statistics.crosses_missed == 0
    assert away_team.stats.interceptions == 0
    assert home_team.stats.crosses == 1
