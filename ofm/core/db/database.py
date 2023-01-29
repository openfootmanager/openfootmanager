#      Openfoot Manager - A free and open source soccer management simulation
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
import datetime
import json
import uuid
from typing import Optional, List

from ofm.core.football.club import Club
from ofm.core.football.player import Player, Positions, PlayerTeam
from ofm.core.settings import Settings
from .generators import PlayerGenerator, TeamGenerator


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
    def squads_file(self):
        return self.settings.squads_file

    @property
    def player_teams_file(self):
        return self.settings.player_teams

    @property
    def clubs_file(self):
        return self.settings.clubs_file

    @property
    def squads_def_file(self):
        return self.settings.squads_def

    @property
    def clubs_def_file(self):
        return self.settings.clubs_def

    def load_clubs(self) -> list[dict]:
        with open(self.clubs_file, "r") as fp:
            return json.load(fp)

    def load_players(self) -> list[dict]:
        with open(self.players_file, "r") as fp:
            return json.load(fp)

    def load_club_definitions(self):
        with open(self.clubs_def_file, 'r') as fp:
            return json.load(fp)

    def load_squad_definitions(self):
        with open(self.squads_def_file, 'r') as fp:
            return json.load(fp)

    def load_player_objects(self, players: list[dict]) -> list[Player]:
        return [Player.get_from_dict(player) for player in players]

    def load_club_objects(self, clubs: list[dict], players: list[Player]) -> list[Club]:
        _clubs = []
        for club in clubs:
            squad = self.get_player_team_from_dicts(club["squad"], players)
            _clubs.append(Club.get_from_dict(club, squad))

        if not _clubs:
            raise DatabaseLoadError("Could not load clubs from definition")

        return _clubs

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
            found = False
            for pl_id in squad_ids:
                if player.player_id.int == pl_id["id"]:
                    squad.append(PlayerTeam.get_from_dict(pl_id, players))
                    found = True
                    break

            if not found:
                raise DatabaseLoadError(f"Player ID not found in the database!")

        if not squad:
            raise PlayerTeamLoadError("Squad not found in database of players!")

        return squad

    def generate_players(self, amount: int = 50 * 22, region: str = None,
                         desired_pos: Optional[List[Positions]] = None):
        players = PlayerGenerator()
        players.generate(amount, region, desired_pos)
        players_dict = players.get_players_dictionaries()
        with open(self.players_file, "w") as fp:
            json.dump(players_dict, fp)

    def generate_teams(self, clubs_def: Optional[list[dict]], squads_def: Optional[list[dict]],
                       season_start: datetime.date = datetime.date.today()):
        if not clubs_def:
            clubs_def = self.load_club_definitions()
        if not squads_def:
            squads_def = self.load_squad_definitions()
        team_generator = TeamGenerator(clubs_def, squads_def, season_start)
        teams = team_generator.generate()
        squads_dict = [team.serialize() for team in teams]
        with open(self.squads_file, "w") as fp:
            json.dump(squads_dict, fp)
