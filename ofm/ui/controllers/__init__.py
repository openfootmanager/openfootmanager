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
from ofm.core.settings import Settings
from ofm.core.db.database import DB
from ofm.ui.gui import GUI
from .home_controller import HomePageController
from .debug_controller import DebugPageController


class OFMController:
    """
    Main controller that groups the OFM controllers
    """
    def __init__(self, settings: Settings, db: DB):
        self.settings = settings
        self.db = db
        self.gui = GUI()
        self.controllers = {
            'home': HomePageController(self.gui, self.gui.pages["home"]),
            'debug_home': DebugPageController(self.gui, self.gui.pages["debug_home"]),
        }

    def run(self):
        self.gui.switch('home')
        self.gui.window.mainloop()
