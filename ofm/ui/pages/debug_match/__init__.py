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
from enum import Enum
from typing import Callable

import ttkbootstrap as ttk
from ttkbootstrap.constants import *

from .game_events_tab import GameEventsTab
from .live_game_tab import LiveGameTab
from .player_details_tab import PlayerDetailsTab
from .substitution_window import SubstitutionWindow
from .team_names_component import TeamNamesComponent
from .team_stats_tab import TeamStatsTab


class DelayComboBoxValues(Enum):
    NONE = "None"
    SHORT = "Short (0.005s)"
    MEDIUM = "Medium (0.01s)"
    LONG = "Long (0.1s)"
    VERY_LONG = "Very Long (1s)"


class CommentaryVerbosity(Enum):
    ALL = "All Events"
    HIGHLIGHTS = "Highlights"
    SHOTS_ONLY = "Shots and Goals Only"


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
        self.title_label.grid(row=0, column=0, padx=10, pady=10, columnspan=3)

        self.scores_details = TeamNamesComponent(self)
        self.scores_details.grid(row=1, column=0, columnspan=2)

        self.progress_bar = ttk.Progressbar(
            self, length=550, maximum=90 * 60, bootstyle="striped"
        )
        self.progress_bar.grid(row=3, column=0, columnspan=2, pady=20, sticky=NSEW)

        self.minutes_elapsed = ttk.Label(self, text="0'")
        self.minutes_elapsed.grid(row=3, column=2, padx=15, pady=20, sticky=NSEW)

        self.delay_label = ttk.Label(self, text="Simulation delay:")
        self.delay_label.grid(row=4, column=0, padx=5, pady=5, sticky=W)

        self.delay_box = ttk.Combobox(
            self,
            values=list(x.value for x in DelayComboBoxValues),
        )
        self.delay_box.set(DelayComboBoxValues.NONE.value)
        self.delay_box.grid(row=4, column=1, padx=5, pady=5, sticky=NSEW)

        self.commentary_label = ttk.Label(self, text="Commentary verbosity:")
        self.commentary_label.grid(row=5, column=0, padx=5, pady=5, sticky=W)
        self.commentary_box = ttk.Combobox(
            self,
            values=list(x.value for x in CommentaryVerbosity),
        )
        self.commentary_box.set(CommentaryVerbosity.ALL.value)
        self.commentary_box.grid(row=5, column=1, padx=5, pady=5, sticky=NSEW)

        self.button_frame = ttk.Frame(self)

        self.play_game_btn = ttk.Button(self.button_frame, text="Play")
        self.play_game_btn.grid(row=0, column=1, padx=10, sticky=NSEW)

        self.new_game_btn = ttk.Button(self.button_frame, text="New Game")
        self.new_game_btn.grid(row=0, column=2, padx=10, sticky=NSEW)

        self.cancel_btn = ttk.Button(self.button_frame, text="Cancel")
        self.cancel_btn.grid(row=0, column=3, padx=10, sticky=NSEW)

        self.button_frame.grid(row=6, column=0, columnspan=3, padx=10, pady=10)

        self.player_details_tab.place(anchor=CENTER, relx=0.5, rely=0.5)
        self.player_reserves_tab.place(anchor=CENTER, relx=0.5, rely=0.5)
        self.team_stats_tab.place(anchor=CENTER, relx=0.5, rely=0.5)
        self.live_game_tab.place(anchor=CENTER, relx=0.5, rely=0.5)

        self.notebook.add(self.live_game_tab, text="Live Game", sticky=NS)
        self.notebook.add(self.game_events_tab, text="Events", sticky=NS)
        self.notebook.add(self.player_details_tab, text="Players", sticky=NS)
        self.notebook.add(self.player_reserves_tab, text="Reserves", sticky=NS)
        self.notebook.add(self.team_stats_tab, text="Stats", sticky=NS)

        self.notebook.grid(row=2, column=0, columnspan=2)

    def update_tables(
        self,
        home_team: list[tuple],
        away_team: list[tuple],
        home_reserves: list[tuple],
        away_reserves: list[tuple],
    ):
        self.player_details_tab.update_tables(home_team, away_team)
        self.player_reserves_tab.update_tables(home_reserves, away_reserves)

    def change_play_button_to_pause(self, func: Callable):
        self.play_game_btn.config(text="Pause", command=func)

    def change_pause_button_to_play(self, func: Callable):
        self.play_game_btn.config(text="Play", command=func)

    def update_team_names(
        self, home_team: str, away_team: str, home_team_score: str, away_team_score: str
    ):
        self.scores_details.update_team_names(
            home_team, home_team_score, away_team, away_team_score
        )

    def update_team_stats(self, home_team_stats: list[int], away_team_stats: list[int]):
        self.team_stats_tab.update_stats(home_team_stats, away_team_stats)

    def update_live_game(self, live_game_events: list[str]):
        self.live_game_tab.update_live_game_events(live_game_events)

    def update_game_events(
        self, home_team_events: list[str], away_team_events: list[str]
    ):
        self.game_events_tab.update_events(home_team_events, away_team_events)

    def update_game_progress(self, minutes_elapsed: int):
        self.progress_bar["value"] = minutes_elapsed
        self.minutes_elapsed.config(text=str(minutes_elapsed) + "'")

    def update_team_strategy(self, home_team_strategy: str, away_team_strategy: str):
        self.player_details_tab.update_strategy(home_team_strategy, away_team_strategy)

    def update_team_formation(self, home_team_formation: str, away_team_formation: str):
        self.player_details_tab.update_formation(
            home_team_formation, away_team_formation
        )
