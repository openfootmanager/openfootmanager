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
from typing import Optional

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
    def players_file(self) -> str:
        return self.settings.players_file

    @property
    def squads_file(self) -> str:
        return self.settings.squads_file

    @property
    def clubs_file(self) -> str:
        return self.settings.clubs_file

    @property
    def clubs_def_file(self) -> str:
        return self.settings.clubs_def

    def load_clubs(self) -> list[dict]:
        with open(self.clubs_file, "r") as fp:
            return json.load(fp)

    def load_players(self) -> list[dict]:
        with open(self.players_file, "r") as fp:
            return json.load(fp)

    def load_club_definitions(self) -> list[dict]:
        with open(self.clubs_def_file, 'r') as fp:
            return json.load(fp)

    def load_squads_file(self) -> list[dict]:
        with open(self.squads_file, 'r') as fp:
            return json.load(fp)

    def load_player_objects(self, players: list[dict]) -> list[Player]:
        return [Player.get_from_dict(player) for player in players]

    def load_club_objects(self, clubs: list[dict], players: list[Player]) -> list[Club]:
        _clubs = []
        for club in clubs:
            squad = self.get_player_team_from_dicts(self.load_club_squads(club["squad"]), players)
            _clubs.append(Club.get_from_dict(club, squad))

        if not _clubs:
            raise DatabaseLoadError("Could not load clubs from definition")

        return _clubs

    def load_club_squads(self, squad_ids: list[int], squads: Optional[list[dict]] = None):
        sq = []
        if not squads:
            squads = self.load_squads_file()

        for player_id in squad_ids:
            for squad in squads:
                if squad["player_id"] == player_id:
                    sq.append(squad)
                    break

        return sq

    def get_player_object_from_id(self, player_id: uuid.UUID, players: list[dict]) -> Player:
        if not players:
            raise DatabaseLoadError("Players list cannot be empty!")

        for player in players:
            if uuid.UUID(int=player["id"]) == player_id:
                return Player.get_from_dict(player)

        raise DatabaseLoadError("Player does not exist in database!")

    def get_player_team_from_dicts(self, squads_dict: list[dict], players: list[Player]) -> list[PlayerTeam]:
        squad = []
        for player in players:
            for playerteam_dict in squads_dict:
                if player.player_id.int == playerteam_dict["player_id"]:
                    squad.append(PlayerTeam.get_from_dict(playerteam_dict, players))
                    break

        if not squad:
            raise PlayerTeamLoadError("Squad not found in database of players!")

        return squad

    def generate_players(self, amount: int = 50 * 22, region: str = None,
                         desired_pos: Optional[list[Positions]] = None) -> list[dict]:
        players = PlayerGenerator()
        players.generate(amount, region, desired_pos)
        players_dict = players.get_players_dictionaries()
        with open(self.players_file, "w") as fp:
            json.dump(players_dict, fp)
        return players_dict

    def generate_teams_and_squads(self, clubs_def: Optional[list[dict]],
                                  season_start: datetime.date = datetime.date.today()):
        if clubs_def is None:
            clubs_def = self.load_club_definitions()

        team_gen = TeamGenerator(clubs_def, season_start)
        clubs = team_gen.generate()
        clubs_dict = [club.serialize() for club in clubs]

        with open(self.clubs_file, 'w') as fp:
            json.dump(clubs_dict, fp)

        with open(self.players_file, 'w') as fp:
            players_dict = []
            for club in clubs:
                players_dict.extend(player.details.serialize() for player in club.squad)
            json.dump(players_dict, fp)

        with open(self.squads_file, 'w') as fp:
            squads_dict = []
            for club in clubs:
                squads_dict.extend(player.serialize() for player in club.squad)
            json.dump(squads_dict, fp)
