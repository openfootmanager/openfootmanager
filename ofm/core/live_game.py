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


class LiveGameStatValue(Enum):
    LIVE_GAME_STAT_VALUE_GOALS_REGULAR = 0
    LIVE_GAME_STAT_VALUE_SHOTS = auto()
    LIVE_GAME_STAT_VALUE_SHOT_PERCENTAGE = auto()
    LIVE_GAME_STAT_VALUE_POSSESSION = auto()
    LIVE_GAME_STAT_VALUE_PENALTIES = auto()
    LIVE_GAME_STAT_VALUE_FOULS = auto()
    LIVE_GAME_STAT_VALUE_CARDS = auto()
    LIVE_GAME_STAT_VALUE_REDS = auto()
    LIVE_GAME_STAT_VALUE_INJURIES = auto()


class LiveGameStatArray(Enum):
    LIVE_GAME_STAT_ARRAY_SCORERS_FOR_DISPLAY = 0
    LIVE_GAME_STAT_ARRAY_SCORERS = auto()
    LIVE_GAME_STAT_ARRAY_YELLOWS = auto()
    LIVE_GAME_STAT_ARRAY_REDS = auto()
    LIVE_GAME_STAT_ARRAY_INJURED = auto()


class GameTeamValue(Enum):
    GAME_TEAM_VALUE_GOALIE = 0
    GAME_TEAM_VALUE_DEFEND = auto()
    GAME_TEAM_VALUE_MIDFIELD = auto()
    GAME_TEAM_VALUE_ATTACK = auto()


@dataclass
class LiveGameStats:
    possession: float
    values: list
    players: list


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
    area: int
    minute: int
    time: int
    result: list
    event: LiveGameEvent


@dataclass
class LiveGameTeamState:
    structure: int
    style: int
    player_ids: list[Union[UUID, int]]


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

@dataclass
class LiveGame:
    match: Match
    subs_left: list
    started_game: int
    stadium_event: int
    team_values: list
    stats: LiveGameStats
    team_state: list[LiveGameTeamState]
    action_ids: list
    units: list = field(default_factory=list)
    


