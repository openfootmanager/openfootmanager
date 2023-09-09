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
from ofm.core.simulation.event import (
    EventOutcome,
    EventType,
    GameState,
    PitchPosition,
)

from ofm.core.simulation.events import CrossEvent


def get_cross_event() -> CrossEvent:
    return CrossEvent(EventType.PASS, GameState(0.0, PitchPosition.OFF_MIDFIELD_CENTER))


# def test_normal_cross_event(simulation_teams):
#     event = get_cross_event()
#     home_team, away_team = simulation_teams
#     home_team.in_possession = True
#     home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
#     away_team.in_possession = False
#     away_team.player_in_possession = None
#     first_player = home_team.player_in_possession
#     event.calculate_event(home_team, away_team)
#     assert event.receiving_player != first_player
