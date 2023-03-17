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
import ttkbootstrap as ttk
from ttkbootstrap.constants import *

from ..gui import GUI
from ..pages import HomePage


class HomePageController:
    def __init__(self, gui: GUI):
        self.gui = gui
        self.page: HomePage = gui.pages["home"]
        self._bind()

    def go_to_debug_page(self):
        self.gui.switch("debug_home")

    def _bind(self):
        self.page.debug_mode_btn.config(command=self.go_to_debug_page)
