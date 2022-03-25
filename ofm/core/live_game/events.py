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
from enum import Enum, auto
from dataclasses import dataclass
from typing import Union
from uuid import UUID


class LiveGameUnitArea(Enum):
    UNIT_AREA_DEFEND = 0
    UNIT_AREA_MIDFIELD = auto()
    UNIT_AREA_ATTACK = auto()


class LiveGameUnitTime(Enum):
    UNIT_TIME_FIRST_HALF = 0
    UNIT_TIME_SECOND_HALF = auto()
    UNIT_TIME_EXTRA_TIME = auto()
    UNIT_TIME_PENALTIES = auto()


class GameTeamValue(Enum):
    GAME_TEAM_VALUE_GOALIE = 0
    GAME_TEAM_VALUE_DEFEND = auto()
    GAME_TEAM_VALUE_MIDFIELD = auto()
    GAME_TEAM_VALUE_ATTACK = auto()


class LiveGameEventType(Enum):
    EVENT_GENERAL = 0
    EVENT_START_MATCH = auto()
    EVENT_HALF_TIME = auto()
    EVENT_EXTRA_TIME = auto()
    EVENT_END_MATCH = auto()
    EVENT_LOST_POSSESSION = auto()
    EVENT_SCORING_CHANCE = auto()
    EVENT_HEADER = auto()
    EVENT_PENALTY = auto()
    EVENT_FREE_KICK = auto()
    EVENT_GOAL = auto()
    EVENT_OWN_GOAL = auto()
    EVENT_POST = auto()
    EVENT_MISS = auto()
    EVENT_SAVE = auto()
    EVENT_KEEPER_PUSHED_IN_CORNER = auto()
    EVENT_CROSS_BAR = auto()
    EVENT_PLAYER_PUSHED_IN_CORNER = auto()
    EVENT_CORNER_KICK = auto()
    EVENT_FOUL = auto()
    EVENT_FOUL_YELLOW = auto()
    EVENT_FOUL_RED = auto()
    EVENT_FOUL_RED_INJURY = auto()
    EVENT_SEND_OFF = auto()
    EVENT_INJURY = auto()
    EVENT_TEMP_INJURY = auto()
    EVENT_PENALTIES = auto()
    EVENT_STADIUM = auto()
    EVENT_STADIUM_BREAKDOWN = auto()
    EVENT_STADIUM_RIOTS = auto()
    EVENT_STADIUM_FIRE = auto()
    EVENT_SUBSTITUTION = auto()
    EVENT_STRUCTURE_CHANGE = auto()
    EVENT_STYLE_CHANGE_ALL_OUT_DEFEND = auto()
    EVENT_STYLE_CHANGE_DEFEND = auto()
    EVENT_STYLE_CHANGE_BALANCED = auto()
    EVENT_STYLE_CHANGE_ATTACK = auto()
    EVENT_STYLE_CHANGE_ALL_OUT_ATTACK = auto()
    EVENT_BOOST_CHANGE_ANTI = auto()
    EVENT_BOOST_CHANGE_OFF = auto()
    EVENT_BOOST_CHANGE_ON = auto()


@dataclass
class LiveGameEvent:
    event_type: int
    verbosity: int
    team: int
    player: Union[int, UUID]
    player2: Union[int, UUID]
    commentary: str
    commentary_id: Union[int, UUID]

    def calculate_event_foul(self, *args, **kwargs):
        pass

    def calculate_event_lost_possession(self, *args, **kwargs):
        pass

    def calculate_event_injury(self, *args, **kwargs):
        pass

    def calculate_event_stadium(self, *args, **kwargs):
        pass

    def calculate_event_scoring_chance(self, *args, **kwargs):
        pass

    def calculate_event_penalty(self, *args, **kwargs):
        pass

    def calculate_event_general(self, *args, **kwargs):
        pass

    def calculate_event_free_kick(self, *args, **kwargs):
        pass

    def calculate_event_send_off(self, *args, **kwargs):
        pass

    def calculate_event_substitution(self, *args, **kwargs):
        pass

    def calculate_event_team_change(self, *args, **kwargs):
        pass

    def calculate_event_duel(self, *args, **kwargs):
        pass

    def calculate_event_corner_kick(self, *args, **kwargs):
        pass


@dataclass
class LiveGameUnit:
    possession: int
    area: LiveGameUnitArea
    minute: int
    time: LiveGameUnitTime
    result: list
    event: LiveGameEvent

    def calculate_event(self, *args, **kwargs):
        if self.event.event_type == LiveGameEventType.EVENT_FOUL:
            self.event.calculate_event_foul(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_LOST_POSSESSION:
            self.event.calculate_event_lost_possession(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_INJURY:
            self.event.calculate_event_injury(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_STADIUM:
            self.event.calculate_event_stadium(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_SCORING_CHANCE:
            self.event.calculate_event_scoring_chance(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_PENALTY:
            self.event.calculate_event_penalty(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_GENERAL:
            self.event.calculate_event_general(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_FREE_KICK:
            self.event.calculate_event_free_kick(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_SEND_OFF:
            self.event.calculate_event_send_off(*args, **kwargs)
        elif self.event.event_type == LiveGameEventType.EVENT_CORNER_KICK:
            self.event.calculate_event_corner_kick(*args, **kwargs)

        # # I'll test this later on. This seems to make sense, but I'm not sure if it works
        # events = {
        #     LiveGameEventType.EVENT_FOUL: self.event.calculate_event_foul,
        #     LiveGameEventType.EVENT_LOST_POSSESSION: self.event.calculate_event_lost_possession,
        #     LiveGameEventType.EVENT_INJURY: self.event.calculate_event_injury,
        #     LiveGameEventType.EVENT_STADIUM: self.event.calculate_event_stadium,
        #     LiveGameEventType.EVENT_SCORING_CHANCE: self.event.calculate_event_scoring_chance,
        #     LiveGameEventType.EVENT_PENALTY: self.event.calculate_event_penalty,
        #     LiveGameEventType.EVENT_GENERAL: self.event.calculate_event_general,
        #     LiveGameEventType.EVENT_FREE_KICK: self.event.calculate_event_free_kick,
        #     LiveGameEventType.EVENT_SUBSTITUTION: self.event.calculate_event_substitution,
        #     LiveGameEventType.EVENT_SEND_OFF: self.event.calculate_event_send_off,
        #     LiveGameEventType.EVENT_CORNER_KICK: self.event.calculate_event_corner_kick,
        # }
        # for key, event in events.items():
        #     if self.event.event_type == key:
        #         event(*args, **kwargs)


class EventHandler:
    def __init__(self):
        self.possible_events = [event.name for event in LiveGameEventType]
        self.event_history = []

    def create_event_unit(self, curr_minutes: float):
        pass
