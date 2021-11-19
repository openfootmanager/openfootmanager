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
import uuid


class Team:
    def __init__(self, name, country=None, team_id=None, roster=None, is_national_team=False):
        self.team_id = uuid.uuid4() if team_id is None else team_id
        self.name = name
        self.country = country
        self.is_national_team = is_national_team
        self.roster = [] if roster is None else roster
        self.game_roster = []
        self.game_bench = [] 
        self.game_score = 0
        self._team_skill = 0

    @property
    def team_skill(self):
        self._team_skill = sum(player.skill for player in self.roster)
        return self.team_skill

    def get_team_dict(self):
        return {
            "id": self.team_id.int,
            "name": self.name,
            "roster": [player.player_id for player in self.roster]
        }

    def __repr__(self):
        return self.name

    def __str__(self):
        return self.name

    def __eq__(self, other):
        if isinstance(other, Team):
            return self.name == other.name

        if isinstance(other, str):
            return self.name == other

        return False
