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
import random

from .match import Match


class MatchLive:
    def __init__(self, match: Match, show_commentary: bool = True):
        self.match = match
        self.show_commentary = show_commentary
        self.game_time = 0.0
        self.current_possession = None

    def is_half_time(self):
        return self.game_time == 45.0

    def match_simulation(self):
        pass

    def run(self):
        pass
