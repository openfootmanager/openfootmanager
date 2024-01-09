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
from ofm.core.simulation.events import DribbleEvent
from ofm.core.simulation.game_state import GameState, SimulationStatus


def get_dribble_event() -> DribbleEvent:
    return DribbleEvent(
        EventType.DRIBBLE,
        GameState(
            timedelta(minutes=10),
            SimulationStatus.FIRST_HALF,
            PitchPosition.OFF_MIDFIELD_CENTER,
        ),
    )


def test_dribble_fail_event(simulation_teams, monkeypatch):
    def get_dribble_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.DRIBBLE_FAIL

    monkeypatch.setattr(
        DribbleEvent, "get_dribble_primary_outcome", get_dribble_primary_outcome
    )
    event = get_dribble_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.DRIBBLE_FAIL
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert event.attacking_player.statistics.dribbles_failed == 1
    assert event.attacking_player.statistics.dribbles == 1
    assert home_team.stats.dribbles == 1
    assert home_team.stats.dribbles_failed == 1


def test_dribble_success_event(simulation_teams, monkeypatch):
    def get_end_position(self, attacking_team) -> PitchPosition:
        return PitchPosition.OFF_BOX

    def get_dribble_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.DRIBBLE_SUCCESS

    monkeypatch.setattr(DribbleEvent, "get_end_position", get_end_position)
    monkeypatch.setattr(
        DribbleEvent, "get_dribble_primary_outcome", get_dribble_primary_outcome
    )
    event = get_dribble_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.DRIBBLE_SUCCESS
    assert home_team.in_possession is True
    assert away_team.in_possession is False
    assert event.defending_player.statistics.interceptions == 0
    assert event.attacking_player.statistics.dribbles == 1
    assert event.attacking_player.statistics.dribbles_failed == 0
    assert home_team.stats.dribbles == 1
    assert home_team.stats.dribbles_failed == 0
