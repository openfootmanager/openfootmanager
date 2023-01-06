#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2023  Pedrenrique G. Guimarães
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
import uuid
from .generators import PlayerGenerator, TeamGenerator
from typing import Optional, List
from ofm.core.common.team import Team
from ofm.core.common.player import Player, Positions, PlayerTeam
from ofm.core.settings import Settings


class DatabaseLoadError(Exception):
    pass


class PlayerTeamLoadError(Exception):
    pass


class DB:
    def __init__(self, settings: Settings):
        self.settings = settings
    
    @property
    def players_file(self):
        return self.settings.players_file
    
    @property
    def teams_file(self):
        return self.settings.teams_file

    def load_teams(self) -> list[dict]:
        with open(self.teams_file, "r") as fp:
            return json.load(fp)

    def load_players(self) -> list[dict]:
        with open(self.players_file, "r") as fp:
            return json.load(fp)

    def load_player_objects(self, players: list[dict]) -> list[Player]:
        return [Player.get_from_dict(player) for player in players]

    def load_team_objects(self, teams: list[dict], players: list[Player]) -> list[Team]:
        return [Team.get_from_dict(team, players) for team in teams]
    
    def get_player_object_from_id(self, player_id: uuid.UUID, players: list[dict]) -> Player:
        if not players:
            raise DatabaseLoadError("Players list cannot be empty!")

        for player in players:
            if uuid.UUID(int=player["id"]) == player_id:
                return Player.get_from_dict(player)
        
        raise DatabaseLoadError("Player does not exist in database!")

    def get_player_team_from_dicts(self, squad_ids: list[dict], players: list[Player]) -> list[PlayerTeam]:
        squad = []
        for player in players:
            for pl_id in squad_ids:
                if player.player_id.int == pl_id["player_id"]:
                    squad.append(PlayerTeam.get_from_dict(pl_id, players))
        
        if squad:
            return squad
        else:
            raise PlayerTeamLoadError("Squad not found in database of players!")
    
    def generate_players(self, amount: int = 50 * 22, region: str = None, desired_pos: Optional[List[Positions]] = None) -> list[Player]:
        players = PlayerGenerator()
        players.generate(amount, region, desired_pos)
        players_dict = players.get_players_dictionaries()
        with open(self.players_file, "w") as fp:
            json.dump(players_dict, fp)

    def generate_teams(self) -> list[Team]:
        teams = TeamGenerator()
        teams.generate()
        teams_dict = teams.get_teams_dictionaries()
        with open(self.teams_file, "w") as fp:
            json.dump(teams_dict, fp)
