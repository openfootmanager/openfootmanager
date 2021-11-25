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
import random
import json

from ofm.core.api.game.team import Team
from ofm.core.api.game.player import Player
from ofm.core.api.game.match import Match
from ofm.core.api.game.match_live import MatchLive
from ofm.core.api.file_management import find_file


class Game:
    def __init__(self):
        self.players = None
        self.teams = []
        self.match = None
        self.match_live = None

    def create_random_match(self):
        team1 = random.choice(self.teams)
        self.teams.remove(team1)
        team2 = random.choice(self.teams)
        self.teams.clear()
        self.match = Match(1, team1, team2)

    def play_random_match(self):
        print(self.match)
        self.match_live = MatchLive(self.match)
        self.match_live.run()

    def get_teams(self):
        filename = find_file('players_22.json')
        with open(filename, 'r') as fp:
            self.players = json.load(fp)
            self.get_players_from_team()

    def get_players_from_team(self):
        for player in self.players:
            pl = Player(
                player["name"],
                player["short_name"],
                player["nationality"],
                player["age"],
                player["dob"],
                player["overall"],
                player["positions"],
                player["international_reputation"],
                player["preferred_foot"],
                player["id"]
            )
            if player["team"] not in self.teams:
                team = Team(player["team"])
                team.roster.append(pl)
                self.teams.append(team)
            else:
                for team in self.teams:
                    if team == player["team"]:
                        team.roster.append(pl)

    def run(self):
        self.get_teams()
        self.create_random_match()
        self.play_random_match()


if __name__ == '__main__':
    game = Game()
    game.run()
