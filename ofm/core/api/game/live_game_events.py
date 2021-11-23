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

    def _calculate_free_kick(self):
        pass

    def _calculate_corner_kick(self):
        pass

    def _calculate_penalty(self):
        pass

    def _calculate_injury(self):
        pass

    def _calculate_scoring_chance(self, scorer, gk):
        pass

    def _calculate_foul(self, player_committing_foul, player_receiving_foul):
        pass

    def calculate_event(self, *args, **kwargs):
        if self.event_type == "FOUL":
            self._calculate_foul(*args, **kwargs)


class LiveGameEventHandler:
    def __init__(self):
        self.event_history = []
        self.events = self.get_events()
        self.probs = self.get_event_probabilities()

    @staticmethod
    def get_events():
        return [
            {"name": "SCORING_CHANCE", "prob": 0.08},
            {"name": "SWITCH_POSSESSION", "prob": 0.25},
            {"name": "FREE_KICK", "prob": 0.125},
            {"name": "FOUL", "prob": 0.125},
            {"name": "INJURY", "prob": 0.125},
            {"name": "CORNER_KICK", "prob": 0.25},
            {"name": "PENALTY", "prob": 0.25},
        ]

    def get_event_probabilities(self):
        return [event["prob"] for event in self.events]

    def generate_events(self, game_time):
        event = random.choices(self.events, self.probs)[0]
        ev = LiveGameEvent(event["name"], game_time)
        self.event_history.append(ev)
