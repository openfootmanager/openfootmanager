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


class TeamNamesComponent(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.home_team_name = ttk.Label(self, text="Brazil", font="Arial 18 bold")
        self.home_team_name.grid(row=0, column=0, padx=10, pady=10, sticky=W)

        self.home_team_score = ttk.Label(self, text="0", font="Arial 18 bold")
        self.home_team_score.grid(row=0, column=1, padx=10, pady=10, sticky=EW)

        self.separator = ttk.Label(self, text="-", font="Arial 18 bold")
        self.separator.grid(row=0, column=2, padx=10, pady=10)

        self.away_team_score = ttk.Label(self, text="0", font="Arial 18 bold")
        self.away_team_score.grid(row=0, column=3, padx=10, pady=10, sticky=EW)

        self.away_team_name = ttk.Label(self, text="Argentina", font="Arial 18 bold")
        self.away_team_name.grid(row=0, column=4, padx=10, pady=10, sticky=E)

    def update_team_names(
        self, home_team_name, home_team_score, away_team_name, away_team_score
    ):
        self.home_team_name.config(text=f"{home_team_name}")
        self.home_team_score.config(text=f"{home_team_score}")

        self.away_team_score.config(text=f"{away_team_score}")
        self.away_team_name.config(text=f"{away_team_name}")
