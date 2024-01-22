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
from datetime import timedelta

from ofm.core.simulation.event import (
    EventOutcome,
    EventType,
    PitchPosition,
    TeamSimulation,
)
from ofm.core.simulation.events import PassEvent, ShotEvent
from ofm.core.simulation.game_state import GameState, SimulationStatus


def get_shot_event() -> ShotEvent:
    return ShotEvent(
        EventType.SHOT,
        GameState(
            timedelta(minutes=10), SimulationStatus.FIRST_HALF, PitchPosition.OFF_BOX
        ),
    )


def get_pass_event() -> PassEvent:
    return PassEvent(
        EventType.PASS,
        GameState(
            timedelta(minutes=10),
            SimulationStatus.FIRST_HALF,
            PitchPosition.OFF_MIDFIELD_CENTER,
        ),
    )


def test_shot_miss_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_MISS

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_GOAL_KICK
    assert state.position == PitchPosition.DEF_BOX
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert away_team.player_in_possession == away_team.formation.gk
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_missed == 1


def test_shot_blocked_change_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_BLOCKED

    def get_shot_blocked(self) -> EventOutcome:
        return EventOutcome.SHOT_BLOCKED_CHANGE_POSSESSION

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(ShotEvent, "get_shot_blocked", get_shot_blocked)
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_BLOCKED_CHANGE_POSSESSION
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_missed == 1
    assert event.defending_player.statistics.shots_blocked == 1
    assert state.position == PitchPosition.DEF_BOX
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert home_team.stats.shots == 1
    assert home_team.stats.shots_on_target == 0


def test_shot_blocked_back_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_BLOCKED

    def get_shot_blocked(self) -> EventOutcome:
        return EventOutcome.SHOT_BLOCKED_BACK

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(ShotEvent, "get_shot_blocked", get_shot_blocked)
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_BLOCKED_BACK
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_missed == 1
    assert event.defending_player.statistics.shots_blocked == 1
    assert state.position == PitchPosition.OFF_BOX
    assert home_team.in_possession is True
    assert away_team.in_possession is False
    assert home_team.stats.shots == 1
    assert home_team.stats.shots_on_target == 0


def test_shot_saved_secured_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        return EventOutcome.SHOT_SAVED

    def get_shot_saved_outcomes(self) -> EventOutcome:
        return EventOutcome.SHOT_SAVED_SECURED

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(ShotEvent, "get_shot_saved_outcomes", get_shot_saved_outcomes)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_SAVED_SECURED
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_on_target == 1
    assert away_team.formation.gk == event.defending_player
    assert event.defending_player.statistics.shots_saved == 1
    assert home_team.stats.shots == 1
    assert home_team.stats.shots_on_target == 1
    assert away_team.stats.goals_conceded == 0


def test_shot_hit_post_change_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        return EventOutcome.SHOT_HIT_POST

    def get_shot_hit_post(self) -> EventOutcome:
        return EventOutcome.SHOT_HIT_POST_CHANGE_POSSESSION

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    monkeypatch.setattr(ShotEvent, "get_shot_hit_post", get_shot_hit_post)
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_HIT_POST_CHANGE_POSSESSION
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_on_target == 1
    assert away_team.formation.gk == event.defending_player
    assert event.defending_player.statistics.shots_saved == 0
    assert home_team.stats.shots == 1
    assert home_team.stats.shots_on_target == 1
    assert away_team.stats.goals_conceded == 0
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert state.position == PitchPosition.DEF_BOX


def test_shot_hit_post_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        return EventOutcome.SHOT_HIT_POST

    def get_shot_hit_post(self) -> EventOutcome:
        return EventOutcome.SHOT_HIT_POST

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    monkeypatch.setattr(ShotEvent, "get_shot_hit_post", get_shot_hit_post)
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_HIT_POST
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_on_target == 1
    assert away_team.formation.gk == event.defending_player
    assert event.defending_player.statistics.shots_saved == 0
    assert home_team.stats.shots == 1
    assert home_team.stats.shots_on_target == 1
    assert away_team.stats.goals_conceded == 0
    assert home_team.in_possession is True
    assert away_team.in_possession is False
    assert state.position == PitchPosition.OFF_BOX


def test_shot_hit_post_goal_kick_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        return EventOutcome.SHOT_HIT_POST

    def get_shot_hit_post(self) -> EventOutcome:
        return EventOutcome.SHOT_GOAL_KICK

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    monkeypatch.setattr(ShotEvent, "get_shot_hit_post", get_shot_hit_post)
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_GOAL_KICK
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_on_target == 1
    assert away_team.formation.gk == event.defending_player
    assert event.defending_player.statistics.shots_saved == 0
    assert home_team.stats.shots == 1
    assert home_team.stats.shots_on_target == 1
    assert away_team.stats.goals_conceded == 0
    assert home_team.in_possession is False
    assert away_team.in_possession is True
    assert state.position == PitchPosition.DEF_BOX


def test_shot_saved_left_corner_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        return EventOutcome.SHOT_SAVED

    def get_shot_saved_outcomes(self) -> EventOutcome:
        return EventOutcome.SHOT_LEFT_CORNER_KICK

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(ShotEvent, "get_shot_saved_outcomes", get_shot_saved_outcomes)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_LEFT_CORNER_KICK
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_on_target == 1
    assert away_team.formation.gk == event.defending_player
    assert event.defending_player.statistics.shots_saved == 1
    assert home_team.stats.shots == 1
    assert home_team.stats.corners == 1
    assert home_team.stats.shots_on_target == 1
    assert away_team.stats.goals_conceded == 0
    assert state.position.OFF_LEFT


