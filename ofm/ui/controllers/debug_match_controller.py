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

from .controllerinterface import ControllerInterface
from ..pages.debug_match import DebugMatchPage
from ...core.db.database import DB
from ...core.football.club import TeamSimulation
from ...core.football.player import PlayerSimulation
from ...core.football.formation import Formation


class DebugMatchController(ControllerInterface):
    def __init__(self, controller: ControllerInterface, page: DebugMatchPage, db: DB):
        self.controller = controller
        self.page = page
        self.db = db
        self.teams = None
        self._bind()

    def switch(self, page: str):
        self.controller.switch(page)

    def initialize(self):
        self.teams = self.load_random_teams()
        self.update_player_table()

    def load_random_teams(self) -> list[TeamSimulation]:
        """
        Naive implementation of loading random teams. We can transfer this to a
        Core module later to avoid having the low-level implementation on the Controller side.
        """
        # Creates files if they don't exist
        self.db.check_clubs_file()

        clubs = self.db.load_clubs()
        players = self.db.load_players()

        clubs = random.sample(clubs, 2)
        team1, team2 = self.db.load_club_objects(clubs, players)

        formation_team1 = Formation(team1.default_formation)
        formation_team2 = Formation(team2.default_formation)

        formation_team1.get_best_players(team1.squad)
        formation_team2.get_best_players(team2.squad)

        return [
            TeamSimulation(team1, formation_team1),
            TeamSimulation(team2, formation_team2),
        ]

    def get_player_data(self, team: TeamSimulation):
        return [
            (
                player.player.details.short_name.encode("utf-8").decode(
                    "unicode_escape"
                ),
                player.current_position.name.encode("utf-8").decode("unicode_escape"),
                player.current_stamina,
                player.current_skill,
            )
            for player in team.formation.players
        ]

    def get_reserve_players(self, team: TeamSimulation):
        return [
            (
                player.player.details.short_name.encode("utf-8").decode(
                    "unicode_escape"
                ),
                player.current_position.name.encode("utf-8").decode("unicode_escape"),
                player.current_stamina,
                player.current_skill,
            )
            for player in team.formation.bench
        ]

    def update_player_table(self):
        home_team = self.get_player_data(self.teams[0])
        away_team = self.get_player_data(self.teams[1])
        home_reserves = self.get_reserve_players(self.teams[0])
        away_reserves = self.get_reserve_players(self.teams[1])

        self.page.update_tables(home_team, away_team, home_reserves, away_reserves)
        self.page.update_team_names(
            self.teams[0].club.name.encode("utf-8").decode("unicode_escape"),
            self.teams[1].club.name.encode("utf-8").decode("unicode_escape"),
        )

    def go_to_debug_home_page(self):
        self.switch("debug_home")

    def _bind(self):
        self.page.new_game_btn.config(command=self.initialize)
        self.page.cancel_btn.config(command=self.go_to_debug_home_page)
