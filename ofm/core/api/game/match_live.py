#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2021  Pedrenrique G. Guimarães
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
from .live_game_events import LiveGameEventHandler
from .team import Team
from .player import Player


class MatchLive:
    def __init__(self, match: Match, show_commentary: bool = True):
        self.match = match
        self.event_handler = LiveGameEventHandler()
        self.show_commentary = show_commentary
        self.game_time = 0.0

    def run(self):
        while self.game_time < 90.0:
            self.event_handler.generate_events(self.game_time)
            self.game_time += 1

        for event in self.event_handler.event_history:
            print(event.event_type)
