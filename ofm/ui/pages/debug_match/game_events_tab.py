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


class GameEventsTab(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.scores_details = TeamNamesComponent(self)
        self.scores_details.grid(row=0, column=0, columnspan=2)

        self.home_team_events = []
        self.away_team_events = []

        self.place(anchor=CENTER, relx=0.5, rely=0.5)

    def update_team_names(self, home_team_name, home_team_score, away_team_name, away_team_score):
        self.scores_details.update_team_names(home_team_name, home_team_score, away_team_name, away_team_score)

    def update_events(self, home_team_event: list[str], away_team_event: list[str]):
        if self.home_team_events:
            for event in self.home_team_events:
                event.destroy()
            self.home_team_events = []
        if self.away_team_events:
            for event in self.away_team_events:
                event.destroy()
            self.away_team_events = []

        if home_team_event:
            for event in home_team_event:
                self.home_team_events.append(ttk.Label(self, text=event))
            for row, event in enumerate(self.home_team_events):
                event.grid(row=row + 1, column=1, padx=5, pady=5, sticky=NS)
        if away_team_event:
            for event in away_team_event:
                self.away_team_events.append(ttk.Label(self, text=event))

            for row, event in enumerate(self.away_team_events):
                event.grid(row=row + 1, column=0, padx=5, pady=5, sticky=NE)
