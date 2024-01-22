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

from .controllerinterface import ControllerInterface
from ..pages import PlayerProfilePage


class PlayerProfilePageController(ControllerInterface):
    def __init__(self, controller: ControllerInterface, page: PlayerProfilePage):
        self.controller = controller
        self.page = page
        self._bind()

    def switch(self, page):
        self.controller.switch(page)

    def initialize(self):
        pass

    def go_to_debug_page(self):
        self.switch("debug_home")

    def _bind(self):
        self.page.cancel_btn.config(command=self.go_to_debug_page)
