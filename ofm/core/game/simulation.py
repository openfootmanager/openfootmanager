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
from .match import Match
from ..common.team import TeamSimulation
from .event_handler import EventHandler


class MatchSimulation:
    def __init__(
            self,
            match: Match,
            team1: TeamSimulation,
            team2: TeamSimulation,
            possible_extra_time: bool,
            possible_penalties: bool
    ):
        self.match = match
        self.team1 = team1
        self.team2 = team2
        self.teams = [self.team1, self.team2]
        self.possible_extra_time = possible_extra_time
        self.possible_penalties = possible_penalties
        self.event_handler = EventHandler(possible_extra_time, possible_penalties)
        self.running = False
        self.is_paused = False
        self.game_is_over = False

    def check_is_players_team(self) -> bool:
        return any(team.is_players_team for team in self.match.teams)

    def simulate_match(self):
        self.running = True
        while self.running or not self.is_paused:
            self.event_handler.get_possible_events()

