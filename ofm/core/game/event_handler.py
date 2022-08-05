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
    return {
        "start_match": {
            "event": "start_match",
            "cond": "start_match",
            "type": StartMatchEvent,
        },
        "nothing": {
            "event": "nothing",
            "prob": 0.3,
            "type": NothingEvent,
        },
        "foul": {
            "event": "foul",
            "prob": 0.1,
            "type": FoulEvent,
        },
        "penalty": {
            "event": "penalty",
            "prob": 0.1,
            "type": PenaltyEvent,
        },
        "goal_opportunity": {
            "event": "goal_opportunity",
            "prob": 0.05,
            "type": GoalOpportunityEvent,
        },
        "free_kick": {
            "event": "free_kick",
            "prob": 0.1,
            "type": FreeKickEvent,
        },
        "corner_kick": {
            "event": "corner_kick",
            "prob": 0.1,
            "type": CornerKickEvent,
        },
        "injury": {
            "event": "injury",
            "prob": 0.03,
            "type": InjuryEvent,
        },
        "substitution": {
            "event": "substitution",
            "cond": "substitution",
            "type": SubstitutionEvent,
        },
        "yellow_card": {
            "event": "yellow_card",
            "prob": 0.08,
            "type": YellowCardEvent,
        },
        "red_card": {
            "event": "red_card",
            "prob": 0.03,
            "type": RedCardEvent,
        },
        "end_match": {
            "event": "end_match",
            "cond": "end_match",
            "type": EndMatchEvent,
        },
        "extra_time": {
            "event": "extra_time",
            "cond": "extra_time",
            "type": ExtraTimeEvent,
        },
        "half_time": {
            "event": "half_time",
            "cond": "half_time",
            "type": HalfTimeEvent,
        }
    }


class EventHandler:
    def __init__(self,  possible_extra_time: bool, possible_penalties: bool):
        self.minutes = 0
        self.all_events = get_events()
        self.possible_events = None
        self.event_history = []
        self.possible_extra_time = possible_extra_time
        self.possible_penalties = possible_penalties

    def get_possible_events(self):
        if self.minutes == 0:
            self.possible_events = self.all_events["start_match"]
        elif self.minutes in [45, 105]:
            self.possible_events = self.all_events["half_time"]
        elif self.minutes == 90:
            if self.possible_extra_time:
                self.possible_events = self.all_events["extra_time"]
            else:
                self.possible_events = self.all_events["end_match"]
        elif self.minutes == 120:
            if self.possible_penalties:
                self.possible_events = self.all_events["penalties"]
            else:
                self.possible_events = self.all_events["end_match"]
        else:
            self.possible_events = [event for event in self.all_events if 'prob' in event]

        return self.possible_events

    def generate_event(self):
        if isinstance(self.possible_events, dict):
            return self.possible_events
        if isinstance(self.possible_events, list):
            probs = [event["prob"] for event in self.possible_events]
            return random.choices(self.possible_events, probs)

    def get_event_from_list(self, event: str, team1: TeamSimulation, team2: TeamSimulation):
        return self.all_events[event]["type"](self.minutes, team1, team2)
