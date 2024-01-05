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


class PlayerProfilePage(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.player_image = ttk.PhotoImage(file="ofm/res/images/placeholder.png")

        self.player_image = self.player_image.subsample(3, 3)

        self.canvas = ttk.Canvas(self)
        self.canvas.create_image(0, 0, image=self.player_image, anchor=NW)
        self.canvas.grid(row=0, column=0, padx=20, pady=20, sticky=NSEW)

        self.player_name_label = ttk.Label(self, text="Full Player Name", font=("Helvetica", 20))
        self.player_name_label.grid(row=0, column=1, padx=20, pady=20, sticky=NSEW)

        self.player_birth_date_label = ttk.Label(self, text="Birth date: ", font=("Helvetica", 12))
        self.player_birth_date_label.grid(row=1, column=0, padx=20, pady=10, sticky=NSEW)

        self.player_birth_date_value = ttk.Label(self, text="01-01-2020", font=("Helvetica", 12))
        self.player_birth_date_value.grid(row=1, column=1, padx=10, pady=10, sticky=NSEW)

        self.player_nationality_label = ttk.Label(self, text="Nationality: ", font=("Helvetica", 12))
        self.player_nationality_label.grid(row=2, column=0, padx=20, pady=10, sticky=NSEW)

        self.player_nationality_value = ttk.Label(self, text="Brazil", font=("Helvetica", 12))
        self.player_nationality_value.grid(row=2, column=1, padx=10, pady=10, sticky=NSEW)

        self.cancel_btn = ttk.Button(self, text="Cancel")
        self.cancel_btn.grid(row=3, column=0, padx=20, pady=10, sticky=NSEW)

