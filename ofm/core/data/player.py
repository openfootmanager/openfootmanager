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
import inspect
from uuid import UUID
from dataclasses import dataclass
from datetime import datetime, date
from typing import Union


@dataclass
class Player:
    player_id: Union[int, UUID]
    name: str
    short_name: str
    club_number: int
    date_of_birth: Union[datetime, date, str]
    nationality: str
    international_reputation: int
    overall: int
    positions: str
    potential: int
    preferred_foot: str
    value: float
    wage: float
    
    def get_age(self, today: datetime = date.today()):
        if isinstance(self.date_of_birth, str):
            date_of_birth = datetime.strptime(self.date_of_birth, "%Y-%m-%d")
        if isinstance(self.date_of_birth, (datetime, date)):
            date_of_birth = date
        
        return today.year - date_of_birth.year - ((today.month, today.day) < (date_of_birth.month, date_of_birth.day))
    
    @classmethod
    def get_from_dict(cls, data):
        return cls(
            **{
                key: (data[key] if val.default == val.empty else data.get(key, val.default))
                for key, val in inspect.signature(Player).parameters.items()
            }
        )
    