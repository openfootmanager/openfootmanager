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
import os
import json
import logging

from ofm.core.api.game.team import Team
from ofm import RES_DIR


class Game:
    def __init__(self):
        self.players = None
        self.teams = []
        self.match = None
        self.match_live = None
        logging.basicConfig()
        self.logger = logging.getLogger(__file__)

    @staticmethod
    def get_data():
        filename = os.path.join(RES_DIR, "data.json")
        with open(filename, "r", encoding="utf-8") as fp:
            return json.load(fp)

    def run(self):
        data = self.get_data()
        teams = [Team.get_from_dict(team) for team in data]


if __name__ == '__main__':
    game = Game()
    game.run()
