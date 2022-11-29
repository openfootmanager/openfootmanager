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
from abc import ABC, abstractmethod
from ...common.player import PlayerSimulation
from ...common.team import TeamSimulation


class Event(ABC):
    def __init__(self, minutes: int, team1: TeamSimulation, team2: TeamSimulation):
        self.minutes = minutes
        self.team1 = team1
        self.team2 = team2

    @abstractmethod
    def process_event(self, *args, **kwargs):
        pass


class EventDuel:
    def event_duel(self, player_atk: PlayerSimulation, player_def: PlayerSimulation):
        pass


class GoalOpportunityEvent(Event, EventDuel):
    def process_event(self, *args, **kwargs):
        pass


class PenaltyEvent(Event, EventDuel):
    def process_event(self, *args, **kwargs):
        pass


class FoulEvent(Event, EventDuel):
    def process_event(self, *args, **kwargs):
        pass


class FreeKickEvent(Event, EventDuel):
    def process_event(self, *args, **kwargs):
        pass


class StartMatchEvent(Event):
    def process_event(self, *args, **kwargs):
        pass


class CornerKickEvent(Event, EventDuel):
    def process_event(self, *args, **kwargs):
        pass


class InjuryEvent(Event):
    def process_event(self, *args, **kwargs):
        pass


class SubstitutionEvent(Event):
    def process_event(
            self,
            players_substituted: list[PlayerSimulation],
            players_entered: list[PlayerSimulation],
    ):
        pass


class YellowCardEvent(Event):
    def process_event(self, *args, **kwargs):
        pass


class RedCardEvent(Event):
    def process_event(self, *args, **kwargs):
        pass


class NothingEvent(Event):
    def process_event(self, *args, **kwargs):
        pass


class ExtraTimeEvent(Event):
    def process_event(self, *args, **kwargs):
        pass


class HalfTimeEvent(Event):
    def process_event(self, *args, **kwargs):
        pass


class EndMatchEvent(Event):
    def process_event(self, *args, **kwargs):
        pass
