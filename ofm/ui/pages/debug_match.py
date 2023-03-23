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
from ttkbootstrap.tableview import Tableview
from ttkbootstrap.constants import *


class DebugMatchPage(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.title_label = ttk.Label(self, text="Debug Match")
        self.title_label.grid(row=0, column=0, padx=10, pady=10, columnspan=3, sticky=NS)

        columns = [
            {"text": "Name", "stretch": False},
            {"text": "Position", "stretch": False},
            {"text": "Stamina", "stretch": False}
        ]

        home_rows = [
            ("Gomez", "FW", "100"),
            ("Allejo", "FW", "100"),
            ("Beranco", "MF", "100"),
            ("Pardilla", "MF", "100"),
            ("Santos", "MF", "100"),
            ("Ferreira", "MF", "100"),
            ("Roca", "DF", "100"),
            ("Vincento", "DF", "100"),
            ("Cicero", "DF", "100"),
            ("Marengez", "DF", "100"),
            ("Da Silva", "GK", "100"),
        ]

        away_rows = [
            ("Estrade", "FW", "100"),
            ("Capitale", "FW", "100"),
            ("Hajo", "MF", "100"),
            ("Redonda", "MF", "100"),
            ("Vasquez", "MF", "100"),
            ("Santos", "MF", "100"),
            ("Basile", "DF", "100"),
            ("Morelli", "DF", "100"),
            ("Costa", "DF", "100"),
            ("Alerto", "DF", "100"),
            ("Pablo", "GK", "100"),
        ]

        self.home_team_score = ttk.Label(self, text="Brazil\t0")
        self.home_team_score.grid(row=1, column=0,  padx=10, pady=10, sticky=E)

        self.away_team_score = ttk.Label(self, text="0\tArgentina")
        self.away_team_score.grid(row=1, column=1, padx=10, pady=10, sticky=W)

        self.home_team_table = Tableview(self, coldata=columns, rowdata=home_rows, searchable=False, autofit=True,
                                         paginated=False, pagesize=8, height=11)
        self.home_team_table.grid(row=2, column=0, padx=10, pady=10, sticky=EW)

        self.away_team_table = Tableview(self, coldata=columns, rowdata=away_rows, searchable=False, autofit=True, paginated=False, pagesize=8, height=11)
        self.away_team_table.grid(row=2, column=1, padx=10, pady=10, sticky=EW)

        self.game_progress_bar = ttk.Progressbar(self, value=50, bootstyle="striped")
        self.game_progress_bar.grid(row=3, column=0, columnspan=2, padx=10, pady=10, sticky=EW)

        self.game_minutes_elapsed = ttk.Label(self, text="0'")
        self.game_minutes_elapsed.grid(row=4, column=0, columnspan=2, padx=10, pady=10, sticky=NS)

        self.play_game_btn = ttk.Button(self, text="Play")
        self.play_game_btn.grid(row=5, column=0, padx=10, pady=50, sticky=EW)

        self.new_game_btn = ttk.Button(self, text="New Game")
        self.new_game_btn.grid(row=5, column=1, padx=10, pady=50, sticky=EW)

        self.cancel_btn = ttk.Button(self, text="Cancel")
        self.cancel_btn.grid(row=5, column=2, padx=10, pady=50, sticky=EW)
        