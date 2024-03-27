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

from ...football.team_simulation import TeamSimulation
from .. import PitchPosition
from ..event import CommentaryImportance, EventOutcome, SimulationEvent
from ..event_type import EventType
from ..game_state import GameState


@dataclass
class ShotEvent(SimulationEvent):
    commentary_importance = CommentaryImportance.HIGH

    def get_shot_saved_outcomes(self) -> EventOutcome:
        final_outcomes = [
            EventOutcome.SHOT_RIGHT_CORNER_KICK,
            EventOutcome.SHOT_LEFT_CORNER_KICK,
            EventOutcome.SHOT_SAVED_SECURED,
        ]
        self.commentary.append(f"{self.defending_player} saved the ball!")
        return random.choice(final_outcomes)

    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        basic_event_outcomes = [
            EventOutcome.SHOT_MISS,
            EventOutcome.SHOT_ON_GOAL,
        ]
        event_probabilities = [
            100 - shot_on_goal,
            shot_on_goal,
        ]

        if self.defending_player != defending_team.formation.gk:
            basic_event_outcomes.append(EventOutcome.SHOT_BLOCKED)
            event_probabilities.append(
                (
                    self.defending_player.attributes.defensive.positioning
                    + self.defending_player.attributes.defensive.interception
                )
                / 2
            )

        return random.choices(basic_event_outcomes, event_probabilities)[0]

    def get_shot_blocked(self):
        outcomes = [
            EventOutcome.SHOT_BLOCKED_CHANGE_POSSESSION,
            EventOutcome.SHOT_BLOCKED_BACK,
        ]
        outcome = random.choice(outcomes)
        self.commentary.append(f"{self.defending_player} blocked the shot!")

        return outcome

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        outcomes = [
            EventOutcome.SHOT_HIT_POST,
            EventOutcome.SHOT_SAVED,
            EventOutcome.GOAL,
        ]
        if self.event_type == EventType.PENALTY_KICK:
            gk_skills = (
                self.defending_player.attributes.gk.penalty * 2
                + self.defending_player.attributes.gk.jumping
                + self.defending_player.attributes.gk.positioning
            ) / 4
        else:
            gk_skills = self.defending_player.attributes.gk.get_general_overall()

        probabilities = [
            110 - shot_on_goal,
            gk_skills,
            120 - gk_skills,  # Even very good goalies can let balls pass sometimes
        ]
        return random.choices(outcomes, probabilities)[0]

    def get_shot_hit_post(self) -> EventOutcome:
        final_outcomes = [
            EventOutcome.SHOT_HIT_POST,
            EventOutcome.SHOT_HIT_POST_CHANGE_POSSESSION,
            EventOutcome.SHOT_GOAL_KICK,
        ]

        return random.choice(final_outcomes)

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        if self.attacking_player is None:
            self.attacking_player = attacking_team.player_in_possession
        if self.defending_player is None:
            self.defending_player = defending_team.get_player_on_pitch(
                self.state.position
            )
        self.attacking_player.statistics.shots += 1

        if self.event_type == EventType.FREE_KICK:
            shot_on_goal = (
                self.attacking_player.attributes.offensive.shot_accuracy
                + self.attacking_player.attributes.offensive.shot_power
                + self.attacking_player.attributes.offensive.free_kick * 2
            ) / 4
        elif self.event_type == EventType.PENALTY_KICK:
            shot_on_goal = (
                self.attacking_player.attributes.offensive.penalty * 2
                + self.attacking_player.attributes.offensive.shot_power
                + self.attacking_player.attributes.offensive.shot_accuracy
            ) / 4
        else:
            shot_on_goal = (
                self.attacking_player.attributes.offensive.shot_accuracy
                + self.attacking_player.attributes.offensive.shot_power
            ) / 2

        self.commentary.append(f"{self.attacking_player} shoots!")

        first_outcome = self.get_shot_on_goal(shot_on_goal, defending_team)

        if first_outcome == EventOutcome.SHOT_MISS:
            self.attacking_player.statistics.shots_missed += 1
            self.outcome = EventOutcome.SHOT_GOAL_KICK
        elif first_outcome == EventOutcome.SHOT_BLOCKED:
            self.attacking_player.statistics.shots_missed += 1
            self.defending_player.statistics.shots_blocked += 1
            self.outcome = self.get_shot_blocked()
        elif first_outcome == EventOutcome.SHOT_ON_GOAL:
            self.defending_player = defending_team.formation.gk
            self.state.position = PitchPosition.OFF_BOX
            self.attacking_player.statistics.shots_on_target += 1
            self.outcome = self.get_shot_on_goal_outcomes(shot_on_goal)

        if self.outcome == EventOutcome.SHOT_HIT_POST:
            self.commentary.append(f"{self.attacking_player} hits the post!")
            self.outcome = self.get_shot_hit_post()
        elif self.outcome == EventOutcome.SHOT_SAVED:
            self.commentary.append(f"{self.defending_player} saves the shot!")
            self.defending_player.statistics.shots_saved += 1
            self.attacking_player.statistics.shots_missed += 1
            self.outcome = self.get_shot_saved_outcomes()
        elif self.outcome == EventOutcome.GOAL:
            self.commentary.append(f"GOAL! {self.attacking_player} scores!")
            self.attacking_player.statistics.goals += 1
            self.defending_player.statistics.goals_conceded += 1
            if self.attacking_player.received_ball is not None:
                self.attacking_player.received_ball.statistics.assists += 1
            self.state.position = PitchPosition.MIDFIELD_CENTER

            if self.state.in_additional_time:
                additional_time = self.state.additional_time_elapsed
            else:
                additional_time = timedelta(0)

            if self.event_type == EventType.PENALTY_KICK:
                attacking_team.add_goal(
                    self.attacking_player,
                    self.state.minutes,
                    additional_time,
                    penalty=True,
                )
            else:
                attacking_team.add_goal(
                    self.attacking_player,
                    self.state.minutes,
                    additional_time,
                    penalty=False,
                )

            defending_player = defending_team.get_player_on_pitch(self.state.position)
            self.state = self.change_possession(
                attacking_team,
                defending_team,
                defending_player,
                self.state.position,
            )

        if self.outcome in [
            EventOutcome.SHOT_BLOCKED_CHANGE_POSSESSION,
            EventOutcome.SHOT_HIT_POST_CHANGE_POSSESSION,
        ]:
            self.state = self.change_possession(
                attacking_team,
                defending_team,
                self.defending_player,
                self.state.position,
            )
        elif self.outcome in [
            EventOutcome.SHOT_GOAL_KICK,
            EventOutcome.SHOT_SAVED_SECURED,
        ]:
            self.state = self.change_possession(
                attacking_team,
                defending_team,
                defending_team.formation.gk,
                self.state.position,
            )
        elif self.outcome == EventOutcome.SHOT_RIGHT_CORNER_KICK:
            attacking_team.stats.corners += 1
            self.state.position = PitchPosition.OFF_RIGHT
        elif self.outcome == EventOutcome.SHOT_LEFT_CORNER_KICK:
            attacking_team.stats.corners += 1
            self.state.position = PitchPosition.OFF_LEFT

        self.attacking_player.received_ball = None
        self.defending_player.received_ball = None
        attacking_team.update_stats()
        defending_team.update_stats()
        return self.state
