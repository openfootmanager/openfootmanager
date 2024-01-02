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
import ttkbootstrap as ttk
from ttkbootstrap.constants import *


class SettingsPage(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.title_label = ttk.Label(self, text="Settings", font="Arial 24 bold")
        self.title_label.grid(row=0, column=0, padx=100, pady=45, sticky=NS)

        self.style = ttk.Style()
        self.theme_names = ttk.Style().theme_names()

        self.theme_label = ttk.Label(self, text="Theme: ")
        self.theme_combo_box = ttk.Combobox(self, values=self.theme_names)
        self.theme_combo_box.current(self.theme_names.index(self.style.theme.name))
        self.cancel_btn = ttk.Button(self, text="Cancel")

        self.theme_label.grid(row=1, column=0, padx=10, pady=10, sticky=EW)
        self.theme_combo_box.grid(row=1, column=1, padx=10, pady=10, sticky=EW)
        self.cancel_btn.grid(row=8, column=0, padx=10, pady=10, sticky=EW)
