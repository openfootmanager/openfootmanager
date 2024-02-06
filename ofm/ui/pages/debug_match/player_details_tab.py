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
from ttkbootstrap.tableview import Tableview


class PlayerDetailsTab(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.columns = [
            {"text": "Name", "stretch": False},
            {"text": "Position", "stretch": False},
            {"text": "Stamina", "stretch": False},
            {"text": "Injured", "stretch": False},
            {"text": "Overall", "stretch": False},
        ]

        home_rows = [
            ("Gomez", "FW", "100", "No", "89"),
            ("Allejo", "FW", "100", "No", "95"),
            ("Beranco", "MF", "100", "No", "85"),
            ("Pardilla", "MF", "100", "No", "83"),
            ("Santos", "MF", "100", "No", "80"),
            ("Ferreira", "MF", "100", "No", "87"),
            ("Roca", "DF", "100", "No", "86"),
            ("Vincento", "DF", "100", "No", "84"),
            ("Cicero", "DF", "100", "No", "90"),
            ("Marengez", "DF", "100", "No", "88"),
            ("Da Silva", "GK", "100", "No", "92"),
        ]

        away_rows = [
            ("Estrade", "FW", "100", "No", "84"),
            ("Capitale", "FW", "100", "No", "83"),
            ("Hajo", "MF", "100", "No", "90"),
            ("Redonda", "MF", "100", "No", "87"),
            ("Vasquez", "MF", "100", "No", "80"),
            ("Santos", "MF", "100", "No", "81"),
            ("Basile", "DF", "100", "No", "83"),
            ("Morelli", "DF", "100", "No", "82"),
            ("Costa", "DF", "100", "No", "91"),
            ("Alerto", "DF", "100", "No", "84"),
            ("Pablo", "GK", "100", "No", "87"),
        ]

        self.home_team_table = Tableview(
            self,
            coldata=self.columns,
            rowdata=home_rows,
            searchable=False,
            autofit=True,
            paginated=False,
            pagesize=8,
            height=11,
        )
        self.home_team_table.grid(
            row=0, column=0, padx=10, pady=10, columnspan=2, sticky=EW
        )
        self.home_team_strategy_label = ttk.Label(self, text="Strategy: ")
        self.home_team_strategy = ttk.Label(self, text="")
        self.home_team_strategy_label.grid(row=1, column=0, padx=10, pady=10, sticky=EW)
        self.home_team_strategy.grid(row=1, column=1, padx=10, pady=10, sticky=EW)

        self.away_team_table = Tableview(
            self,
            coldata=self.columns,
            rowdata=away_rows,
            searchable=False,
            autofit=True,
            paginated=False,
            pagesize=8,
            height=11,
        )
        self.away_team_table.grid(
            row=0, column=2, padx=10, pady=10, columnspan=2, sticky=EW
        )
        self.away_team_strategy_label = ttk.Label(self, text="Strategy: ")
        self.away_team_strategy = ttk.Label(self, text="")
        self.away_team_strategy_label.grid(row=1, column=2, padx=10, pady=10, sticky=EW)
        self.away_team_strategy.grid(row=1, column=3, padx=10, pady=10, sticky=EW)

        self.grid(row=0, column=0)

    def update_tables(
        self,
        home_team: list[tuple],
        away_team: list[tuple],
    ):
        self.home_team_table.delete_rows()
        self.home_team_table.insert_rows(END, home_team)
        self.home_team_table.autofit_columns()
        self.home_team_table.load_table_data()

        self.away_team_table.delete_rows()
        self.away_team_table.insert_rows(END, away_team)
        self.away_team_table.autofit_columns()
        self.away_team_table.load_table_data()

    def update_strategy(self, home_team_strategy: str, away_team_strategy: str):
        self.home_team_strategy.config(text=home_team_strategy)
        self.away_team_strategy.config(text=away_team_strategy)
