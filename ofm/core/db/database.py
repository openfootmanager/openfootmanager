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
import json
from .generators import PlayerGenerator, TeamGenerator
from ofm.defaults import PLAYERS_FILE, TEAMS_FILE
from ofm.core.common.team import Team
from ofm.core.common.player import Player


class DB:
    def load_teams(self) -> list[dict]:
        with open(TEAMS_FILE, "r") as fp:
            return json.load(fp)

    def load_players(self) -> list[dict]:
        with open(PLAYERS_FILE, "r") as fp:
            return json.load(fp)

    def load_player_objects(self, players: list[dict]) -> list[Player]:
        return [Player.get_from_dict(player) for player in players]


    def load_team_objects(self, teams: list[dict], players: list[Player]) -> list[Team]:
        return [Team.get_from_dict(team, players) for team in teams]


    def generate_players(self) -> list[Player]:
        players = PlayerGenerator()


    def generate_teams(self) -> list[Team]:
        teams = TeamGenerator()
