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
from abc import abstractmethod
from copy import deepcopy
from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

from ..football.player import PlayerSimulation, PlayerInjury
from ..football.team_simulation import Goal, TeamSimulation
from . import OFF_POSITIONS, PITCH_EQUIVALENTS, PitchPosition
from .event_type import EventType
from .game_state import GameState
from .team_strategy import (
    team_general_strategy,
    team_pass_strategy,
    team_cross_strategy,
    team_goal_kick_strategy,
)


class EventOutcome(Enum):
    PASS_MISS = 0
    PASS_SUCCESS = auto()
    PASS_INTERCEPT = auto()
    PASS_OFFSIDE = auto()  # Only if PASS_SUCCESS happened first
    DRIBBLE_FAIL = auto()
    DRIBBLE_SUCCESS = auto()
    CROSS_MISS = auto()
    CROSS_SUCCESS = auto()
    CROSS_INTERCEPT = auto()
    CROSS_OFFSIDE = auto()
    FOUL_WARNING = auto()
    FOUL_YELLOW_CARD = auto()
    FOUL_RED_CARD = auto()
    SHOT_MISS = auto()
    SHOT_ON_GOAL = auto()
    SHOT_BLOCKED = auto()
    SHOT_BLOCKED_CHANGE = auto()  # Only if SHOT_BLOCKED happened first
    SHOT_BLOCKED_BACK = auto()  # Only if SHOT_BLOCKED happened first
    SHOT_SAVED = auto()  # Only if SHOT_ON_GOAL happened first
    SHOT_SAVED_SECURED = (
        auto()
    )  # Only if SHOT_SAVED happend first. Keeps the ball after saving.
    SHOT_HIT_POST = auto()  # Only if SHOT_ON_GOAL happened first
    SHOT_HIT_POST_CHANGE = (
        auto()
    )  # Only if SHOT_HIT_POST happened first. Changes possession after the event.
    SHOT_LEFT_CORNER_KICK = auto()  # Only if SHOT_MISS or SHOT_SAVED happened first
    SHOT_RIGHT_CORNER_KICK = auto()  # Only if SHOT_MISS or SHOT_SAVED happened first
    SHOT_GOAL_KICK = auto()  # Only if SHOT_MISS happened first
    GOAL = auto()  # Only if SHOT_ON_GOAL happened first
    OWN_GOAL = auto()


class FoulTypes(Enum):
    OFFENSIVE_FOUL = auto()
    DEFENSIVE_FOUL = auto()


class FreeKickType(Enum):
    DIRECT_SHOT = auto()
    CROSS = auto()
    PASS = auto()


@dataclass
class SimulationEvent:
    event_type: EventType
    state: GameState
    outcome: Optional[EventOutcome] = None
    attacking_player: Optional[PlayerSimulation] = None
    defending_player: Optional[PlayerSimulation] = None

    @abstractmethod
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        pass

    def change_possession(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
        defending_player: PlayerSimulation,
        position: PitchPosition,
    ) -> GameState:
        attacking_team.in_possession = False
        attacking_team.player_in_possession = None
        defending_team.in_possession = True
        defending_team.player_in_possession = defending_player
        position = PITCH_EQUIVALENTS[position]
        print(f"Ball is now with team {defending_team}")
        return GameState(self.state.minutes, position)


