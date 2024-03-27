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

        self.home_team_data = TeamTableComponent(self)
        self.away_team_data = TeamTableComponent(self)

        self.home_team_data.grid(row=0, column=0)
        self.away_team_data.grid(row=0, column=1)

        self.grid(row=0, column=0)

    def enable_home_team_substitution_button(self):
        self.home_team_data.enable_substitution_button()

    def enable_away_team_substitution_button(self):
        self.away_team_data.enable_substitution_button()

    def disable_home_team_substitution_button(self):
        self.home_team_data.disable_substitution_button()

    def disable_away_team_substitution_button(self):
        self.away_team_data.disable_substitution_button()

    def update_tables(
        self,
        home_team: list[tuple],
        away_team: list[tuple],
    ):
        self.home_team_data.update_table(home_team)
        self.away_team_data.update_table(away_team)

    def update_strategy(self, home_team_strategy: str, away_team_strategy: str):
        self.home_team_data.update_strategy(home_team_strategy)
        self.away_team_data.update_strategy(away_team_strategy)

    def update_formation(self, home_team_formation: str, away_team_formation: str):
        self.home_team_data.update_formation(home_team_formation)
        self.away_team_data.update_formation(away_team_formation)


class TeamTableComponent(ttk.Frame):
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

        self.team_table = Tableview(
            self,
            coldata=self.columns,
            rowdata=home_rows,
            searchable=False,
            autofit=True,
            paginated=False,
            pagesize=8,
            height=11,
        )
        self.team_table.grid(row=0, column=0, padx=10, pady=10, columnspan=2, sticky=EW)

        self.team_formation_value = ttk.StringVar()
        self.team_formation_label = ttk.Label(
            self, text="Formation: ", textvariable=self.team_formation_value
        )
        self.team_formation_label.grid(row=1, column=0, padx=10, pady=10, sticky=EW)

        self.team_strategy_label = ttk.Label(self, text="Strategy: ")
        self.team_strategy = ttk.Combobox(self, values=[""])
        self.team_strategy_label.grid(row=2, column=0, padx=10, pady=10, sticky=EW)
        self.team_strategy.grid(row=2, column=1, padx=10, pady=10, sticky=EW)

        self.substitute_team_value = ttk.BooleanVar()
        self.substitute_team_checkbox = ttk.Checkbutton(
            self,
            text="Manual Substitutions",
            variable=self.substitute_team_value,
            onvalue=True,
            offvalue=False,
        )
        self.substitute_team_checkbox.grid(row=3, column=0, padx=10, pady=10, sticky=EW)
        self.substitute_team_btn = ttk.Button(self, text="Substitute")
        self.substitute_team_btn.grid(
            row=4, column=0, columnspan=2, padx=10, pady=10, sticky=NSEW
        )

    def enable_substitution_button(self):
        self.substitute_team_btn.config(state=NORMAL)

    def disable_substitution_button(self):
        self.substitute_team_btn.config(state=DISABLED)

    def update_formation(self, formation: str):
        self.team_formation_value.set(f"Formation: {formation}")

    def update_table(
        self,
        team: list[tuple],
    ):
        self.team_table.delete_rows()
        self.team_table.insert_rows(END, team)
        self.team_table.autofit_columns()
        self.team_table.load_table_data()

    def update_strategy(self, home_team_strategy: str):
        self.team_strategy.set(home_team_strategy)
