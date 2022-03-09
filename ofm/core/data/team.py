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
from typing import Union


@dataclass
class Team:
    team_id: Union[int, UUID]
    name: str
    nationality: str
    stadium_name: str
    international_reputation: int
    overall: int
    financial_status: float
    
    @classmethod
    def get_from_dict(cls, data):
        return cls(
            **{
                key: (data[key] if val.default == val.empty else data.get(key, val.default))
                for key, val in inspect.signature(Team).parameters.items()
            }
        )
    