class EventFactory:
    def get_possible_events(
        self,
        teams: tuple[TeamSimulation, TeamSimulation],
        state: GameState,
        last_event: Optional[SimulationEvent],
    ) -> list[list[EventType] | list[float]]:
        if last_event is None:
            return [[EventType.PASS], [1.0]]

        if last_event.outcome == EventOutcome.GOAL:
            return [[EventType.PASS], [1.0]]
        elif last_event.outcome == EventOutcome.SHOT_GOAL_KICK:
            return [[EventType.GOAL_KICK], [1.0]]
        elif last_event.outcome in [
            EventOutcome.SHOT_LEFT_CORNER_KICK,
            EventOutcome.SHOT_RIGHT_CORNER_KICK,
        ]:
            return [[EventType.CORNER_KICK], [1.0]]
        elif isinstance(last_event, FoulEvent):
            if (
                last_event.foul_type == FoulTypes.DEFENSIVE_FOUL
                and state.position == PitchPosition.OFF_BOX
            ):
                return [[EventType.PENALTY_KICK], [1.0]]
            else:
                return [[EventType.FREE_KICK], [1.0]]

        attacking_team = teams[0]
        transition_matrix = team_general_strategy(attacking_team.team_strategy, state)

        return [
            [
                EventType(i)
                for i, _ in enumerate(transition_matrix[last_event.event_type.value])
            ],
            list(transition_matrix[last_event.event_type.value]),
        ]

    def get_event(self, _state: GameState, event_type: EventType) -> SimulationEvent:
        state = deepcopy(_state)
        if event_type == EventType.PASS:
            return PassEvent(EventType.PASS, state)
        elif event_type == EventType.DRIBBLE:
            return DribbleEvent(EventType.DRIBBLE, state)
        elif event_type == EventType.FOUL:
            return FoulEvent(EventType.FOUL, state)
        elif event_type == EventType.SHOT:
            return ShotEvent(EventType.SHOT, state)
        elif event_type == EventType.CROSS:
            return CrossEvent(EventType.CROSS, state)
        elif event_type == EventType.CORNER_KICK:
            return CornerKickEvent(EventType.CORNER_KICK, state)
        elif event_type == EventType.FREE_KICK:
            return FreeKickEvent(EventType.FREE_KICK, state)
        elif event_type == EventType.GOAL_KICK:
            return GoalKickEvent(EventType.GOAL_KICK, state)
        elif event_type == EventType.PENALTY_KICK:
            return PenaltyKickEvent(EventType.PENALTY_KICK, state)

        return NotImplemented


@dataclass
class PassEvent(SimulationEvent):
    receiving_player: Optional[PlayerSimulation] = None

    def get_end_position(self, attacking_team) -> PitchPosition:
        team_strategy = attacking_team.team_strategy
        transition_matrix = team_pass_strategy(team_strategy)
        probabilities = transition_matrix[self.state.position.value]
        return random.choices(list(PitchPosition), probabilities)[0]

    def get_pass_primary_outcome(self, distance) -> EventOutcome:
        outcomes = [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_SUCCESS,
            EventOutcome.PASS_INTERCEPT,
        ]

        if self.event_type == EventType.FREE_KICK:
            pass_success = (
                (
                    self.attacking_player.attributes.intelligence.passing
                    + self.attacking_player.attributes.intelligence.vision
                    + self.attacking_player.attributes.offensive.free_kick * 2
                )
                / 4
            ) - distance
        else:
            pass_success = (
                (
                    self.attacking_player.attributes.intelligence.passing
                    + self.attacking_player.attributes.intelligence.vision
                )
                / 2
            ) - distance

        outcome_probability = [
            100.0 - pass_success,  # PASS_MISS
            pass_success,  # PASS_SUCCESS
            (
                self.defending_player.attributes.defensive.positioning
                + self.defending_player.attributes.defensive.interception
            )
            / 2,  # PASS_INTERCEPT
        ]

        return random.choices(outcomes, outcome_probability)[0]

    def get_secondary_outcome(self) -> EventOutcome:
        outcomes = [EventOutcome.PASS_SUCCESS, EventOutcome.PASS_OFFSIDE]
        not_offside_probability = (
            self.receiving_player.attributes.offensive.positioning
            + self.receiving_player.attributes.intelligence.team_work
        ) / 2
        outcome_probability = [
            not_offside_probability,
            100 - not_offside_probability,
        ]
        return random.choices(outcomes, outcome_probability)[0]

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        # Transition matrix for each position on the field
        end_position = self.get_end_position(attacking_team)
        distance = abs(
            end_position.value - self.state.position.value
        )  # distance from current position to end position

        if self.attacking_player is None:
            self.attacking_player = attacking_team.player_in_possession
        if self.defending_player is None:
            self.defending_player = defending_team.get_player_on_pitch(end_position)
        self.receiving_player = attacking_team.get_player_on_pitch(end_position)
        self.attacking_player.statistics.passes += 1

        self.outcome = self.get_pass_primary_outcome(distance)

        if end_position in OFF_POSITIONS and self.outcome == EventOutcome.PASS_SUCCESS:
            self.outcome = self.get_secondary_outcome()

        if self.outcome in [
            EventOutcome.PASS_MISS,
            EventOutcome.PASS_INTERCEPT,
            EventOutcome.PASS_OFFSIDE,
        ]:
            self.attacking_player.statistics.passes_missed += 1
            print(f"{self.attacking_player} failed to pass the ball!")
            if self.outcome == EventOutcome.PASS_INTERCEPT:
                self.defending_player.statistics.interceptions += 1
            self.state = self.change_possession(
                attacking_team, defending_team, self.defending_player, end_position
            )
        else:
            print(f"{self.attacking_player} passed the ball to {self.receiving_player}")
            attacking_team.player_in_possession = self.receiving_player

        attacking_team.update_stats()
        defending_team.update_stats()
        return GameState(self.state.minutes, end_position)


