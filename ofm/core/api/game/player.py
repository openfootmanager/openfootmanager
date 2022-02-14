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

import uuid
from typing import Union
from datetime import datetime, date, timedelta
from .positions import Positions


class Player:
    def __init__(
            self,
            fullname: str,
            short_name: str,
            nationality: str,
            dob: Union[str, datetime],
            skill: int,
            pos: str,
            international_rep: int,
            preferred_foot: str,
            player_id: uuid.UUID = None,
            shirt_number: int = 0,
            stamina: int = 100,
            value: float = 0,
            wage: float = 0,
            today: date = date.today(),
    ):
        self.player_id = uuid.uuid4() if player_id is None else player_id
        self.fullname = fullname
        self.short_name = short_name
        self.nationality = nationality
        if not isinstance(dob, datetime):
            self.dob = datetime.strptime(dob, "%Y-%m-%d")
        else:
            self.dob = dob
        self.age = self.get_player_age(today)
        self.shirt_number = shirt_number
        self.skill = int(skill)
        self.international_rep = international_rep
        self.preferred_foot = preferred_foot
        if value == '':
            value = 0.0
        self.value = float(value)
        if wage == '':
            wage = 0.0
        self.wage = float(wage)
        self.pos = pos
        self.stamina = stamina
        self.curr_pos = None

    def set_curr_pos(self, pos: Positions):
        """
        Sets the player's position in the game.
        :param pos: set the player
        :return:
        """
        self.curr_pos = pos

    def get_curr_pos(self):
        """
        Gets the player's current position in the game.
        :return:
        """
        return self.curr_pos

    def parse_pos(self):
        """
        Parses the player position string.

        The string consists of a player position such as:
        "RW, ST, FW, CF, MF" and etc.

        This function is a placeholder and later on we will have a separate module for interpreting
        and parsing positions based on strings or objects.

        :return: The positions enums are appended to the self.pos list.
        """
        if not isinstance(self.pos, str):
            raise ValueError("Position type not supported!")
        positions = self.pos.split(', ').copy()
        self.pos = []
        for position in positions:
            for pos in Positions:
                if position == pos.name:
                    self.pos.append(pos)

    def get_best_pos(self):
        """
        Gets the player's best position.

        For now the best position is the first one from the list of positions.
        :return: Returns the first position on the list of positions.
        """
        self.parse_pos()
        return self.pos[0]

    def get_player_age(self, today: date = date.today()) -> datetime.date:
        """
        Gets the current player age
        :param today:
        :return:
        """
        return today.year - self.dob.year - ((today.month, today.day) < (self.dob.month, self.dob.day))

    def get_dict(self):
        return {
            "name": self.fullname,
            "short_name": self.short_name,
            "nationality": self.nationality,
            "dob": datetime.strftime(self.dob, "%Y-%m-%d"),
            "overall": self.skill,
            "positions": self.pos,
            "international_reputation": self.international_rep,
            "preferred_foot": self.preferred_foot,
            "id": self.player_id,
            "club_number": self.shirt_number,
            "value": self.value,
            "wage": self.wage,
        }

    @classmethod
    def get_from_dict(cls, player: dict):
        return cls(
            player["name"],
            player["short_name"],
            player["nationality"],
            player["dob"],
            player["overall"],
            player["positions"],
            player["international_reputation"],
            player["preferred_foot"],
            player["id"],
            player["club_number"],
            value=player["value"],
            wage=player["wage"],
        )

    def get_int_id(self):
        return self.player_id.int

    def __str__(self):
        return self.short_name

    def __repr__(self):
        return self.short_name
