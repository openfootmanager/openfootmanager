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


class GUI:
    def __init__(self):
        self.window = ttk.Window(title="OpenFoot Manager", themename="cosmo")

        width = 800
        height = 600

        self.window.geometry(f"{width}x{height}")
        self.window.minsize(width, height)

        self.window.rowconfigure(0, weight=1)
        self.window.columnconfigure(0, weight=1)

        self.pages = {
            "home": self._add_frame(HomePage),
            "debug_home": self._add_frame(DebugHomePage),
        }

        self.current_page = None

    def _add_frame(self, frame) -> ttk.Frame:
        f = frame(self.window)
        f.grid(row=0, column=0, sticky=EW)
        return f

    def switch(self, name: str):
        self.current_page = self.pages[name]
        self.current_page.tkraise()
