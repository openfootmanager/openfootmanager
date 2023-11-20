#      Openfoot Manager - A free and open source soccer management simulation
#      Copyright (C) 2020-2023  Pedrenrique G. Guimarães
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
from decimal import Decimal
from dataclasses import dataclass
from typing import Optional, Tuple
from uuid import UUID

from .club import Club
from .formation import Formation
from .player import PlayerSimulation
from ..simulation import PitchPosition
from ..simulation.team_strategy import TeamStrategy


class SubbingError(Exception):
    pass


@dataclass
class Goal:
    player: PlayerSimulation
    minutes: Decimal


class TeamSimulation:
    def __init__(
        self,
        club: Club,
        formation: Formation,
        strategy: TeamStrategy = TeamStrategy.NORMAL,
    ):
        self.club: Club = club
        self.formation: Formation = formation
        self.in_possession: bool = False
        self.substitutions: int = 0
        self.player_in_possession: Optional[PlayerSimulation] = None
        self.sub_history: list[Tuple[PlayerSimulation, PlayerSimulation]] = []
        self.goals_history: list[Optional[Goal]] = []
        self._score: int = 0
        self.team_strategy: TeamStrategy = strategy
        self.stats: TeamStats = TeamStats(self.club.club_id)

    @property
    def score(self) -> int:
        self._score = len(self.goals_history)
        return self._score

    def add_goal(self, goal_data: Goal):
        self.goals_history.append(goal_data)

    def get_player_on_pitch(
        self,
        position: PitchPosition,
    ) -> PlayerSimulation:
        players = self.formation.players.copy()
        if position == PitchPosition.DEF_BOX:
            probabilities = [0.3]
            df_prob = [0.5 / len(self.formation.df) for _ in range(len(self.formation.df))]
            probabilities.extend(df_prob)
            mf_prob = [0.1 / len(self.formation.mf) for _ in range(len(self.formation.mf))]
            probabilities.extend(mf_prob)
            fw_prob = [0.1 / len(self.formation.fw) for _ in range(len(self.formation.fw))]
            probabilities.extend(fw_prob)
        elif position in [
            PitchPosition.DEF_RIGHT,
            PitchPosition.DEF_LEFT,
            PitchPosition.DEF_MIDFIELD_LEFT,
            PitchPosition.DEF_MIDFIELD_CENTER,
            PitchPosition.DEF_MIDFIELD_RIGHT,
        ]:
            players.remove(self.formation.gk)
            probabilities = [0.6 / len(self.formation.df) for _ in range(len(self.formation.df))]
            mf_prob = [0.3 / len(self.formation.mf) for _ in range(len(self.formation.mf))]
            probabilities.extend(mf_prob)
            fw_prob = [0.1 / len(self.formation.fw) for _ in range(len(self.formation.fw))]
            probabilities.extend(fw_prob)
        elif position in [
            PitchPosition.MIDFIELD_RIGHT,
            PitchPosition.MIDFIELD_CENTER,
            PitchPosition.MIDFIELD_LEFT,
        ]:
            players.remove(self.formation.gk)
            probabilities = [0.2 / len(self.formation.df) for _ in range(len(self.formation.df))]
            mf_prob = [0.5 / len(self.formation.mf) for _ in range(len(self.formation.mf))]
            probabilities.extend(mf_prob)
            fw_prob = [0.3 / len(self.formation.fw) for _ in range(len(self.formation.fw))]
            probabilities.extend(fw_prob)
        else:
            players.remove(self.formation.gk)
            probabilities = [0.1 / len(self.formation.df) for _ in range(len(self.formation.df))]
            mf_prob = [0.3 / len(self.formation.mf) for _ in range(len(self.formation.mf))]
            probabilities.extend(mf_prob)
            fw_prob = [0.5 / len(self.formation.fw) for _ in range(len(self.formation.fw))]
            probabilities.extend(fw_prob)

        if self.player_in_possession is not None:
            idx = players.index(self.player_in_possession)
            players.pop(idx)
            probabilities.pop(idx)

        # Red card players cannot receive the ball
        for player, probability in zip(players, probabilities):
            if player.sent_off or not player.able_to_play:
                players.remove(player)
                probabilities.remove(probability)

        return random.choices(players, probabilities)[0]

    def update_stats(self):
        players = self.formation.all_players
        self.stats.update_stats(players)

    def sub_player(self, sub_player: PlayerSimulation, subbed_player: PlayerSimulation):
        if subbed_player.subbed:
            raise SubbingError("Player is already subbed!")
        if subbed_player.sent_off:
            raise SubbingError("Cannot sub a player that has been sent off!")

        self.substitutions += 1

        self.sub_history.append((sub_player, subbed_player))
        self.formation.substitute_player(sub_player, subbed_player)

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

    def update_player_stamina(self):
        pass


@dataclass
class TeamStats:
    club_id: UUID
    shots: int = 0
    shots_on_target: int = 0
    passes: int = 0
    passes_missed: int = 0
    crosses: int = 0
    crosses_missed: int = 0
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
        self.interceptions = sum(player.statistics.interceptions for player in players)
        self.assists = sum(player.statistics.assists for player in players)
