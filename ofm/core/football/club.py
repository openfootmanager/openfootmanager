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

from .player import PlayerTeam


class PlayerSubstitutionError(Exception):
    pass


@dataclass
class Club:
    club_id: UUID
    name: str
    country: str
    location: str
    default_formation: str
    # TODO: Implement a serializable stadium object
    squad: list[PlayerTeam]
    stadium: str
    stadium_capacity: int

    @classmethod
    def get_from_dict(cls, club: dict, players: list[PlayerTeam]):
        club_id = UUID(int=club["id"])
        return cls(
            club_id,
            club["name"],
            club["country"],
            club["location"],
            club["default_formation"],
            players,
            club["stadium"],
            club["stadium_capacity"],
        )

    def serialize(self) -> dict:
        return {
            "id": self.club_id.int,
            "name": self.name,
            "country": self.country,
            "location": self.location,
            "default_formation": self.default_formation,
            "squad": [player.details.player_id.int for player in self.squad],
            "stadium": self.stadium,
            "stadium_capacity": self.stadium_capacity,
        }

    def __repr__(self):
        return self.name.encode("utf-8").decode("unicode_escape")

    def __str__(self):
        return self.name.encode("utf-8").decode("unicode_escape")
