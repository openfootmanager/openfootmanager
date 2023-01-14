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
from datetime import datetime, date, timedelta
import json
import random
import uuid
from abc import ABC, abstractmethod
from typing import Tuple, List, Optional, Union
from ofm.core.football.player import Player, Positions, PreferredFoot, PlayerTeam
from ofm.core.football.playercontract import PlayerContract
from ofm.core.football.club import Club, ClubSquad
from ofm.defaults import NAMES_FILE


class Generator(ABC):
    @abstractmethod
    def generate(self, *args):
        pass


class GeneratePlayerError(Exception):
    pass


class PlayerGenerator(Generator):
    def __init__(self, today: Union[datetime, date] = date.today(), max_age: int = 35, min_age: int = 16,
                 max_skill_lvl: int = 99):
        if min_age > max_age:
            raise GeneratePlayerError("Minimum age must not be greater than maximum age!")

        self.players_obj: List[Player] = []
        self.nationalities = self._get_nationalities()
        self.names = self._get_names()

        year = timedelta(seconds=31556952)  # definition of a Gregorian calendar date
        self.today = today
        self.max_age = max_age * year
        self.min_age = min_age * year
        self.max_skill_lvl = max_skill_lvl

    @staticmethod
    def _get_nationalities():
        with open(NAMES_FILE, "r", encoding="utf-8") as fp:
            data = json.load(fp)
            return [d["region"] for d in data]

    @staticmethod
    def _get_names():
        with open(NAMES_FILE, "r", encoding="utf-8") as fp:
            return json.load(fp)

    def _get_names_from_region(self, region: str) -> dict:
        for reg in self.names:
            if reg["region"] == region:
                return reg

    def generate_id(self):
        return uuid.uuid4()

    def generate_nationality(self, nat: Optional[str]) -> str:
        """
        Returns the player's nationality. If you define a nationality for any reason,
        you should get this nationality here.
        """
        return nat or random.choice(self.nationalities)

    def generate_dob(self) -> datetime:
        """
        Generates the player's date of birth
        """
        min_year = self.today - self.max_age  # minimum date for birthday
        max_year = self.today - self.min_age  # max date for birthday

        days_interval = max_year - min_year
        rand_date = random.randrange(
            days_interval.days
        )  # chooses a random date from the max days interval
        return min_year + timedelta(days=rand_date)  # assigns date of birth

    def generate_name(self, region: Optional[str]) -> Tuple[str, str, str]:
        if not region:
            region = random.choice(self.nationalities)
        names = self._get_names_from_region(region)
        first_name = random.choice(names["male"])
        last_name = random.choice(names["surnames"])
        short_name = f'{first_name[0]}. {last_name}'
        # TODO: Generate some nicknames for players, but for now just keep it that way
        return first_name, last_name, short_name

    def generate_skill(self, mu: int = 50, sigma: int = 20) -> int:
        """
        Generates the player's skill lvl. Region-tuned skill-lvl might come later,
        but for now, just generates players with skill lvls from 30 to 99.

        I'm capping skill lvls to not return negative values or values above 99.

        The planned skill rating should go from 0 to 99 in this simulation, just like other soccer games do.
        """
        if mu is None:
            mu = 50

        if sigma is None:
            sigma = 20

        skill = int(random.gauss(mu, sigma))

        skill = min(skill, self.max_skill_lvl)
        skill = max(30, skill)

        return skill

    def generate_potential_skill(self, skill: int, age: int) -> int:
        """
        Generates the player's potential skill.
        """
        # TODO: improve this algorithm
        return random.randint(skill, self.max_skill_lvl)

    def generate_positions(self, desired_pos: Optional[List[Positions]]) -> list[Positions]:
        if desired_pos:  # might be useful if we want to generate teams later, so we don't get entirely random positions
            return desired_pos
        positions = list(Positions)
        return random.choices(positions)  # very naive implementation, I will improve it later

    @staticmethod
    def generate_preferred_foot() -> PreferredFoot:
        return random.choice(list(PreferredFoot))

    def generate_player_value(self, skill: int) -> float:
        """
        Should return how much a player's worth.

        Right now I'm just going to say it is skill * 1000.00. It's not too important to come up
        with an algorithm for that at the moment.
        :param skill:
        :return:
        """
        # TODO: Implement an algorithm to calculate player value
        # This algorithm should take into account the player's international reputation,
        # The potential skill value and the current skill of the player
        return skill * 1000.00

    def generate_international_reputation(self, skill: int) -> int:
        """
        Returns the player's international reputation. This number ranges from 0 to 5.
        """
        return random.randint(0, 5)

    def get_players_dictionaries(self) -> List[dict]:
        if not self.players_obj:
            raise GeneratePlayerError("Players objects were not generated!")
        return [player.serialize() for player in self.players_obj]

    def generate_player(
            self,
            region: Optional[str] = None,
            mu: Optional[int] = 50,
            sigma: Optional[int] = 20,
            desired_pos: Optional[List[Positions]] = None
    ) -> Player:
        player_id = self.generate_id()
        nationality = self.generate_nationality(region)
        first_name, last_name, short_name = self.generate_name(region)
        dob = self.generate_dob()
        age = int((self.today - dob).days * 0.0027379070)
        positions = self.generate_positions(desired_pos)
        preferred_foot = self.generate_preferred_foot()
        skill = self.generate_skill(mu, sigma)
        potential_skill = self.generate_potential_skill(skill, age)
        international_reputation = self.generate_international_reputation(skill)
        value = self.generate_player_value(skill)

        return Player(
            player_id,
            nationality,
            dob,
            first_name,
            last_name,
            short_name,
            positions,
            100.0,
            100.0,
            0.5,
            skill,
            potential_skill,
            international_reputation,
            preferred_foot,
            value,
        )

    def generate(self, amount: int, region: Optional[str] = None, desired_pos: Optional[List[Positions]] = None):
        self.players_obj = [self.generate_player(region, desired_pos) for _ in range(amount)]