@dataclass
class FoulEvent(SimulationEvent):
    foul_type: Optional[FoulTypes] = None

    def get_foul_type(self) -> FoulTypes:
        type_of_foul = [
            FoulTypes.OFFENSIVE_FOUL,
            FoulTypes.DEFENSIVE_FOUL,
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
            0.90,
            0.08,
            0.00099,
            0.00001,
        ]
        return random.choices(injuries, injuries_prob)[0]

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

        return random.choice(outcomes)

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.attacking_player = attacking_team.player_in_possession
        self.defending_player = defending_team.get_player_on_pitch(self.state.position)

        self.foul_type = self.get_foul_type()

        if self.foul_type == FoulTypes.OFFENSIVE_FOUL:
            fouled_player = self.defending_player
            offending_player = self.attacking_player
        else:
            fouled_player = self.attacking_player
            offending_player = self.defending_player

        offending_player.statistics.fouls += 1

        injury_type = self.get_player_injury(offending_player, fouled_player)
        fouled_player.injury_type = injury_type

        if injury_type in [
            PlayerInjury.SEVERE_INJURY,
            PlayerInjury.CAREER_ENDING_INJURY,
        ]:
            self.outcome = EventOutcome.FOUL_RED_CARD
        else:
            self.outcome = self.get_player_card(injury_type)

        if self.outcome == EventOutcome.FOUL_YELLOW_CARD:
            offending_player.statistics.yellow_cards += 1
            print(f"Player {offending_player} received a yellow card!")

        if offending_player.statistics.yellow_cards == 2:
            self.outcome = EventOutcome.FOUL_RED_CARD
            print(
                f"Player {offending_player} now has 2 yellow cards! That's a send off!"
            )

        if self.outcome == EventOutcome.FOUL_RED_CARD:
            offending_player.statistics.red_cards += 1
            print(f"Player {offending_player} received a red card!")

        return GameState(self.state.minutes, self.state.position)


@dataclass
class CornerKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Corner {self.state.position}")
        return GameState(self.state.minutes, self.state.position)


