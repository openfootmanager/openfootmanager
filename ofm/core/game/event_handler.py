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
import random

from .events import *


def get_events():
    return [
        {
            "event": "start_match",
            "cond": "start_match",
            "type": StartMatchEvent,
        },
        {
            "event": "nothing",
            "prob": 0.3,
            "type": NothingEvent,
        },
        {
            "event": "foul",
            "prob": 0.1,
            "type": FoulEvent,
        },
        {
            "event": "penalty",
            "prob": 0.1,
            "type": PenaltyEvent,
        },
        {
            "event": "goal_opportunity",
            "prob": 0.05,
            "type": GoalOpportunityEvent,
        },
        {
            "event": "free_kick",
            "prob": 0.1,
            "type": FreeKickEvent,
        },
        {
            "event": "corner_kick",
            "prob": 0.1,
            "type": CornerKickEvent,
        },
        {
            "event": "longshot",
            "prob": 0.05,
            "type": LongShotEvent,
        },
        {
            "event": "injury",
            "prob": 0.03,
            "type": InjuryEvent,
        },
        {
            "event": "substitution",
            "cond": "substitution",
            "type": SubstitutionEvent,
        },
        {
            "event": "yellow_card",
            "prob": 0.08,
            "type": YellowCardEvent,
        },
        {
            "event": "red_card",
            "prob": 0.03,
            "type": RedCardEvent,
        },
        {
            "event": "penalties",
            "cond": "penalty_shootout",
            "type": PenaltiesEvent,
        },
        {
            "event": "end_match",
            "cond": "end_match",
            "type": EndMatchEvent,
        }
    ]


class EventHandler:
    def __init__(self, minutes: int = 0):
        self.minutes = minutes
        self.all_events = get_events()
        self.possible_events = self.get_possible_events()
        self.event_history = []

    def get_possible_events(self):
        if self.minutes == 0:
            return self.all_events[0]



    def generate_event(self):
        rand = random.randint(0, 1)

    def get_event(self, event_type: str):
        for event in self.all_events:
            if event["event"] == event_type:
                return event["type"](self.minutes)

        return NotImplemented
