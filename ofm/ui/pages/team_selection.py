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


class TeamSelectionPage(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.title_label = ttk.Label(self, text="Team Selection", font="Arial 24 bold")
        self.title_label.grid(
            row=0, column=0, padx=10, pady=10, columnspan=2, sticky=NS
        )

        self.button_frame = ttk.Frame(self)

        self.select_team_btn = ttk.Button(self.button_frame, text="Select Team")
        self.select_team_btn.pack(side="left", padx=10)

        self.cancel_btn = ttk.Button(self.button_frame, text="Cancel")
        self.cancel_btn.pack(side="left", padx=10)

        self.button_frame.grid(
            row=1, column=0, columnspan=2, padx=10, pady=10, sticky=NS
        )
