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
from ofm.core.api.database import Database


class Team:
    def __init__(self, name, country, team_id, roster, is_national_team=False):
        self.team_id = team_id
        self.name = name
        self.country = country
        self.is_national_team = is_national_team
        self.roster = self.get_roster(roster)
        self.game_roster = []
        self.game_bench = [] 
        self.game_score = 0
        self._team_skill = 0

    @staticmethod
    def get_roster(self, roster):
        database = Database()
        return [
            database.load_player(player_id)
            for player_id in roster
        ]
    
    def get_game_roster(self):
        pass

    def get_bench_roster(self):
        pass

    @property
    def team_skill(self):
        self._team_skill = sum(player.skill for player in self.roster)
        return self.team_skill
    
    