#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2021  Pedrenrique G. Guimarães
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


class LiveGameEvent:
    def __init__(self, event_type, game_time):
        self.event_type = event_type
        self.game_time = game_time

    def _calculate_switch_possession(self, teams=None, match=None):
        team1 = teams[0]
        team2 = teams[1]

        # I think this will change the possession without returning it, maybe
        match.current_possession = team2 if match.current_possession == team1 else team1
        print(match.current_possession)

    def _calculate_free_kick(self, *args, **kwargs):
        print("free kick")

    def _calculate_corner_kick(self, *args, **kwargs):
        print("corner_kick")

    def _calculate_penalty(self, *args, **kwargs):
        print("penalty")

    def _calculate_injury(self, *args, **kwargs):
        print("injury")

    def _calculate_scoring_chance(self, *args, **kwargs):
        print("scoring_chance")

    def _calculate_foul(self, *args, **kwargs):
        print("foul")

    def _get_event_dict_function(self):
        return {
            "SCORING_CHANCE": self._calculate_foul,
            "SWITCH_POSSESSION": self._calculate_switch_possession,
            "FREE_KICK": self._calculate_free_kick,
            "FOUL": self._calculate_foul,
            "INJURY": self._calculate_injury,
            "CORNER_KICK": self._calculate_corner_kick,
            "PENALTY": self._calculate_penalty,
        }

    def calculate_event(self, *args, **kwargs):
        events = self._get_event_dict_function()
        events[self.event_type](*args, **kwargs)


class LiveGameEventHandler:
    def __init__(self):
        self.event = None
        self.event_history = []
        self.events = self.get_events()
        self.probs = self.get_event_probabilities()

    @staticmethod
    def get_events():
        return [
            {"name": "SCORING_CHANCE", "prob": 0.10},
            {"name": "SWITCH_POSSESSION", "prob": 0.25},
            {"name": "FREE_KICK", "prob": 0.125},
            {"name": "FOUL", "prob": 0.125},
            {"name": "INJURY", "prob": 0.05},
            {"name": "CORNER_KICK", "prob": 0.25},
            {"name": "PENALTY", "prob": 0.25},
        ]

    def get_event_probabilities(self):
        return [event["prob"] for event in self.events]

    def generate_events(self, game_time):
        event = random.choices(self.events, self.probs)[0]
        self.event = LiveGameEvent(event["name"], game_time)
        self.event_history.append(self.event)
