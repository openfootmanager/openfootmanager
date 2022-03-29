#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2022  Pedrenrique G. Guimarães
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
"""
This file is totally inspired by the implementations of live game events,
structs and enums from Bygfoot's source code, with minor modifications
to comply with Python's syntax.

This is just to get a headstart with the project, and some enums
and classes might be removed in the future.
"""
from uuid import UUID
from dataclasses import dataclass, field
from typing import Union

from ofm.core.data.team import Team
from ofm.core.live_game.events import EventHandler


@dataclass
class LiveGameStats:
    goals_regular: int = 0
    shots: int = 0
    shot_percentage: float = 0.0
    possession: float = 0.0
    penalties: int = 0
    fouls: int = 0
    cards: int = 0
    reds: int = 0
    injuries: int = 0


@dataclass
class LiveGameTeamState:
    structure: int
    style: int


@dataclass
class Match:
    championship_id: Union[int, UUID]
    match_id: Union[int, UUID]
    championship_round: int
    replay_number: int
    week_number: int
    week_round_number: int
    teams: list[Team]
    result: list[int]
    home_advantage: bool
    second_leg: bool
    decisive: bool
    attendance: int


@dataclass
class LiveGame:
    match: Match
    verbosity: int
    stats = LiveGameStats()
    started_game: bool = False
    minutes: float = 0.0
    eventhandler: EventHandler = EventHandler()
    units: list = field(default_factory=list)
    running: bool = False

    def play_live_game(self):
        self.running = True
        while self.running:
            self.eventhandler.create_event_unit(
                minutes=self.minutes,
                verbosity=self.verbosity,

            )
