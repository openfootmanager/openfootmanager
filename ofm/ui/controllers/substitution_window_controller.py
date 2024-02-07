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
from ttkbootstrap import Toplevel

from ...core.football.formation import FORMATION_STRINGS
from ...core.football.team_simulation import PlayerSimulation, TeamSimulation
from ...core.simulation.live_game_manager import LiveGameManager
from ..pages.debug_match.substitution_window import SubstitutionWindow


class SubstitutionWindowController:
    def __init__(
            self, parent: Toplevel, team: TeamSimulation, live_game_manager: LiveGameManager
    ):
        self.page = SubstitutionWindow(parent)
        self.team = team
        self.live_game_manager = live_game_manager
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

    def update_formation_table(self):
        player_data = self.get_player_data(self.team.formation.players)
        self.page.update_team_table(player_data)

    def update_reserves_table(self):
        player_data = self.get_player_data(self.team.formation.bench)
        self.page.update_reserves_table(player_data)

    def apply_changes(self):
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