class GenerateSquadError(Exception):
    pass


class TeamGenerator(Generator):
    """
    Teams are defined in a definition file.

    The definition file is a list of teams. However, teams do not contain a squad by default,
    and a squad should be generated for each team.
    """

    def __init__(self, team_definitions: list[dict], squad_definitions: list[dict], season_start: date):
        self.team_definitions = team_definitions
        self.season_start = season_start
        self.squad_definitions = squad_definitions
        self.player_gen = PlayerGenerator()

    def _get_nationalities(self, squad_definition: dict) -> Tuple[list[str], list[float]]:
        nationalities = []
        probabilities = []
        for nat in squad_definition["nationalities"]:
            nationalities.append(nat["name"])
            probabilities.append(nat["probability"])

        return nationalities, probabilities

    def generate_player_contract(self, player: Player) -> PlayerContract:
        wage = player.value / 12
        contract_started = self.season_start
        contract_length = random.randint(1, 4) * timedelta(seconds=31556952)   # pick a contract length in years
        contract_end = contract_started + contract_length
        bonus_for_goal = 0
        bonus_for_def = 0
        if any(x in player.positions for x in [Positions.FW, Positions.MF]):
            bonus_for_goal = player.value * ((player.skill / 2) / 100)
        if any(x in player.positions for x in [Positions.GK, Positions.DF]):
            bonus_for_def = player.value * ((player.skill / 2) / 100)

        return PlayerContract(wage, contract_started, contract_end, bonus_for_goal, bonus_for_def)

    def generate_squad(self, team: Club, squad_definition: dict) -> ClubSquad:
        # A team must have at least 2 GKs, 6 defenders, 6 midfielders and 4 forwards to play
        needed_positions = [
            Positions.GK,
            Positions.DF,
            Positions.DF,
            Positions.DF,
            Positions.DF,
            Positions.MF,
            Positions.MF,
            Positions.MF,
            Positions.MF,
            Positions.FW,
            Positions.FW,
            # Reserves
            Positions.GK,
            Positions.DF,
            Positions.DF,
            Positions.MF,
            Positions.MF,
            Positions.FW,
            Positions.FW,
        ]

        # Variables for player generation
        shirt_number = 1
        mu = squad_definition["mu"]
        sigma = squad_definition["sigma"]
        nationalities, probabilities = self._get_nationalities(squad_definition)
        players = []

        # Generate players for squad
        for position in needed_positions:
            nationality = random.choices(nationalities, probabilities)[0]
            players.append(self.player_gen.generate_player(nationality, mu, sigma, [position]))

        # Generate the PlayerTeam object
        squad = []
        for player in players:
            squad.append(PlayerTeam(
                player,
                team.club_id,
                shirt_number,
                self.generate_player_contract(player)
            ))
            shirt_number += 1

        return ClubSquad(team, squad)

    def generate(self, *args):
        teams = [Club.get_from_dict(team) for team in self.team_definitions]
        team_squads = []
        for team in teams:
            found = False
            for squad_def in self.squad_definitions:
                if team.club_id.int == squad_def["id"]:
                    found = True
                    team_squads.append(self.generate_squad(team, squad_def))
                    break

            if not found:
                raise GenerateSquadError(f"Squad definition not found for team {team.name}")

        if not team_squads:
            raise GenerateSquadError(f"Team Squads are empty!")

        return team_squads
