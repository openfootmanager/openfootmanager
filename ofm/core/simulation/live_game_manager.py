#      Openfoot Manager - A free and open source soccer management simulation
#      Copyright (C) 2020-2024  Pedrenrique G. Guimarães
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
from threading import Thread
from typing import Optional

from .simulation import LiveGame, TeamSimulation


class LiveGameManager:
    def __init__(self):
        self.game_thread: Optional[Thread] = None
        self.live_game: Optional[LiveGame] = None

    def start_live_game(self):
        if not self.live_game.is_game_over:
            self.live_game.run()

    def run(self):
        if self.live_game is None:
            return
        self.live_game.running = True
        try:
            self.game_thread = Thread(target=self.start_live_game, daemon=True)
            self.game_thread.start()
        except RuntimeError as e:
            print(e)
