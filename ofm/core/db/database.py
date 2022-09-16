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
from ofm.defaults import PLAYERS_FILE, TEAMS_FILE


def load_players():
    with open(PLAYERS_FILE, "r") as fp:
        return json.load(fp)


def load_teams():
    with open(TEAMS_FILE, "r") as fp:
        return json.load(fp)


def load_players_from_team(team_id: int, teams: list, players: list):
    """
    :team_id: the ID of the team to load the players from
    :teams: list of teams loaded with load_teams()
    :players: list of players loaded with load_players()
    """
    for team in teams:
        if team["id"] == team_id:
            player_ids = team["roster"]
            roster = [player for player in players if player["id"] in player_ids]



if __name__ == "__main__":
    # testing the function
    teams = load_teams()
    players = load_players()
    team1 = load_players_from_team()