@dataclass
class ShotEvent(SimulationEvent):
    def get_shot_saved_outcomes(self) -> EventOutcome:
        final_outcomes = [
            EventOutcome.SHOT_RIGHT_CORNER_KICK,
            EventOutcome.SHOT_LEFT_CORNER_KICK,
            EventOutcome.SHOT_SAVED_SECURED,
        ]
        print(f"{self.defending_player} saved the ball!")
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
            event_probabilities = [
                (
                    self.defending_player.attributes.defensive.positioning
                    + self.defending_player.attributes.defensive.interception
                )
                / 2
            ]

        return random.choices(basic_event_outcomes, event_probabilities)[0]

    def get_shot_blocked(self):
        outcomes = [
            EventOutcome.SHOT_BLOCKED_CHANGE,
            EventOutcome.SHOT_BLOCKED_BACK,
        ]
        outcome = random.choice(outcomes)
        print(f"{self.defending_player} blocked the shot!")

        return outcome

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        outcomes = [
            EventOutcome.SHOT_HIT_POST,
            EventOutcome.SHOT_SAVED,
            EventOutcome.GOAL,
        ]
        gk_skills = self.defending_player.attributes.gk.get_general_overall()
        probabilities = [
            100 - shot_on_goal,
            gk_skills,
            110 - gk_skills,  # Even very good goalies can let balls pass sometimes
        ]
        return random.choices(outcomes, probabilities)

    def get_shot_hit_post(self) -> EventOutcome:
        final_outcomes = [
            EventOutcome.SHOT_HIT_POST,
            EventOutcome.SHOT_HIT_POST_CHANGE,
            EventOutcome.SHOT_GOAL_KICK,
        ]
        print(f"The ball hit the post!")

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
        else:
            shot_on_goal = (
                self.attacking_player.attributes.offensive.shot_accuracy
                + self.attacking_player.attributes.offensive.shot_power
            ) / 2

        print(f"{self.attacking_player} shoots!")

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
            self.outcome = self.get_shot_hit_post()
        elif self.outcome == EventOutcome.SHOT_SAVED:
            self.defending_player.statistics.shots_saved += 1
            self.attacking_player.statistics.shots_missed += 1
            self.outcome = self.get_shot_saved_outcomes()
        elif self.outcome == EventOutcome.GOAL:
            self.attacking_player.statistics.goals += 1
            self.defending_player.statistics.goals_conceded += 1
            self.state.position = PitchPosition.MIDFIELD_CENTER
            attacking_team.add_goal(Goal(self.attacking_player, self.state.minutes))
            defending_player = defending_team.get_player_on_pitch(self.state.position)
            self.state = self.change_possession(
                attacking_team,
                defending_team,
                defending_player,
                self.state.position,
            )
        elif self.outcome == EventOutcome.SHOT_BLOCKED_CHANGE:
            self.state = self.change_possession(
                attacking_team,
                defending_team,
                self.defending_player,
                self.state.position,
            )

        if self.outcome == EventOutcome.SHOT_GOAL_KICK:
            self.state = self.change_possession(
                attacking_team,
                defending_team,
                defending_team.formation.gk,
                self.state.position,
            )
        if self.outcome == EventOutcome.SHOT_HIT_POST_CHANGE:
            self.state = self.change_possession(
                attacking_team,
                defending_team,
                self.defending_player,
                self.state.position,
            )
        elif self.outcome == EventOutcome.SHOT_SAVED_SECURED:
            self.change_possession(
                attacking_team,
                defending_team,
                self.defending_player,
                self.state.position,
            )
        elif self.outcome == EventOutcome.SHOT_RIGHT_CORNER_KICK:
            attacking_team.stats.corners += 1
            self.state.position = PitchPosition.OFF_RIGHT
        elif self.outcome == EventOutcome.SHOT_LEFT_CORNER_KICK:
            attacking_team.stats.corners += 1
            self.state.position = PitchPosition.OFF_LEFT

        attacking_team.update_stats()
        defending_team.update_stats()
        return GameState(self.state.minutes, self.state.position)


@dataclass
class CrossEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        print(f"Cross {self.state.position}")
        return GameState(self.state.minutes, self.state.position)


@dataclass
class GoalKickEvent(SimulationEvent):
    receiving_player: Optional[PlayerSimulation] = None

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.attacking_player = attacking_team.formation.gk

        team_strategy = attacking_team.team_strategy

        print(f"Goal Kick {self.state.position.name}")
        return GameState(self.state.minutes, self.state.position)


@dataclass
class FreeKickEvent(SimulationEvent):
    free_kick_type: Optional[FreeKickType] = None

    def get_free_kick_type(self):
        free_kick_types = [
            FreeKickType.PASS,
            FreeKickType.CROSS,
        ]

        if self.state.position in OFF_POSITIONS:
            free_kick_types.append(FreeKickType.DIRECT_SHOT)

        return random.choice(free_kick_types)

    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        if self.state.position in OFF_POSITIONS:
            self.attacking_player = attacking_team.get_best_free_kick_taker()
        else:
            self.attacking_player = attacking_team.get_player_on_pitch(
                self.state.position
            )

        self.free_kick_type = self.get_free_kick_type()

        if self.free_kick_type == FreeKickType.PASS:
            pass_event = PassEvent(
                EventType.FREE_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
                defending_player=self.defending_player,
            )
            return pass_event.calculate_event(attacking_team, defending_team)
        if self.free_kick_type == FreeKickType.DIRECT_SHOT:
            shot_event = ShotEvent(
                EventType.FREE_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
                defending_player=self.defending_player,
            )
            return shot_event.calculate_event(attacking_team, defending_team)
        if self.free_kick_type.CROSS:
            cross_event = CrossEvent(
                EventType.FREE_KICK,
                self.state,
                outcome=None,
                attacking_player=self.attacking_player,
                defending_player=self.defending_player,
            )
            return cross_event.calculate_event(attacking_team, defending_team)


