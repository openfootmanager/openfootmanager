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
import uuid
from datetime import timedelta
from threading import Thread
from typing import Optional

from ...core.db.database import DB
from ...core.football.formation import Formation
from ...core.football.player import PlayerSimulation
from ...core.football.team_simulation import TeamSimulation, TeamStrategy
from ...core.simulation.event import CommentaryImportance
from ...core.simulation.fixture import Fixture
from ...core.simulation.simulation import DelayValue, LiveGame, SimulationStatus
from ..pages.debug_match import CommentaryVerbosity, DebugMatchPage, DelayComboBoxValues
from .controllerinterface import ControllerInterface


class DebugMatchController(ControllerInterface):
    def __init__(self, controller: ControllerInterface, page: DebugMatchPage, db: DB):
        self.controller = controller
        self.page = page
        self.db = db
        self.teams: Optional[list[TeamSimulation]] = None
        self.live_game: Optional[LiveGame] = None
        self.game_thread: Optional[Thread] = None
        self._bind()

    def switch(self, page: str):
        self.controller.switch(page)

    def initialize(self):
        self.teams = self.load_random_teams()
        self.live_game = None
        self.update_game_data()

    def start_simulation(self):
        if self.live_game is None:
            fixture = Fixture(
                uuid.uuid4(),
                uuid.uuid4(),
                self.teams[0].club.club_id,
                self.teams[1].club.club_id,
                self.teams[0].club.stadium,
            )
            self.live_game = LiveGame(
                fixture,
                self.teams[0],
                self.teams[1],
                False,
                False,
                True,
                delay=DelayValue.NONE,
            )

        self.page.disable_button()
        if not self.live_game.is_game_over:
            self.live_game.run()

    def start_match(self):
        if self.teams is None:
            return
        try:
            self.game_thread = Thread(target=self.start_simulation, daemon=True)
            self.game_thread.start()
        except RuntimeError as e:
            print(e)
        self.check_thread_status()

    def check_thread_status(self):
        if self.game_thread is None:
            return
        if self.game_thread.is_alive():
            self.update_game_data()
            self.page.after(100, lambda: self.check_thread_status())
        else:
            self.page.enable_button()
            self.game_thread = None

    def update_game_data(self):
        self.update_player_table()
        self.update_team_strategy()
        self.update_live_game_events()
        self.update_game_events()
        self.update_game_time()
        self.update_game_delay()

    def load_random_teams(self) -> list[TeamSimulation]:
        """
        Naive implementation of loading random teams. We can transfer this to a
        Core module later to avoid having the low-level implementation on the Controller side.
        """
        # Creates files if they don't exist
        self.db.check_clubs_file(amount=50)

        clubs = self.db.load_clubs()
        players = self.db.load_players()

        clubs = random.sample(clubs, 2)
        team1, team2 = self.db.load_club_objects(clubs, players)

        formation_team1 = Formation(team1.default_formation)
        formation_team2 = Formation(team2.default_formation)

        formation_team1.get_best_players(team1.squad)
        formation_team2.get_best_players(team2.squad)

        return [
            TeamSimulation(
                team1, formation_team1, strategy=random.choice(list(TeamStrategy))
            ),
            TeamSimulation(
                team2, formation_team2, strategy=random.choice(list(TeamStrategy))
            ),
        ]

    def get_player_data(self, players: list[PlayerSimulation]):
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

    def get_team_stats(self, team: TeamSimulation):
        if team.stats.passes > 0:
            pass_accuracy = int(
                ((team.stats.passes - team.stats.passes_missed) / team.stats.passes)
                * 100
            )
        else:
            pass_accuracy = 0

        if team.stats.crosses > 0:
            cross_accuracy = int(
                ((team.stats.crosses - team.stats.crosses_missed) / team.stats.crosses)
                * 100
            )
        else:
            cross_accuracy = 0

        if self.live_game is not None:
            if self.live_game.minutes != timedelta(seconds=0):
                minutes = self.live_game.total_elapsed_time.total_seconds()
                possession = (team.stats.possession / float(minutes)) * 100
                possession = f"{possession:.2f}%"
            else:
                possession = "0%"
        else:
            possession = "0%"
        return [
            team.stats.shots,
            team.stats.shots_on_target,
            possession,
            team.stats.passes,
            pass_accuracy,
            team.stats.crosses,
            cross_accuracy,
            team.stats.fouls,
            team.stats.yellow_cards,
            team.stats.red_cards,
            team.stats.offsides,
            team.stats.corners,
        ]

    def update_player_table(self):
        if self.teams is None:
            return

        home_team = self.get_player_data(self.teams[0].formation.players)
        away_team = self.get_player_data(self.teams[1].formation.players)
        home_reserves = self.get_player_data(self.teams[1].formation.bench)
        away_reserves = self.get_player_data(self.teams[1].formation.bench)

        self.page.update_tables(home_team, away_team, home_reserves, away_reserves)
        self.page.update_team_names(
            self.teams[0].club.name,
            self.teams[1].club.name,
            str(self.teams[0].score),
            str(self.teams[1].score),
        )
        home_team_stats = self.get_team_stats(self.teams[0])
        away_team_stats = self.get_team_stats(self.teams[1])
        self.page.update_team_stats(home_team_stats, away_team_stats)

    def update_team_strategy(self):
        if self.teams:
            self.page.update_team_strategy(
                self.teams[0].team_strategy.name, self.teams[1].team_strategy.name
            )

    def update_game_time(self):
        if not self.live_game:
            self.page.progress_bar["maximum"] = 90 * 60
            self.page.progress_bar["value"] = 0
            self.page.minutes_elapsed.config(text="0'")
            return
        if self.live_game.state.status == SimulationStatus.SECOND_HALF_BREAK:
            self.page.progress_bar["maximum"] = 120 * 60
        self.page.progress_bar["value"] = self.live_game.minutes.total_seconds()
        self.page.minutes_elapsed.config(
            text=f"{int(self.live_game.minutes.total_seconds() / 60)}'"
        )

    def update_game_delay(self):
        delay = self.page.delay_box.get()
        if self.live_game:
            match delay:
                case DelayComboBoxValues.NONE.value:
                    self.live_game.delay = DelayValue.NONE
                case DelayComboBoxValues.MEDIUM.value:
                    self.live_game.delay = DelayValue.MEDIUM
                case DelayComboBoxValues.SHORT.value:
                    self.live_game.delay = DelayValue.SHORT
                case DelayComboBoxValues.LONG.value:
                    self.live_game.delay = DelayValue.LONG
                case DelayComboBoxValues.VERY_LONG.value:
                    self.live_game.delay = DelayValue.VERY_LONG

    def update_commentary_verbosity(self) -> list[CommentaryImportance]:
        commentary_verbosity = self.page.commentary_box.get()
        if self.live_game:
            match commentary_verbosity:
                case CommentaryVerbosity.ALL.value:
                    return list(CommentaryImportance)
                case CommentaryVerbosity.HIGHLIGHTS.value:
                    return [CommentaryImportance.MEDIUM, CommentaryImportance.HIGH]
                case CommentaryVerbosity.SHOTS_ONLY.value:
                    return [CommentaryImportance.HIGH]

        return list(CommentaryImportance)

    def update_live_game_events(self):
        if not self.live_game:
            self.page.update_live_game([])
            return

        events = []
        for event in self.live_game.engine.event_history:
            minutes = event.state.minutes.total_seconds() / 60
            commentary = ""
            for comment in event.commentary:
                commentary_verbosity = self.update_commentary_verbosity()
                if event.commentary_importance in commentary_verbosity:
                    commentary += comment + "\n"
            if commentary:
                events.append(f"{int(minutes)}' - {commentary}")

        self.page.update_live_game(events)

    def update_game_events(self):
        # TODO: Add yellow and red cards and substitutions
        home_team_events = []
        away_team_events = []
        if self.live_game:
            for event in self.live_game.engine.home_team.game_events:
                text = ""
                if event in self.live_game.engine.home_team.goals_history:
                    text = f"⚽ {event.__repr__()}"
                if event in self.live_game.engine.home_team.yellow_card_history:
                    text = f"🟨Y {event.__repr__()}"
                if event in self.live_game.engine.home_team.red_card_history:
                    text = f"🟥R {event.__repr__()}"

                home_team_events.append(text)

            for event in self.live_game.engine.away_team.game_events:
                text = ""
                if event in self.live_game.engine.away_team.goals_history:
                    text = f"⚽ {event.__repr__()}"
                if event in self.live_game.engine.away_team.yellow_card_history:
                    text = f"🟨Y {event.__repr__()}"
                if event in self.live_game.engine.away_team.red_card_history:
                    text = f"🟥R {event.__repr__()}"

                away_team_events.append(text)

        self.page.update_game_events(home_team_events, away_team_events)

    def go_to_debug_home_page(self):
        self.switch("debug_home")

    def _bind(self):
        self.page.play_game_btn.config(command=self.start_match)
        self.page.new_game_btn.config(command=self.initialize)
        self.page.cancel_btn.config(command=self.go_to_debug_home_page)
