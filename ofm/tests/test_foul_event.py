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
from datetime import timedelta

from ofm.core.football.player import PlayerInjury
from ofm.core.simulation.event import EventOutcome, EventType, PitchPosition
from ofm.core.simulation.event_type import FoulType
from ofm.core.simulation.events import FoulEvent
from ofm.core.simulation.game_state import GameState, SimulationStatus


def get_foul_event() -> FoulEvent:
    return FoulEvent(
        EventType.FOUL,
        GameState(
            timedelta(minutes=10),
            SimulationStatus.FIRST_HALF,
            PitchPosition.MIDFIELD_CENTER,
        ),
    )


def test_no_card_foul_event(simulation_teams, monkeypatch):
    def get_foul_type(self) -> FoulType:
        return FoulType.DEFENSIVE_FOUL

    def get_player_injury(self, *args, **kwargs) -> PlayerInjury:
        return PlayerInjury.NO_INJURY

    def get_player_card(self, *args, **kwargs) -> EventOutcome:
        return EventOutcome.FOUL_WARNING

    monkeypatch.setattr(FoulEvent, "get_foul_type", get_foul_type)
    monkeypatch.setattr(FoulEvent, "get_player_injury", get_player_injury)
    monkeypatch.setattr(FoulEvent, "get_player_card", get_player_card)
    event = get_foul_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.defending_player.statistics.fouls == 1
    assert event.defending_player.statistics.yellow_cards == 0
    assert event.defending_player.statistics.red_cards == 0
    assert event.attacking_player.injury_type == PlayerInjury.NO_INJURY
    assert event.attacking_player.is_injured is False


def test_yellow_card_foul_event(simulation_teams, monkeypatch):
    def get_foul_type(self) -> FoulType:
        return FoulType.DEFENSIVE_FOUL

    def get_player_injury(self, *args, **kwargs) -> PlayerInjury:
        return PlayerInjury.NO_INJURY

    def get_player_card(self, *args, **kwargs) -> EventOutcome:
        return EventOutcome.FOUL_YELLOW_CARD

    monkeypatch.setattr(FoulEvent, "get_foul_type", get_foul_type)
    monkeypatch.setattr(FoulEvent, "get_player_injury", get_player_injury)
    monkeypatch.setattr(FoulEvent, "get_player_card", get_player_card)
    event = get_foul_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.defending_player.statistics.fouls == 1
    assert event.defending_player.statistics.yellow_cards == 1
    assert event.defending_player.statistics.red_cards == 0
    assert event.attacking_player.injury_type == PlayerInjury.NO_INJURY
    assert event.attacking_player.is_injured is False


def test_two_yellow_cards_foul_event(simulation_teams, monkeypatch):
    def get_foul_type(self) -> FoulType:
        return FoulType.DEFENSIVE_FOUL

    def get_player_injury(self, *args, **kwargs) -> PlayerInjury:
        return PlayerInjury.NO_INJURY

    def get_player_card(self, *args, **kwargs) -> EventOutcome:
        return EventOutcome.FOUL_YELLOW_CARD

    monkeypatch.setattr(FoulEvent, "get_foul_type", get_foul_type)
    monkeypatch.setattr(FoulEvent, "get_player_injury", get_player_injury)
    monkeypatch.setattr(FoulEvent, "get_player_card", get_player_card)
    event = get_foul_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    for player in away_team.formation.players:
        player.statistics.yellow_cards = 1
    event.calculate_event(home_team, away_team)
    assert event.defending_player.statistics.fouls == 1
    assert event.defending_player.statistics.yellow_cards == 2
    assert event.defending_player.statistics.red_cards == 1
    assert event.attacking_player.injury_type == PlayerInjury.NO_INJURY
    assert event.attacking_player.is_injured is False


def test_red_card_foul_event(simulation_teams, monkeypatch):
    def get_foul_type(self) -> FoulType:
        return FoulType.DEFENSIVE_FOUL

    def get_player_injury(self, *args, **kwargs) -> PlayerInjury:
        return PlayerInjury.NO_INJURY

    def get_player_card(self, *args, **kwargs) -> EventOutcome:
        return EventOutcome.FOUL_RED_CARD

    monkeypatch.setattr(FoulEvent, "get_foul_type", get_foul_type)
    monkeypatch.setattr(FoulEvent, "get_player_injury", get_player_injury)
    monkeypatch.setattr(FoulEvent, "get_player_card", get_player_card)
    event = get_foul_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.defending_player.statistics.fouls == 1
    assert event.defending_player.statistics.yellow_cards == 0
    assert event.defending_player.statistics.red_cards == 1
    assert event.attacking_player.injury_type == PlayerInjury.NO_INJURY
    assert event.attacking_player.is_injured is False


def test_severe_injury_red_card_foul_event(simulation_teams, monkeypatch):
    def get_foul_type(self) -> FoulType:
        return FoulType.DEFENSIVE_FOUL

    def get_player_injury(self, *args, **kwargs) -> PlayerInjury:
        return PlayerInjury.SEVERE_INJURY

    monkeypatch.setattr(FoulEvent, "get_foul_type", get_foul_type)
    monkeypatch.setattr(FoulEvent, "get_player_injury", get_player_injury)
    event = get_foul_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.defending_player.statistics.fouls == 1
    assert event.defending_player.statistics.yellow_cards == 0
    assert event.defending_player.statistics.red_cards == 1
    assert event.attacking_player.injury_type == PlayerInjury.SEVERE_INJURY
    assert event.attacking_player.is_injured is True


def test_career_ending_injury_red_card_foul_event(simulation_teams, monkeypatch):
    def get_foul_type(self) -> FoulType:
        return FoulType.DEFENSIVE_FOUL

    def get_player_injury(self, *args, **kwargs) -> PlayerInjury:
        return PlayerInjury.CAREER_ENDING_INJURY

    monkeypatch.setattr(FoulEvent, "get_foul_type", get_foul_type)
    monkeypatch.setattr(FoulEvent, "get_player_injury", get_player_injury)
    event = get_foul_event()
    home_team, away_team = simulation_teams
    home_team.in_possession = True
    home_team.player_in_possession = home_team.get_player_on_pitch(event.state.position)
    away_team.in_possession = False
    away_team.player_in_possession = None
    event.calculate_event(home_team, away_team)
    assert event.defending_player.statistics.fouls == 1
    assert event.defending_player.statistics.yellow_cards == 0
    assert event.defending_player.statistics.red_cards == 1
    assert event.attacking_player.injury_type == PlayerInjury.CAREER_ENDING_INJURY
    assert event.attacking_player.is_injured is True
