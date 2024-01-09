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


class DebugHomePage(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.title_label = ttk.Label(self, text="Debug Mode", font="Arial 24 bold")
        self.title_label.grid(row=0, column=0, padx=100, pady=45, sticky=NS)

        self.match_sim_btn = ttk.Button(self, text="Match Simulation")
        self.match_sim_btn.grid(row=1, column=0, padx=10, pady=10, sticky=EW)

        self.team_selection_btn = ttk.Button(self, text="Team Selection")
        self.team_selection_btn.grid(row=2, column=0, padx=10, pady=10, sticky=EW)

        self.team_formation_btn = ttk.Button(self, text="Team Formation")
        self.team_formation_btn.grid(row=3, column=0, padx=10, pady=10, sticky=EW)

        self.championship_btn = ttk.Button(self, text="Championship")
        self.championship_btn.grid(row=4, column=0, padx=10, pady=10, sticky=EW)

        self.stats_explorer_btn = ttk.Button(self, text="Stats Explorer")
        self.stats_explorer_btn.grid(row=5, column=0, padx=10, pady=10, sticky=EW)

        self.player_profile_btn = ttk.Button(self, text="Player Profile")
        self.player_profile_btn.grid(row=6, column=0, padx=10, pady=10, sticky=EW)

        self.team_explorer_btn = ttk.Button(self, text="Team Explorer")
        self.team_explorer_btn.grid(row=7, column=0, padx=10, pady=10, sticky=EW)

        self.cancel_btn = ttk.Button(self, text="Cancel")
        self.cancel_btn.grid(row=8, column=0, padx=10, pady=10, sticky=EW)
