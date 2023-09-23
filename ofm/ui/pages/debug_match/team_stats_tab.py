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
from .team_names_component import TeamNamesComponent


class TeamStatsTab(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.team_names = TeamNamesComponent(self)
        self.team_names.grid(row=0, column=0, columnspan=2)
        self.home_team_stats = [
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
        ]
        for row, stat in enumerate(self.home_team_stats):
            stat.grid(row=row + 1, column=0, padx=5, pady=5, sticky=NW)

        self.stats_descriptions = [
            ttk.Label(self, text="Shots"),
            ttk.Label(self, text="Shots on target"),
            ttk.Label(self, text="Possession"),
            ttk.Label(self, text="Passes"),
            ttk.Label(self, text="Pass accuracy"),
            ttk.Label(self, text="Fouls"),
            ttk.Label(self, text="Yellow cards"),
            ttk.Label(self, text="Red cards"),
            ttk.Label(self, text="Offsides"),
            ttk.Label(self, text="Corners"),
        ]
        for row, stat in enumerate(self.stats_descriptions):
            stat.grid(row=row + 1, column=1, padx=5, pady=5, sticky=NS)

        self.away_team_stats = [
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
        ]
        for row, stat in enumerate(self.away_team_stats):
            stat.grid(row=row + 1, column=2, padx=5, pady=5, sticky=NE)

        self.place(anchor=CENTER, relx=0.5, rely=0.5)

    def update_team_names(
        self, home_team: str, away_team: str, home_team_score: str, away_team_score: str
    ):
        self.team_names.update_team_names(home_team, home_team_score, away_team, away_team_score)

    def update_stats(self, home_team_stats: list[int], away_team_stats: list[int]):
        for row, stat in enumerate(self.home_team_stats):
            stat.destroy()
            stat = ttk.Label(self, text=home_team_stats[row])
            stat.grid(row=row + 1, column=0, padx=5, pady=5, sticky=NW)

        for row, stat in enumerate(self.away_team_stats):
            stat.destroy()
            stat = ttk.Label(self, text=away_team_stats[row])
            stat.grid(row=row + 1, column=2, padx=5, pady=5, sticky=NE)
