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
from ...core.db.database import DB
from ...core.settings import Settings
from ..gui import GUI
from .controllerinterface import ControllerInterface
from .debug_controller import DebugPageController
from .debug_match_controller import DebugMatchController
from .home_controller import HomePageController
from .player_profile_controller import PlayerProfilePageController
from .settings_controller import SettingsController
from .team_selection_controller import TeamSelectionController


class OFMController(ControllerInterface):
    """
    Main controller that groups the OFM controllers
    """

    def __init__(self, settings: Settings, db: DB):
        self.settings = settings
        self.db = db
        self.gui = GUI()
        self.controllers = {
            "home": HomePageController(self, self.gui.pages["home"]),
            "debug_home": DebugPageController(self, self.gui.pages["debug_home"]),
            "debug_match": DebugMatchController(
                self,
                self.gui.pages["debug_match"],
                self.db,
            ),
            "team_selection": TeamSelectionController(
                self, self.gui.pages["team_selection"]
            ),
            "settings": SettingsController(self, self.gui.pages["settings"]),
            "player_profile": PlayerProfilePageController(
                self, self.gui.pages["player_profile"]
            ),
        }

    def initialize(self):
        pass

    def _bind(self):
        pass

    def switch(self, page: str):
        self.gui.switch(page)
        self.controllers[page].initialize()

    def run(self):
        self.switch("home")
        self.gui.window.mainloop()
