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
from ofm.core.api.game.player import Player
from ofm.core.api.game.team import Team


def read_file_data(file: str):
    with open(file, "r", encoding="utf-8") as fp:
        return json.load(fp)


def get_teams(file: str):
    data = read_file_data(file)
    teams = []

    for player in data:
        pl = Player.get_from_dict(player)
        if player["team"] not in teams:
            team = Team(
                player["team"],
            )
            teams.append(team)
        else:
            team = get_team(teams, player["team"])

        team.roster.append(pl)

    return teams


def serialize_data(input_file: str, output_file: str):
    teams = get_teams(input_file)
    teams_dict = [team.get_team_full_dict() for team in teams]

    with open(output_file, "w", encoding="utf-8") as fp:
        json.dump(teams_dict, fp)


def get_team(teams: list, team_name: str) -> Team:
    for team in teams:
        if team.name == team_name:
            return team


if __name__ == "__main__":
    serialize_data("../res/players_fifa.json", "../res/data.json")
