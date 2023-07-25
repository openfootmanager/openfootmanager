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
from enum import Enum, auto


class TeamStrategy(Enum):
    NORMAL = 0
    KEEP_POSSESSION = auto()
    COUNTER_ATTACK = auto()
    DEFEND = auto()
    ALL_ATTACK = auto()
    

class TeamStrategyFactory:
    def get_game_transition_matrix(self, strategy: TeamStrategy):
        pass
    
    def get_pass_transition_matrix(self, strategy: TeamStrategy):
        match strategy:
            case TeamStrategy.NORMAL:
                return [
                    [1, 4, 6, 8, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1],  # BOX
                    [1, 4, 6, 8, 5, 2, 4, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_LEFT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_MIDFIELD
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4],  # MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 4, 6, 4, 8, 6, 6, 8, 8, 8],  # OFF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 6],  # OFF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 8, 6],  # OFF_MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 8, 14],  # OFF_BOX
                ]
            case TeamStrategy.KEEP_POSSESSION:
                return [
                    [1, 4, 6, 8, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1],  # BOX
                    [1, 4, 6, 8, 5, 2, 4, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_LEFT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_MIDFIELD
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4],  # MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 4, 6, 4, 8, 6, 6, 8, 8, 8],  # OFF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 6],  # OFF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 8, 6],  # OFF_MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 8, 14],  # OFF_BOX
                ]
            case TeamStrategy.COUNTER_ATTACK:
                return [
                    [1, 4, 6, 8, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1],  # BOX
                    [1, 4, 6, 8, 5, 2, 4, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_LEFT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_MIDFIELD
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4],  # MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 4, 6, 4, 8, 6, 6, 8, 8, 8],  # OFF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 6],  # OFF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 8, 6],  # OFF_MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 8, 14],  # OFF_BOX
                ]
            case TeamStrategy.DEFEND:
                return [
                    [1, 4, 6, 8, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1],  # BOX
                    [1, 4, 6, 8, 5, 2, 4, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_LEFT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_MIDFIELD
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4],  # MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 4, 6, 4, 8, 6, 6, 8, 8, 8],  # OFF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 6],  # OFF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 8, 6],  # OFF_MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 8, 14],  # OFF_BOX
                ]
            case TeamStrategy.ALL_ATTACK:
                return [
                    [1, 4, 6, 8, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1],  # BOX
                    [1, 4, 6, 8, 5, 2, 4, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_LEFT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_BOX
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],  # DEF_RIGHT_MIDFIELD
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4],  # MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 4],  # MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 4, 6, 4, 8, 6, 6, 8, 8, 8],  # OFF_MIDFIELD_CENTER
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 6],  # OFF_MIDFIELD_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 14, 8, 8, 6],  # OFF_MIDFIELD_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_LEFT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 6, 14],  # OFF_RIGHT
                    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 8, 8, 14],  # OFF_BOX
                ]