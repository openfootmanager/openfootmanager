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
from dataclasses import dataclass
from uuid import UUID
from typing import Tuple, Optional

from .player import PlayerSimulation, Player, PlayerTeam
from .formation import Formation


@dataclass
class Club:
    club_id: UUID
    name: str
    # TODO: Implement a serializable stadium object
    stadium: str
    stadium_capacity: int

    @classmethod
    def get_from_dict(cls, club: dict):
        club_id = UUID(int=club["id"])
        return Club(
            club_id,
            club["name"],
            club["stadium_name"],
            club["stadium_capacity"],
        )
    
    def serialize(self) -> dict:
        return {
            "id": self.club_id.int,
            "name": self.name,
            "stadium_name": self.stadium,
            "stadium_capacity": self.stadium_capacity,
        }


@dataclass
class ClubSquad:
    club: Club
    squad: list[PlayerTeam]

    @classmethod
    def get_from_dict(cls, club: dict, players_list: list[PlayerTeam]):
        team_id = UUID(int=club["id"])
        return ClubSquad(
            Club(team_id, club["name"], club["stadium_name"], club["stadium_capacity"]),
            squad=players_list,
        )

    def serialize(self) -> dict:
        return {
            "id": self.club.club_id.int,
            "squad": [player.details.player_id.int for player in self.squad]
        }


class TeamSimulation:
    def __init__(
            self,
            club: Club,
            players: list[PlayerSimulation] = None,
            bench: list[PlayerSimulation] = None,
            formation: Optional[Formation] = None,
            max_substitutions: int = 3,
    ):
        self.club: Club = club
        self.players: list[PlayerSimulation] = players
        self.bench: list[PlayerSimulation] = bench
        self.formation: Formation = formation
        self.in_possession: bool = False
        self.substitutions: int = 0
        self.sub_history: list[Tuple[PlayerSimulation, PlayerSimulation]]
        self.score: int = 0
        self.max_substitutions: int = max_substitutions
        self.stats: TeamStats = TeamStats(self.club.club_id)

    def update_player_stamina(self):
        pass

    def substitute_player(self, player1: PlayerSimulation, player2: PlayerSimulation):
        pass

    def remove_player(self, player: PlayerSimulation):
        """
        Remove player if it got injured, or received a red card.
        :param player:
        :return:
        """
        self.players.remove(player)


@dataclass
class TeamStats:
    club_id: UUID
    shots: int = 0
    shots_on_target: int = 0
    fouls: int = 0
    goals: int = 0
    own_goals: int = 0
    penalties: int = 0
    corners: int = 0
    throw_ins: int = 0
    kick_offs: int = 0
    injuries: int = 0
    yellow_cards: int = 0
    red_cards: int = 0
    avg_rating: float = 0.0
    possession: float = 0.0
    goals_conceded: int = 0

