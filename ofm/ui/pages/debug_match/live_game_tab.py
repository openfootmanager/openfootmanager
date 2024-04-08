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


class LiveGameTab(ttk.Frame):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.live_game_events = ttk.ScrolledText(self, height=15)
        self.live_game_events.config(state=DISABLED)
        self.live_game_events.grid(row=0, column=0, padx=10, pady=10)

        self.grid(row=0, column=0)

    def update_live_game_events(self, game_events: list[str]):
        self.live_game_events.config(state="normal")
        text = ""
        if game_events:
            for event in game_events:
                text = text + event + "\n"

        self.live_game_events.delete(1.0, END)
        self.live_game_events.insert(ttk.END, text)
        self.live_game_events.see(ttk.END)
        self.live_game_events.config(state=DISABLED)
