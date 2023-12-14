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
from typing import Optional

from ...football.player import PlayerSimulation, PlayerInjury
from ...football.team_simulation import TeamSimulation
from ..event import SimulationEvent, EventOutcome, CommentaryImportance
from ..event_type import FoulType
from ..game_state import GameState


@dataclass
class FoulEvent(SimulationEvent):
    commentary_importance = CommentaryImportance.MEDIUM
    foul_type: Optional[FoulType] = None

    def get_foul_type(self) -> FoulType:
        type_of_foul = [
            FoulType.OFFENSIVE_FOUL,
            FoulType.DEFENSIVE_FOUL,
        ]

        return random.choice(type_of_foul)

    def get_player_injury(
        self, offending_player: PlayerSimulation, fouled_player: PlayerSimulation
    ):
        fouled_player_resistance = (
            (
                fouled_player.attributes.physical.endurance
                + fouled_player.attributes.physical.strength
                + fouled_player.player.details.fitness * 2
            )
            / 4
        ) - (100 - fouled_player.player.details.stamina)
        offending_player_aggression = (
            offending_player.attributes.defensive.tackling
            + offending_player.attributes.physical.strength
            + offending_player.attributes.defensive.positioning
        ) / 3

        enduring_probability = fouled_player_resistance + offending_player_aggression
        not_enduring_prob = 200 - enduring_probability

        # light_inj + medium_inj + severe_inj + career_ending = not_enduring_prob

        endures = [
            PlayerInjury.NO_INJURY,
            None,
        ]

        probability_of_injury = [enduring_probability, not_enduring_prob]

        enduring = random.choices(endures, probability_of_injury)[0]

        if enduring is not None:
            return enduring

        injuries = list(PlayerInjury)
        injuries.remove(PlayerInjury.NO_INJURY)

        injuries_prob = [
            0.99,
            0.009,
            0.000099,
            0.000001,
        ]
        injury = random.choices(injuries, injuries_prob)[0]
        if injury in [
            PlayerInjury.MEDIUM_INJURY,
            PlayerInjury.SEVERE_INJURY,
            PlayerInjury.CAREER_ENDING_INJURY
        ]:
            fouled_player.able_to_play = False

        return injury

    def get_player_card(self, player_injury: PlayerInjury) -> EventOutcome:
        if player_injury in [
            PlayerInjury.SEVERE_INJURY,
            PlayerInjury.CAREER_ENDING_INJURY,
        ]:
            return EventOutcome.FOUL_RED_CARD
        elif player_injury == PlayerInjury.MEDIUM_INJURY:
            return EventOutcome.FOUL_YELLOW_CARD

        outcomes = [
            EventOutcome.FOUL_WARNING,
            EventOutcome.FOUL_YELLOW_CARD,
        ]
        probability = [
            95,
            5
        ]

        return random.choices(outcomes, probability)[0]

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.attacking_player = attacking_team.player_in_possession
        self.defending_player = defending_team.get_player_on_pitch(self.state.position)

        self.foul_type = self.get_foul_type()

        if self.foul_type == FoulType.OFFENSIVE_FOUL:
            fouled_player = self.defending_player
            offending_player = self.attacking_player
        else:
            fouled_player = self.attacking_player
            offending_player = self.defending_player

        offending_player.statistics.fouls += 1

        injury_type = self.get_player_injury(offending_player, fouled_player)
        fouled_player.injury_type = injury_type

        self.outcome = self.get_player_card(injury_type)

        if self.outcome == EventOutcome.FOUL_YELLOW_CARD:
            offending_player.statistics.yellow_cards += 1
            self.commentary.append(f"{offending_player} received a yellow card!")

        if offending_player.statistics.yellow_cards == 2:
            self.outcome = EventOutcome.FOUL_RED_CARD
            self.commentary.append(
                f"{offending_player} now has 2 yellow cards! That's a send off!"
            )

        if self.outcome == EventOutcome.FOUL_RED_CARD:
            offending_player.statistics.red_cards += 1
            self.commentary.append(f"{offending_player} received a red card!")

        return self.state
