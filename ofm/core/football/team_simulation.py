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
import random
from dataclasses import dataclass
from datetime import timedelta
from enum import Enum, auto
from math import ceil
from typing import Optional
from uuid import UUID

from ..simulation import PitchPosition
from ..simulation.team_strategy import TeamStrategy
from .club import Club
from .formation import Formation
from .player import PlayerSimulation


class SubbingError(Exception):
    pass


class GameEventType(Enum):
    GOAL = auto()
    PENALTY_GOAL = auto()
    YELLOW_CARD = auto()
    RED_CARD = auto()
    OWN_GOAL = auto()
    SUBSTITUTION = auto()


@dataclass
class GameEventAdditionalTime:
    additional_time: timedelta = timedelta(0)


@dataclass
class GameEventBase:
    player: PlayerSimulation
    minutes: timedelta
    event_type: GameEventType


@dataclass(repr=False)
class GameEvent(GameEventAdditionalTime, GameEventBase):
    def __repr__(self):
        minutes = f"{int(self.minutes.total_seconds() / 60)}'"
        if self.additional_time > timedelta(0):
            minutes = f"{minutes} + {ceil(self.additional_time.total_seconds() / 60)}'"
        if self.event_type == GameEventType.PENALTY_GOAL:
            minutes += " (pen)"
        if self.event_type == GameEventType.OWN_GOAL:
            minutes += " (o.g.)"
        return f"{self.player} {minutes}"


@dataclass
class SubstitutionEventBase(GameEventBase):
    player_subbed_in: PlayerSimulation


@dataclass(repr=False)
class SubstitutionEvent(GameEventAdditionalTime, SubstitutionEventBase):
    def __repr__(self):
        minutes = f"{int(self.minutes.total_seconds() / 60)}'"
        if self.additional_time > timedelta(0):
            minutes = f"{minutes} + {ceil(self.additional_time.total_seconds() / 60)}'"
        return f"{self.player} -> {self.player_subbed_in} {minutes}"


