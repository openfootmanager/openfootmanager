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
from dataclasses import dataclass
from typing import Optional

from ...football.team_simulation import TeamSimulation
from ..event import CommentaryImportance, SimulationEvent
from ..event_type import EventType
from ..game_state import GameState
from .shot_event import ShotEvent


@dataclass
class PenaltyKickEvent(SimulationEvent):
    commentary_importance = CommentaryImportance.HIGH
    sub_event: Optional[ShotEvent] = None

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.commentary.append("Penalty Kick!")
        self.defending_player = defending_team.formation.gk
        self.attacking_player = attacking_team.get_best_penalty_taker()

        self.commentary.append(f"{self.attacking_player} goes to the ball")
        self.sub_event = ShotEvent(
            EventType.PENALTY_KICK,
            self.state,
            outcome=None,
            attacking_player=self.attacking_player,
            defending_player=self.defending_player,
        )
        self.state = self.sub_event.calculate_event(attacking_team, defending_team)
        self.attacking_player = self.sub_event.attacking_player
        self.defending_player = self.sub_event.defending_player
        self.outcome = self.sub_event.outcome
        self.commentary.extend(self.sub_event.commentary)

        return self.state
