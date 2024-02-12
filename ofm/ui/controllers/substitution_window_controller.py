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
from abc import ABC, abstractmethod
from copy import deepcopy
from dataclasses import dataclass
from datetime import timedelta
from enum import Enum, auto

from ttkbootstrap import Toplevel

from ...core.football.formation import FORMATION_STRINGS
from ...core.football.team_simulation import PlayerSimulation, TeamSimulation
from ...core.simulation.live_game_manager import LiveGameManager
from ..pages.debug_match.substitution_window import SubstitutionWindow


class Command(ABC):
    @abstractmethod
    def execute(self, *args):
        return NotImplemented


@dataclass
class SubstitutionCommand(Command):
    player_in: PlayerSimulation
    player_out: PlayerSimulation
    time: timedelta
    additional_time: timedelta

    def execute(self, team: TeamSimulation):
        team.sub_player(
            self.player_out, self.player_in, self.time, self.additional_time
        )


@dataclass
class FormationChangeCommand(Command):
    formation_string: str

    def execute(self, team: TeamSimulation):
        team.formation.change_formation(self.formation_string)


class SubstitutionWindowController:
    def __init__(
        self, parent: Toplevel, team: TeamSimulation, live_game_manager: LiveGameManager
    ):
        self.page = SubstitutionWindow(parent)
        self.team = deepcopy(team)
        self.live_game_manager = live_game_manager
        self.commands: list[Command] = []
        self.initialize()
        self._bind()

    @property
    def live_game(self):
        return self.live_game_manager.live_game

    @live_game.setter
    def live_game(self, value):
        self.live_game_manager.live_game = value

    def initialize(self):
        self.live_game.running = False
        self.page.update_team_name(self.team.club.name)
        self.update_formation_table()
        self.update_reserves_table()
        self.page.update_formations(FORMATION_STRINGS)
        self.page.update_formation_box(self.team.formation.formation_string)

    def get_player_data(self, players: list[PlayerSimulation]) -> list[tuple]:
        return [
            (
                player.player.details.short_name.encode("utf-8").decode(
                    "unicode_escape"
                ),
                player.current_position.name.encode("utf-8").decode("unicode_escape"),
                player.stamina,
                "Yes" if player.is_injured else "No",
                player.current_skill,
            )
            for player in players
        ]

    def update_team_formation(self, *args):
        # TODO: build Commands list with formation
        formation = self.page.formation_combobox.get()
        self.team.formation.change_formation(formation)
        self.update_formation_table()

    def update_formation_table(self):
        player_data = self.get_player_data(self.team.formation.players)
        self.page.update_team_table(player_data)

    def update_reserves_table(self):
        player_data = self.get_player_data(self.team.formation.bench)
        self.page.update_reserves_table(player_data)

    def apply_changes(self):
        # TODO: Apply commands list here
        if self.team == self.live_game.engine.home_team:
            self.live_game.engine.home_team = self.team
        else:
            self.live_game.engine.away_team = self.team

        self.return_game()

    def return_game(self):
        if not self.live_game.is_game_over:
            self.live_game.running = True

        self.page.destroy()

    def cancel(self):
        result = self.page.cancel_dialog()

        if result == self.page.get_yes_result():
            self.return_game()

    def _bind(self):
        self.page.cancel_button.config(command=self.cancel)
        self.page.apply_button.config(command=self.apply_changes)
        self.page.formation_combobox.bind(
            "<<ComboboxSelected>>", self.update_team_formation
        )
