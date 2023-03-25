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
from .pages import *


FOOTBALL_THEME = {
    "football": {
        "type": "light",
        "colors": {
            "primary": "#56c2ad",
            "secondary": "#6bc49a",
            "success": "#29cc24",
            "info": "#5968d5",
            "warning": "#666e67",
            "danger": "#007851",
            "light": "#f1fffc",
            "dark": "#514c50",
            "bg": "#ebffd9",
            "fg": "#5a5a5a",
            "selectbg": "#56c2ad",
            "selectfg": "#f1ffee",
            "border": "#ced4da",
            "inputfg": "#696969",
            "inputbg": "#f1ffee",
            "active": "#e5e5e5"
        }
    }
}


class GUI:
    def __init__(self):
        self.window = ttk.Window(title="OpenFoot Manager", themename="minty")

        width = 800
        height = 600

        self.window.geometry(f"{width}x{height}")
        self.window.minsize(width, height)

        self.window.rowconfigure(0, weight=1)
        self.window.columnconfigure(0, weight=1)

        self.pages = {
            "home": self._add_frame(HomePage),
            "debug_home": self._add_frame(DebugHomePage),
            "debug_match": self._add_frame(DebugMatchPage),
            "team_selection": self._add_frame(TeamSelectionPage)
        }

        self.current_page = self.pages["home"]

    def _add_frame(self, frame) -> ttk.Frame:
        f = frame(self.window)
        f.place(anchor=CENTER, relx=.5, rely=.5)
        # Necessary to hide all frames
        f.place_forget()
        return f

    def switch(self, name: str):
        self.current_page.place_forget()
        self.current_page = self.pages[name]
        self.current_page.place(anchor=CENTER, relx=.5, rely=.5)
        self.current_page.tkraise()
        
