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
import random

from enum import Enum, auto
from dataclasses import dataclass, field
from typing import Union
from uuid import UUID

from ofm.core.data.team import Team
from ofm.core.data.player import Player


class LiveGameEventType(Enum):
    LIVE_GAME_EVENT_GENERAL = 0
    LIVE_GAME_EVENT_START_MATCH = auto()
    LIVE_GAME_EVENT_HALF_TIME = auto()
    LIVE_GAME_EVENT_EXTRA_TIME = auto()
    LIVE_GAME_EVENT_END_MATCH = auto()
    LIVE_GAME_EVENT_LOST_POSSESSION = auto()
    LIVE_GAME_EVENT_SCORING_CHANCE = auto()
    LIVE_GAME_EVENT_HEADER = auto()
    LIVE_GAME_EVENT_PENALTY = auto()
    LIVE_GAME_EVENT_FREE_KICK = auto()
    LIVE_GAME_EVENT_GOAL = auto()
    LIVE_GAME_EVENT_OWN_GOAL = auto()
    LIVE_GAME_EVENT_POST = auto()
    LIVE_GAME_EVENT_MISS = auto()
    LIVE_GAME_EVENT_SAVE = auto()
    LIVE_GAME_EVENT_KEEPER_PUSHED_IN_CORNER = auto()
    LIVE_GAME_EVENT_CROSS_BAR = auto()
    LIVE_GAME_EVENT_PLAYER_PUSHED_IN_CORNER = auto()
    LIVE_GAME_EVENT_CORNER_KICK = auto()
    LIVE_GAME_EVENT_FOUL = auto()
    LIVE_GAME_EVENT_FOUL_YELLOW = auto()
    LIVE_GAME_EVENT_FOUL_RED = auto()
    LIVE_GAME_EVENT_FOUL_RED_INJURY = auto()
    LIVE_GAME_EVENT_SEND_OFF = auto()
    LIVE_GAME_EVENT_INJURY = auto()
    LIVE_GAME_EVENT_TEMP_INJURY = auto()
    LIVE_GAME_EVENT_PENALTIES = auto()
    LIVE_GAME_EVENT_STADIUM = auto()
    LIVE_GAME_EVENT_STADIUM_BREAKDOWN = auto()
    LIVE_GAME_EVENT_STADIUM_RIOTS = auto()
    LIVE_GAME_EVENT_STADIUM_FIRE = auto()
    LIVE_GAME_EVENT_SUBSTITUTION = auto()
    LIVE_GAME_EVENT_STRUCTURE_CHANGE = auto()
    LIVE_GAME_EVENT_STYLE_CHANGE_ALL_OUT_DEFEND = auto()
    LIVE_GAME_EVENT_STYLE_CHANGE_DEFEND = auto()
    LIVE_GAME_EVENT_STYLE_CHANGE_BALANCED = auto()
    LIVE_GAME_EVENT_STYLE_CHANGE_ATTACK = auto()
    LIVE_GAME_EVENT_STYLE_CHANGE_ALL_OUT_ATTACK = auto()
    LIVE_GAME_EVENT_BOOST_CHANGE_ANTI = auto()
    LIVE_GAME_EVENT_BOOST_CHANGE_OFF = auto()
    LIVE_GAME_EVENT_BOOST_CHANGE_ON = auto()


class LiveGameUnitArea(Enum):
    LIVE_GAME_UNIT_AREA_DEFEND = 0
    LIVE_GAME_UNIT_AREA_MIDFIELD = auto()
    LIVE_GAME_UNIT_AREA_ATTACK = auto()


class LiveGameUnitTime(Enum):
    LIVE_GAME_UNIT_TIME_FIRST_HALF = 0
    LIVE_GAME_UNIT_TIME_SECOND_HALF = auto()
    LIVE_GAME_UNIT_TIME_EXTRA_TIME = auto()
    LIVE_GAME_UNIT_TIME_PENALTIES = auto()


class GameTeamValue(Enum):
    GAME_TEAM_VALUE_GOALIE = 0
    GAME_TEAM_VALUE_DEFEND = auto()
    GAME_TEAM_VALUE_MIDFIELD = auto()
    GAME_TEAM_VALUE_ATTACK = auto()


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
class LiveGameEvent:
    event_type: int
    verbosity: int
    team: int
    player: Union[int, UUID]
    player2: Union[int, UUID]
    commentary: str
    commentary_id: Union[int, UUID]


@dataclass
class LiveGameUnit:
    possession: int
    area: LiveGameUnitArea
    minute: int
    time: LiveGameUnitTime
    result: list
    event: LiveGameEvent


@dataclass
class LiveGameTeamState:
    structure: int
    style: int


@dataclass
class Match:
    championship_id: int
    match_id: int
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


class EventHandler:
    def __init__(self):
        self.possible_events = [event.name for event in LiveGameEventType]
        self.event_history = []

    def create_event_unit(self, curr_minutes: float):
        pass


@dataclass
class LiveGame:
    match: Match
    stats = LiveGameStats()
    started_game: bool = False
    minutes: float = 0.0
    eventhandler: EventHandler = EventHandler()
    units: list = field(default_factory=list)
    running: bool = False

    def play_live_game(self):
        self.running = True
        while self.running:
            self.eventhandler.create_event_unit(self.minutes)
