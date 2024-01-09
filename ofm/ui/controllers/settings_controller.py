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
from ..pages.settings import SettingsPage
from .controllerinterface import ControllerInterface


class SettingsController(ControllerInterface):
    def __init__(self, controller: ControllerInterface, page: SettingsPage):
        self.controller = controller
        self.page = page
        self._bind()

    def initialize(self):
        pass

    def switch(self, page):
        self.controller.switch(page)

    def select_theme(self, e):
        theme = self.page.theme_combo_box.get()
        self.controller.gui.style.theme_use(theme)
        self.page.theme_combo_box.selection_clear()

    def go_to_debug_home_page(self):
        self.switch("home")

    def _bind(self):
        self.page.cancel_btn.config(command=self.go_to_debug_home_page)
        self.page.theme_combo_box.bind("<<ComboboxSelected>>", self.select_theme)