def test_shot_saved_right_corner_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal) -> EventOutcome:
        return EventOutcome.SHOT_SAVED

    def get_shot_saved_outcomes(self) -> EventOutcome:
        return EventOutcome.SHOT_RIGHT_CORNER_KICK

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(ShotEvent, "get_shot_saved_outcomes", get_shot_saved_outcomes)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    state = event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.SHOT_RIGHT_CORNER_KICK
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_on_target == 1
    assert away_team.formation.gk == event.defending_player
    assert event.defending_player.statistics.shots_saved == 1
    assert home_team.stats.shots == 1
    assert home_team.stats.corners == 1
    assert home_team.stats.shots_on_target == 1
    assert away_team.stats.goals_conceded == 0
    assert state.position == PitchPosition.OFF_RIGHT


def test_shot_goal_event(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal: float) -> EventOutcome:
        return EventOutcome.GOAL

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.outcome == EventOutcome.GOAL
    assert event.attacking_player.statistics.shots == 1
    assert event.attacking_player.statistics.shots_on_target == 1
    assert away_team.formation.gk == event.defending_player
    assert event.defending_player.statistics.shots_saved == 0
    assert home_team.stats.shots == 1
    assert home_team.stats.goals == 1
    assert home_team.stats.shots_on_target == 1
    assert away_team.stats.goals_conceded == 1
    assert home_team.score == 1
    assert away_team.score == 0


def test_goals_from_different_players(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal: float) -> EventOutcome:
        return EventOutcome.GOAL

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.formation.fw[0]
    first_player = home_team.player_in_possession
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    home_team.in_possession = True
    home_team.player_in_possession = home_team.formation.mf[0]
    second_player = home_team.player_in_possession
    away_team.in_possession = False
    away_team.player_in_possession = None
    event = get_shot_event()
    event.calculate_event(home_team, away_team)
    assert first_player.statistics.goals == 1
    assert second_player.statistics.goals == 1
    assert event.defending_player.statistics.goals_conceded == 2
    assert home_team.stats.goals == 2
    assert home_team.goals_history[0].player == first_player
    assert home_team.goals_history[1].player == second_player


def test_goal_with_assist(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal: float) -> EventOutcome:
        return EventOutcome.GOAL

    def get_end_position(self, attacking_team) -> PitchPosition:
        return PitchPosition.OFF_BOX

    def get_pass_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.PASS_SUCCESS

    def get_secondary_outcome(self) -> EventOutcome:
        return EventOutcome.PASS_SUCCESS

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    monkeypatch.setattr(PassEvent, "get_end_position", get_end_position)
    monkeypatch.setattr(PassEvent, "get_pass_primary_outcome", get_pass_primary_outcome)
    monkeypatch.setattr(PassEvent, "get_secondary_outcome", get_secondary_outcome)

    pass_event = get_pass_event()
    shot_event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.formation.fw[0]
    away_team.in_possession = False
    away_team.player_in_possession = None
    pass_event.calculate_event(home_team, away_team)
    assert pass_event.outcome == EventOutcome.PASS_SUCCESS
    assert pass_event.receiving_player.received_ball is not None
    assert home_team.player_in_possession == pass_event.receiving_player
    assistant = pass_event.receiving_player.received_ball
    shot_event.calculate_event(home_team, away_team)
    assert shot_event.outcome == EventOutcome.GOAL
    assert shot_event.attacking_player.statistics.goals == 1
    assert assistant.statistics.assists == 1


def test_goal_after_pass_miss(simulation_teams, monkeypatch):
    def get_shot_on_goal(
        self, shot_on_goal: float, defending_team: TeamSimulation
    ) -> EventOutcome:
        return EventOutcome.SHOT_ON_GOAL

    def get_shot_on_goal_outcomes(self, shot_on_goal: float) -> EventOutcome:
        return EventOutcome.GOAL

    def get_end_position(self, attacking_team) -> PitchPosition:
        return PitchPosition.OFF_BOX

    def get_pass_primary_outcome(self, distance) -> EventOutcome:
        return EventOutcome.PASS_MISS

    def get_intercept_prob(self) -> EventOutcome:
        return EventOutcome.PASS_MISS

    monkeypatch.setattr(ShotEvent, "get_shot_on_goal", get_shot_on_goal)
    monkeypatch.setattr(
        ShotEvent, "get_shot_on_goal_outcomes", get_shot_on_goal_outcomes
    )
    monkeypatch.setattr(PassEvent, "get_end_position", get_end_position)
    monkeypatch.setattr(PassEvent, "get_pass_primary_outcome", get_pass_primary_outcome)
    monkeypatch.setattr(PassEvent, "get_intercept_prob", get_intercept_prob)

    pass_event = get_pass_event()
    shot_event = get_shot_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.formation.fw[0]
    away_team.in_possession = False
    away_team.player_in_possession = None
    pass_event.calculate_event(home_team, away_team)
    assert pass_event.outcome == EventOutcome.PASS_MISS
    assert pass_event.receiving_player.received_ball is None
    assert home_team.player_in_possession is None
    assert away_team.player_in_possession is not None
    assert away_team.player_in_possession.received_ball is None
    shot_event.calculate_event(away_team, home_team)
    assert shot_event.outcome == EventOutcome.GOAL
    assert shot_event.attacking_player.statistics.goals == 1
    assert away_team.stats.assists == 0
