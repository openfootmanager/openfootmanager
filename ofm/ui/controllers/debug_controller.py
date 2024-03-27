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
from ..pages.debug_home import DebugHomePage
from .controllerinterface import ControllerInterface


class DebugPageController(ControllerInterface):
    def __init__(self, controller: ControllerInterface, page: DebugHomePage):
        self.controller = controller
        self.page = page
        self._bind()

    def switch(self, page: str):
        self.controller.switch(page)

    def initialize(self):
        pass

    def go_to_home_page(self):
        self.switch("home")

    def go_to_match_sim_page(self):
        self.switch("debug_match")

    def go_to_team_selection_page(self):
        self.switch("team_selection")

    def go_to_player_profile_page(self):
        self.switch("player_profile")

    def _bind(self):
        self.page.match_sim_btn.config(command=self.go_to_match_sim_page)
        self.page.cancel_btn.config(command=self.go_to_home_page)
        self.page.team_selection_btn.config(command=self.go_to_team_selection_page)
        self.page.player_profile_btn.config(command=self.go_to_player_profile_page)