@dataclass
class DribbleEvent(SimulationEvent):
    def calculate_event(
        self, attacking_team: TeamSimulation, defending_team: TeamSimulation
    ) -> GameState:
        self.attacking_player = attacking_team.player_in_possession
        self.defending_player = defending_team.get_player_on_pitch(self.state.position)

        # Player moves the ball according to the transition matrix
        # I'm using the pass transition matrix, we can rename it accordingly
        transition_matrix = team_pass_strategy(attacking_team.team_strategy)
        probability = transition_matrix[self.state.position.value]
        end_position = random.choices(list(PitchPosition), probability)[0]

        # Dribble distance
        distance = abs(end_position.value - self.state.position.value)

        outcomes = [
            EventOutcome.DRIBBLE_SUCCESS,
            EventOutcome.DRIBBLE_FAIL,
        ]

        dribble_success = (
            self.attacking_player.attributes.intelligence.ball_control
            + self.attacking_player.attributes.intelligence.dribbling
            + self.attacking_player.attributes.intelligence.skills
        ) / 3

        if self.attacking_player.attributes.intelligence.skills >= 80:
            dribble_success = (
                self.attacking_player.attributes.intelligence.ball_control
                + self.attacking_player.attributes.intelligence.dribbling
                + self.attacking_player.attributes.intelligence.skills * 3
            ) / 5

        outcome_probability = [
            dribble_success - distance,
            (
                self.defending_player.attributes.defensive.positioning
                + self.defending_player.attributes.defensive.tackling
                + self.defending_player.attributes.physical.strength
            )
            / 3,
        ]

        self.outcome = random.choices(outcomes, outcome_probability)[0]

        if self.outcome == EventOutcome.DRIBBLE_FAIL:
            attacking_team.in_possession = False
            attacking_team.player_in_possession = None
            defending_team.player_in_possession = self.defending_player
            defending_team.in_possession = True
            end_position = PITCH_EQUIVALENTS[end_position]
        return GameState(self.state.minutes, end_position)


@dataclass
class PenaltyKickEvent(SimulationEvent):
    def calculate_event(
        self,
        attacking_team: TeamSimulation,
        defending_team: TeamSimulation,
    ) -> GameState:
        self.defending_player = defending_team.formation.gk
        self.attacking_player = attacking_team.get_best_penalty_taker()

        outcomes = [EventOutcome.SHOT_ON_GOAL, EventOutcome.SHOT_MISS]

        shot_success = (
            self.attacking_player.attributes.offensive.penalty * 2
            + self.attacking_player.attributes.offensive.shot_accuracy
        ) / 3

        outcome_probability = [
            shot_success,
            110 - shot_success,
        ]

        self.attacking_player.statistics.shots += 1

        outcome = random.choices(outcomes, outcome_probability)[0]

        if outcome == EventOutcome.SHOT_ON_GOAL:
            self.attacking_player.statistics.shots_on_target += 1
            outcomes = [EventOutcome.SHOT_SAVED, EventOutcome.GOAL]
            gk_skills = (
                self.defending_player.attributes.gk.penalty * 2
                + self.defending_player.attributes.gk.jumping
            ) / 3
            outcome_probability = [
                gk_skills,
                125 - gk_skills,
            ]
            self.outcome = random.choices(outcomes, outcome_probability)[0]

        if self.outcome == EventOutcome.SHOT_SAVED:
            outcomes = [
                EventOutcome.SHOT_LEFT_CORNER_KICK,
                EventOutcome.SHOT_RIGHT_CORNER_KICK,
                EventOutcome.SHOT_SAVED_SECURED,
            ]
            self.defending_player.statistics.shots_saved += 1
            self.outcome = random.choice(outcomes)
        if self.outcome == EventOutcome.SHOT_MISS:
            outcomes = [EventOutcome.SHOT_HIT_POST, EventOutcome.SHOT_GOAL_KICK]
            self.attacking_player.statistics.shots_missed += 1
            self.outcome = random.choice(outcomes)
        elif self.outcome == EventOutcome.SHOT_SAVED_SECURED:
            self.change_possession(
                attacking_team,
                defending_team,
                self.defending_player,
                self.state.position,
            )
        elif self.outcome == EventOutcome.SHOT_RIGHT_CORNER_KICK:
            self.state.position = PitchPosition.OFF_RIGHT
        elif self.outcome == EventOutcome.SHOT_LEFT_CORNER_KICK:
            self.state.position = PitchPosition.OFF_LEFT

        return GameState(self.state.minutes, self.state.position)
