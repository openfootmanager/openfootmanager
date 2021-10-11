#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2021  Pedrenrique G. Guimarães
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


class DatabaseError(Exception):
    pass


class Database:
    def __init__(self):
        self.connection = None
    
    def get_players_list(self):
        pass

    def load_player(self, player):
        players_list = self.get_players_list()
        for player_obj in players_list:
            if player_obj.player_id == player:
                return player_obj
        else:
            raise DatabaseError("Player ID {} not found in the database!".format(player))

