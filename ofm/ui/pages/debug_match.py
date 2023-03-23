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

        self.title_label = ttk.Label(self, text="Debug Match", font='Arial 24 bold')
        self.title_label.grid(row=0, column=0, padx=10, pady=10, columnspan=3, sticky=NS)

        columns = [
            {"text": "Name", "stretch": False},
            {"text": "Position", "stretch": False},
            {"text": "Stamina", "stretch": False},
            {"text": "Overall", "stretch": False},
        ]

        home_rows = [
            ("Gomez", "FW", "100", "89"),
            ("Allejo", "FW", "100", "95"),
            ("Beranco", "MF", "100", "85"),
            ("Pardilla", "MF", "100", "83"),
            ("Santos", "MF", "100", "80"),
            ("Ferreira", "MF", "100", "87"),
            ("Roca", "DF", "100", "86"),
            ("Vincento", "DF", "100", "84"),
            ("Cicero", "DF", "100", "90"),
            ("Marengez", "DF", "100", "88"),
            ("Da Silva", "GK", "100", "92"),
        ]

        away_rows = [
            ("Estrade", "FW", "100", "84"),
            ("Capitale", "FW", "100", "83"),
            ("Hajo", "MF", "100", "90"),
            ("Redonda", "MF", "100", "87"),
            ("Vasquez", "MF", "100", "80"),
            ("Santos", "MF", "100", "81"),
            ("Basile", "DF", "100", "83"),
            ("Morelli", "DF", "100", "82"),
            ("Costa", "DF", "100", "91"),
            ("Alerto", "DF", "100", "84"),
            ("Pablo", "GK", "100", "87"),
        ]

        self.home_team_score = ttk.Label(self, text="Brazil\t0", font='Arial 15 bold')
        self.home_team_score.grid(row=1, column=0,  padx=10, pady=10, sticky=E)

        self.away_team_score = ttk.Label(self, text="0\tArgentina", font='Arial 15 bold')
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

        self.button_frame = ttk.Frame(self)

        self.play_game_btn = ttk.Button(self.button_frame, text="Play")
        self.play_game_btn.pack(side="left", padx=10, fill="x")

        self.new_game_btn = ttk.Button(self.button_frame, text="New Game")
        self.new_game_btn.pack(side="left", padx=10, fill="x")

        self.cancel_btn = ttk.Button(self.button_frame, text="Cancel")
        self.cancel_btn.pack(side="left", padx=10, fill="both")

        self.button_frame.grid(row=5, column=0, columnspan=2, padx=10, pady=10, sticky=NS)
