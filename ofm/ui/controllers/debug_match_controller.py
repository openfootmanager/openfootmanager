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

from ..gui import GUI
from ..pages.debug_match import DebugMatchPage
from ...core.db.database import DB
from ...core.football.club import TeamSimulation
from ...core.football.player import PlayerSimulation


class DebugMatchController:
    def __init__(self, gui: GUI, page: DebugMatchPage, db: DB):
        self.gui = gui
        self.page = page
        self.db = db
        self.teams = None
        self._bind()

    def load_random_teams(self):
        """
        Naive implementation of loading random teams. We can transfer this to a
        Core module later to avoid having the low-level implementation on the Controller side.
        """
        # Creates files if they don't exist
        self.db.check_clubs_file()

        clubs = self.db.load_clubs()
        players = self.db.load_players()
        players_obj = self.db.load_player_objects(players)
        clubs_obj = self.db.load_club_objects(clubs, players_obj)

        teams = random.sample(clubs_obj, 2)
        teams = [TeamSimulation.get_from_club(team) for team in teams]
        return teams

    def get_player_data(self, team: TeamSimulation):
        return [
            (
                player.player.details.short_name,
                player.current_position.name,
                player.current_stamina,
                player.current_skill
            )
            for player in team.players
        ]

    def update_player_table(self):
        home_team = self.get_player_data(self.teams[0])
        away_team = self.get_player_data(self.teams[1])

        self.page.update_tables(home_team, away_team)

    def go_to_debug_home_page(self):
        self.gui.switch("debug_home")

    def _bind(self):
        self.page.cancel_btn.config(command=self.go_to_debug_home_page)
