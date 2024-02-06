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


class TeamStatsTab(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

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
            stat.grid(row=row, column=0, padx=10, pady=5, sticky=NW)

        self.stats_descriptions = [
            ttk.Label(self, text="Shots"),
            ttk.Label(self, text="Shots on target"),
            ttk.Label(self, text="Possession"),
            ttk.Label(self, text="Passes"),
            ttk.Label(self, text="Pass accuracy"),
            ttk.Label(self, text="Crosses"),
            ttk.Label(self, text="Cross accuracy"),
            ttk.Label(self, text="Fouls"),
            ttk.Label(self, text="Yellow cards"),
            ttk.Label(self, text="Red cards"),
            ttk.Label(self, text="Offsides"),
            ttk.Label(self, text="Corners"),
        ]
        for row, stat in enumerate(self.stats_descriptions):
            stat.grid(row=row, column=1, padx=70, pady=5, sticky=NS)

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
            ttk.Label(self, text="0"),
            ttk.Label(self, text="0"),
        ]
        for row, stat in enumerate(self.away_team_stats):
            stat.grid(row=row, column=2, padx=10, pady=5, sticky=NE)

        self.grid(row=0, column=0)

    def update_stats(self, home_team_stats: list[int], away_team_stats: list[int]):
        for row, stat in enumerate(self.home_team_stats):
            stat.destroy()

        for row, stat in enumerate(self.away_team_stats):
            stat.destroy()

        self.home_team_stats.clear()
        self.away_team_stats.clear()

        for row, _ in enumerate(self.stats_descriptions):
            home_stat = ttk.Label(self, text=home_team_stats[row])
            away_stat = ttk.Label(self, text=away_team_stats[row])
            self.home_team_stats.append(home_stat)
            self.away_team_stats.append(away_stat)
            home_stat.grid(row=row, column=0, padx=5, pady=5, sticky=NW)
            away_stat.grid(row=row, column=2, padx=5, pady=5, sticky=NE)