class TeamSimulation:
    def __init__(
        self,
        club: Club,
        formation: Formation,
        max_substitutions: int = 5,
        strategy: TeamStrategy = TeamStrategy.NORMAL,
    ):
        self.club: Club = club
        self.formation: Formation = formation
        self.in_possession: bool = False
        self.max_substitutions: int = max_substitutions
        self.substitutions: int = 0
        self.player_in_possession: Optional[PlayerSimulation] = None
        self.game_events: list[Optional[GameEvent]] = []
        self.sub_history: list[Optional[SubstitutionEvent]] = []
        self.goals_history: list[Optional[GameEvent]] = []
        self.red_card_history: list[Optional[GameEvent]] = []
        self.yellow_card_history: list[Optional[GameEvent]] = []
        self._score: int = 0
        self.team_strategy: TeamStrategy = strategy
        self.stats: TeamStats = TeamStats(self.club.club_id)

    @property
    def score(self) -> int:
        self._score = len(self.goals_history)
        return self._score

    def add_game_event(self, game_event: GameEvent):
        self.game_events.append(game_event)

    def add_goal(
        self,
        player: PlayerSimulation,
        minutes: timedelta,
        additional_time: timedelta = timedelta(0),
        penalty: bool = False,
    ):
        goal_data = GameEvent(
            player,
            minutes,
            GameEventType.GOAL if not penalty else GameEventType.PENALTY_GOAL,
            additional_time,
        )
        self.goals_history.append(goal_data)
        self.game_events.append(goal_data)

    def add_yellow_card(
        self,
        player: PlayerSimulation,
        minutes: timedelta,
        additional_time: timedelta = timedelta(0),
    ):
        yellow_card = GameEvent(
            player, minutes, GameEventType.YELLOW_CARD, additional_time
        )
        self.yellow_card_history.append(yellow_card)
        self.game_events.append(yellow_card)

    def add_red_card(
        self,
        player: PlayerSimulation,
        minutes: timedelta,
        additional_time: timedelta = timedelta(0),
    ):
        red_card = GameEvent(player, minutes, GameEventType.RED_CARD, additional_time)
        self.red_card_history.append(red_card)
        self.game_events.append(red_card)

    def get_player_on_pitch(
        self,
        position: PitchPosition,
    ) -> PlayerSimulation:
        players = self.formation.players.copy()
        if position == PitchPosition.DEF_BOX:
            probabilities = [0.3]
            df_prob = [
                0.5 if player.able_to_play else 0 for player in self.formation.df
            ]
            probabilities.extend(df_prob)
            mf_prob = [
                0.1 if player.able_to_play else 0 for player in self.formation.mf
            ]
            probabilities.extend(mf_prob)
            fw_prob = [
                0.1 if player.able_to_play else 0 for player in self.formation.fw
            ]
            probabilities.extend(fw_prob)
        elif position in [
            PitchPosition.DEF_RIGHT,
            PitchPosition.DEF_LEFT,
            PitchPosition.DEF_MIDFIELD_LEFT,
            PitchPosition.DEF_MIDFIELD_CENTER,
            PitchPosition.DEF_MIDFIELD_RIGHT,
        ]:
            players.remove(self.formation.gk)
            probabilities = [
                0.6 if player.able_to_play else 0 for player in self.formation.df
            ]
            mf_prob = [
                0.3 if player.able_to_play else 0 for player in self.formation.mf
            ]
            probabilities.extend(mf_prob)
            fw_prob = [
                0.1 if player.able_to_play else 0 for player in self.formation.fw
            ]
            probabilities.extend(fw_prob)
        elif position in [
            PitchPosition.MIDFIELD_RIGHT,
            PitchPosition.MIDFIELD_CENTER,
            PitchPosition.MIDFIELD_LEFT,
            PitchPosition.OFF_MIDFIELD_RIGHT,
            PitchPosition.OFF_MIDFIELD_LEFT,
            PitchPosition.OFF_MIDFIELD_CENTER,
        ]:
            players.remove(self.formation.gk)
            probabilities = [
                0.2 if player.able_to_play else 0 for player in self.formation.df
            ]
            mf_prob = [
                0.5 if player.able_to_play else 0 for player in self.formation.mf
            ]
            probabilities.extend(mf_prob)
            fw_prob = [
                0.3 if player.able_to_play else 0 for player in self.formation.fw
            ]
            probabilities.extend(fw_prob)
        else:
            players.remove(self.formation.gk)
            probabilities = [
                0.1 if player.able_to_play else 0 for player in self.formation.df
            ]
            mf_prob = [
                0.3 if player.able_to_play else 0 for player in self.formation.mf
            ]
            probabilities.extend(mf_prob)
            fw_prob = [
                0.6 if player.able_to_play else 0 for player in self.formation.fw
            ]
            probabilities.extend(fw_prob)

        if (
            self.player_in_possession is not None
            and self.player_in_possession in players
        ):
            idx = players.index(self.player_in_possession)
            players.pop(idx)
            probabilities.pop(idx)

        probabilities = list(filter(lambda x: x > 0, probabilities))
        players = list(filter(lambda x: x.able_to_play, players))
        if len(probabilities) != len(players):
            return random.choice(players)

        return random.choices(players, probabilities)[0]

    def update_stats(self):
        players = self.formation.all_players
        self.stats.update_stats(players)

    def sub_player(
        self,
        player_out: PlayerSimulation,
        player_in: PlayerSimulation,
        time: timedelta,
        additional_time: timedelta,
    ):
        if player_out.subbed:
            raise SubbingError("Player is already subbed!")
        if player_out.sent_off or player_in.sent_off:
            raise SubbingError("Cannot sub a player that has been sent off!")
        if self.substitutions == self.max_substitutions:
            raise SubbingError(f"Already made {self.max_substitutions} substitutions!")

        self.substitutions += 1

        sub = SubstitutionEvent(
            player_out, time, GameEventType.SUBSTITUTION, player_in, additional_time
        )
        self.sub_history.append(sub)
        self.formation.substitute_player(player_out, player_in)

    def get_best_penalty_taker(self) -> PlayerSimulation:
        best_penalty_taker = None
        for player in self.formation.players:
            if player.sent_off:
                continue
            if best_penalty_taker is None:
                best_penalty_taker = player
            elif (
                player.attributes.offensive.penalty
                > best_penalty_taker.attributes.offensive.penalty
            ):
                best_penalty_taker = player

        return best_penalty_taker

    def get_best_free_kick_taker(self) -> PlayerSimulation:
        best_free_kick_taker = None
        for player in self.formation.players:
            if player.sent_off:
                continue
            if best_free_kick_taker is None:
                best_free_kick_taker = player
            elif (
                player.attributes.offensive.free_kick
                > best_free_kick_taker.attributes.offensive.free_kick
            ):
                best_free_kick_taker = player

        return best_free_kick_taker

    def get_best_corner_kick_taker(self, is_pass: bool) -> PlayerSimulation:
        best_corner_kick_taker = None
        for player in self.formation.players:
            if player.sent_off:
                continue
            if best_corner_kick_taker is None:
                best_corner_kick_taker = player
            elif is_pass:
                if (
                    player.attributes.intelligence.passing
                    > best_corner_kick_taker.attributes.intelligence.passing
                ):
                    best_corner_kick_taker = player
            else:
                if (
                    player.attributes.intelligence.crossing
                    > best_corner_kick_taker.attributes.intelligence.crossing
                ):
                    best_corner_kick_taker = player

        return best_corner_kick_taker

    def update_player_stamina(self, duration: float):
        for player in self.formation.players:
            player.update_stamina(duration)


@dataclass
class TeamStats:
    club_id: UUID
    shots: int = 0
    shots_on_target: int = 0
    passes: int = 0
    passes_missed: int = 0
    crosses: int = 0
    crosses_missed: int = 0
    dribbles: int = 0
    dribbles_failed: int = 0
    interceptions: int = 0
    assists: int = 0
    fouls: int = 0
    goals: int = 0
    own_goals: int = 0
    penalties: int = 0
    corners: int = 0
    goal_kicks: int = 0
    injuries: int = 0
    yellow_cards: int = 0
    red_cards: int = 0
    avg_rating: float = 0.0
    possession: float = 0.0
    goals_conceded: int = 0
    offsides: int = 0

    def update_stats(self, players: list[PlayerSimulation]):
        self.fouls = sum(player.statistics.fouls for player in players)
        self.goals = sum(player.statistics.goals for player in players)
        self.yellow_cards = sum(player.statistics.yellow_cards for player in players)
        self.red_cards = sum(player.statistics.red_cards for player in players)
        self.goals_conceded = sum(
            player.statistics.goals_conceded for player in players
        )
        self.shots = sum(player.statistics.shots for player in players)
        self.shots_on_target = sum(
            player.statistics.shots_on_target for player in players
        )
        self.passes = sum(player.statistics.passes for player in players)
        self.passes_missed = sum(player.statistics.passes_missed for player in players)
        self.crosses = sum(player.statistics.crosses for player in players)
        self.crosses_missed = sum(
            player.statistics.crosses_missed for player in players
        )
        self.dribbles = sum(player.statistics.dribbles for player in players)
        self.dribbles_failed = sum(
            player.statistics.dribbles_failed for player in players
        )
        self.interceptions = sum(player.statistics.interceptions for player in players)
        self.assists = sum(player.statistics.assists for player in players)
