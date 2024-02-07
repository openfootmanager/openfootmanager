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
from ttkbootstrap.dialogs.dialogs import Messagebox, MessageCatalog
from ttkbootstrap.tableview import Tableview


class SubstitutionWindow(ttk.Toplevel):
    def __init__(self, parent):
        super().__init__(parent)
        self.parent = parent
        self.wm_title("Substitute Players")
        self.resizable(False, False)
        self.geometry("")
        self.grab_set()

        self.main_frame = ttk.Frame(self)

        self.team_name_variable = ttk.StringVar()
        self.team_name = ttk.Label(
            self.main_frame,
            text="TeamName",
            font="Arial 18 bold",
            textvariable=self.team_name_variable,
        )

        self.columns = [
            {"text": "Name", "stretch": False},
            {"text": "Position", "stretch": False},
            {"text": "Stamina", "stretch": False},
            {"text": "Injured", "stretch": False},
            {"text": "Overall", "stretch": False},
        ]

        rows = [
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
            self.main_frame,
            coldata=self.columns,
            rowdata=rows,
            searchable=False,
            autofit=True,
            paginated=False,
            pagesize=8,
            height=11,
        )
        self.formation_label = ttk.Label(self.main_frame, text="Formation: ")
        self.formation_combobox = ttk.Combobox(self.main_frame, values=["4-4-2"])

        self.button_in = ttk.Button(self.main_frame, text=">")
        self.button_out = ttk.Button(self.main_frame, text="<")

        self.reserves_table = Tableview(
            self.main_frame,
            coldata=self.columns,
            rowdata=rows,
            searchable=False,
            autofit=True,
            paginated=False,
            pagesize=8,
            height=11,
        )
        self.substitutions_left_label = ttk.Label(
            self.main_frame, text="Substitutions left: 0"
        )

        self.team_name.grid(row=0, column=0, columnspan=3, padx=10, pady=10, sticky=NS)
        self.team_table.grid(row=1, rowspan=2, column=0, padx=10, pady=10, sticky=NSEW)
        self.button_in.grid(row=1, column=1, padx=10, pady=10, sticky=NS)
        self.button_out.grid(row=2, column=1, padx=10, pady=10, sticky=NS)
        self.reserves_table.grid(
            row=1, rowspan=2, column=2, padx=10, pady=10, sticky=NSEW
        )
        self.formation_label.grid(row=3, column=0, padx=10, pady=10, sticky=NSEW)
        self.formation_combobox.grid(
            row=3, column=1, columnspan=2, padx=10, pady=10, sticky=NSEW
        )
        self.substitutions_left_label.grid(
            row=4, column=0, columnspan=3, padx=10, pady=10, sticky=NSEW
        )

        self.button_frame = ttk.Frame(self)
        self.apply_button = ttk.Button(self.button_frame, text="Apply")
        self.cancel_button = ttk.Button(self.button_frame, text="Cancel")

        self.button_frame.grid(
            row=5, column=0, columnspan=3, padx=10, pady=10, sticky=NS
        )
        self.apply_button.grid(row=0, column=0, padx=10, pady=10, sticky=EW)
        self.cancel_button.grid(row=0, column=1, padx=10, pady=10, sticky=EW)

        self.main_frame.grid(row=0, column=0, sticky=NSEW)

    def update_formations(self, formations: list[str]):
        self.formation_combobox["values"] = formations

    def cancel_dialog(self):
        return Messagebox.yesno(
            parent=self,
            title="Cancel formation",
            message="Are you sure you want to cancel the change? All changes will be lost.",
            alert=True,
        )

    def update_team_table(self, players: list[tuple]):
        self.team_table.delete_rows()
        self.team_table.insert_rows(END, players)
        self.team_table.autofit_columns()
        self.team_table.load_table_data()

    def update_reserves_table(self, players: list[tuple]):
        self.reserves_table.delete_rows()
        self.reserves_table.insert_rows(END, players)
        self.reserves_table.autofit_columns()
        self.reserves_table.load_table_data()

    def get_yes_result(self):
        return MessageCatalog.translate("Yes")

    def get_no_result(self):
        return MessageCatalog.translate("No")

    def update_team_name(self, team_name: str):
        self.team_name_variable.set(team_name)
