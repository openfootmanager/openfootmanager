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

from .live_game_tab import LiveGameTab
from .player_details_tab import PlayerDetailsTab
from .team_stats_tab import TeamStatsTab
from .game_events_tab import GameEventsTab


class DebugMatchPage(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.notebook = ttk.Notebook(self)

        self.player_details_tab = PlayerDetailsTab(self.notebook)
        self.player_reserves_tab = PlayerDetailsTab(self.notebook)
        self.team_stats_tab = TeamStatsTab(self.notebook)
        self.live_game_tab = LiveGameTab(self.notebook)
        self.game_events_tab = GameEventsTab(self.notebook)

        self.title_label = ttk.Label(self, text="Debug Match", font="Arial 24 bold")
        self.title_label.grid(
            row=0, column=0, padx=10, pady=10, columnspan=3, sticky=NS
        )

        self.button_frame = ttk.Frame(self)

        self.play_game_btn = ttk.Button(self.button_frame, text="Play")
        self.play_game_btn.pack(side="left", padx=10)

        self.new_game_btn = ttk.Button(self.button_frame, text="New Game")
        self.new_game_btn.pack(side="left", padx=10)

        self.cancel_btn = ttk.Button(self.button_frame, text="Cancel")
        self.cancel_btn.pack(side="left", padx=10)

        self.button_frame.grid(
            row=4, column=0, columnspan=2, padx=10, pady=10, sticky=NS
        )

        self.player_details_tab.place(anchor=CENTER, relx=0.5, rely=0.5)
        self.player_reserves_tab.place(anchor=CENTER, relx=0.5, rely=0.5)
        self.team_stats_tab.place(anchor=CENTER, relx=0.5, rely=0.5)
        self.live_game_tab.place(anchor=CENTER, relx=0.5, rely=0.5)

        self.notebook.add(self.live_game_tab, text="Live Game", sticky=NS)
        self.notebook.add(self.game_events_tab, text="Events", sticky=NS)
        self.notebook.add(self.player_details_tab, text="Players", sticky=NS)
        self.notebook.add(self.player_reserves_tab, text="Reserves", sticky=NS)
        self.notebook.add(self.team_stats_tab, text="Stats", sticky=NS)

        self.notebook.grid(row=1, column=0, sticky=NSEW)

    def update_tables(
        self,
        home_team: list[tuple],
        away_team: list[tuple],
        home_reserves: list[tuple],
        away_reserves: list[tuple],
    ):
        self.player_details_tab.update_tables(home_team, away_team)
        self.player_reserves_tab.update_tables(home_reserves, away_reserves)

    def disable_button(self):
        self.play_game_btn.config(state=ttk.DISABLED)

    def enable_button(self):
        self.play_game_btn.config(state=ttk.NORMAL)

    def update_team_names(
        self, home_team: str, away_team: str, home_team_score: str, away_team_score: str
    ):
        self.player_details_tab.update_team_names(home_team, away_team, home_team_score, away_team_score)
        self.player_reserves_tab.update_team_names(home_team, away_team, home_team_score, away_team_score)
        self.team_stats_tab.update_team_names(home_team, away_team, home_team_score, away_team_score)
        self.live_game_tab.update_team_names(home_team, home_team_score, away_team, away_team_score)
        self.game_events_tab.update_team_names(home_team, home_team_score, away_team, away_team_score)

    def update_team_stats(self, home_team_stats: list[int], away_team_stats: list[int]):
        self.team_stats_tab.update_stats(home_team_stats, away_team_stats)
