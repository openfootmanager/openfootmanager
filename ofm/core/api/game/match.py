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


class Match:
    def __init__(self, match_id, team1, team2):
        self.match_id = match_id
        self.team1 = team1
        self.team2 = team2
        self.teams = [team1, team2]
        self.victorious_team = None

    def __repr__(self):
        return str(self.team1.name + " - " + self.team2.name)

    def __str__(self):
        return str(self.team1.name + " " + str(self.team1.game_score) + " - " + str(self.team2.game_score) + " " + self.team2.name)
