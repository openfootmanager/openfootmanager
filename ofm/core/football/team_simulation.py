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
from dataclasses import dataclass
from typing import Optional, Tuple
from uuid import UUID

from ..simulation import PitchPosition
from ..simulation.team_strategy import TeamStrategy
from .club import Club
from .formation import Formation
from .player import PlayerSimulation


@dataclass
class Goal:
    player: PlayerSimulation
    minutes: float


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
        self.sub_history: list[Tuple[PlayerSimulation, PlayerSimulation]]
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
        if position == PitchPosition.DEF_BOX:
            players = [self.formation.gk]
            players.extend(self.formation.df)
        elif position in [
            PitchPosition.DEF_RIGHT,
            PitchPosition.DEF_LEFT,
            PitchPosition.DEF_MIDFIELD_LEFT,
            PitchPosition.DEF_MIDFIELD_RIGHT,
            PitchPosition.DEF_MIDFIELD_CENTER,
        ]:
            players = self.formation.df.copy()
            players.extend(self.formation.mf)
        elif position in [
            PitchPosition.MIDFIELD_RIGHT,
            PitchPosition.MIDFIELD_CENTER,
            PitchPosition.MIDFIELD_LEFT,
        ]:
            players = self.formation.df.copy()
            players.extend(self.formation.mf)
            players.extend(self.formation.fw)
        else:
            players = self.formation.fw.copy()
            players.extend(self.formation.mf)

        # Removes the player in possession
        if self.player_in_possession is not None:
            players.remove(self.player_in_possession)

        return random.choice(players)

    def update_player_stamina(self):
        pass


@dataclass
class TeamStats:
    club_id: UUID
    shots: int = 0
    shots_on_target: int = 0
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
