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
import json
import random
import uuid
from abc import ABC, abstractmethod
from datetime import date, datetime, timedelta
from typing import List, Optional, Tuple, Union

from ofm.core.football.club import Club
from ofm.core.football.player import Player, PlayerTeam, Positions, PreferredFoot
from ofm.core.football.player_attributes import (
    DefensiveAttributes,
    GkAttributes,
    IntelligenceAttributes,
    OffensiveAttributes,
    PhysicalAttributes,
    PlayerAttributes,
)
from ofm.core.football.playercontract import PlayerContract
from ofm.defaults import NAMES_FILE


def generate_skill_values(mu: int, sigma: int) -> int:
    value = random.gauss(mu, sigma)
    value = min(value, 95)
    value = max(value, 25)
    return int(value)


class Generator(ABC):
    @abstractmethod
    def generate(self, *args):
        pass


class GeneratePlayerError(Exception):
    pass


class PlayerAttributeGenerator(Generator):
    def __init__(self, max_skill_lvl):
        self.max_skill_lvl = max_skill_lvl

    def generate_offensive_attributes(self, mu: int, sigma: int) -> OffensiveAttributes:
        return OffensiveAttributes(
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
        )

    def generate_defensive_attributes(self, mu: int, sigma: int) -> DefensiveAttributes:
        return DefensiveAttributes(
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
        )

    def generate_physical_attributes(self, mu: int, sigma: int) -> PhysicalAttributes:
        return PhysicalAttributes(
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
        )

    def generate_intelligence_attributes(
        self, mu: int, sigma: int
    ) -> IntelligenceAttributes:
        return IntelligenceAttributes(
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
        )

    def generate_gk_attributes(self, mu: int, sigma: int) -> GkAttributes:
        return GkAttributes(
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
            generate_skill_values(mu, sigma),
        )

    def get_gk_attributes(self, mu: int, sigma: int) -> PlayerAttributes:
        gk = self.generate_gk_attributes(mu, sigma)
        offensive = self.generate_offensive_attributes(35, 10)
        defensive = self.generate_defensive_attributes(mu, sigma)
        physical = self.generate_physical_attributes(mu, sigma)
        intelligence = self.generate_intelligence_attributes(mu, sigma)
        return PlayerAttributes(offensive, physical, defensive, intelligence, gk)

    def get_df_attributes(self, mu: int, sigma: int) -> PlayerAttributes:
        gk = self.generate_gk_attributes(35, 10)
        offensive = self.generate_offensive_attributes(45, 10)
        defensive = self.generate_defensive_attributes(mu, sigma)
        physical = self.generate_physical_attributes(mu, sigma)
        intelligence = self.generate_intelligence_attributes(mu, sigma)
        return PlayerAttributes(offensive, physical, defensive, intelligence, gk)

    def get_mf_attributes(self, mu: int, sigma: int) -> PlayerAttributes:
        gk = self.generate_gk_attributes(35, 10)
        offensive = self.generate_offensive_attributes(50, 20)
        defensive = self.generate_defensive_attributes(mu, sigma)
        physical = self.generate_physical_attributes(mu, sigma)
        intelligence = self.generate_intelligence_attributes(mu, sigma)
        return PlayerAttributes(offensive, physical, defensive, intelligence, gk)

    def get_fw_attributes(self, mu: int, sigma: int) -> PlayerAttributes:
        gk = self.generate_gk_attributes(35, 10)
        offensive = self.generate_offensive_attributes(mu, sigma)
        defensive = self.generate_defensive_attributes(45, 10)
        physical = self.generate_physical_attributes(mu, sigma)
        intelligence = self.generate_intelligence_attributes(mu, sigma)
        return PlayerAttributes(offensive, physical, defensive, intelligence, gk)

    def generate(
        self, positions: list[Positions], mu: int = 50, sigma: int = 20
    ) -> PlayerAttributes:
        """
        Generates the player's attributes. Generates players with attribute ranging from 20 to 99.

        I'm capping skill lvls to not return negative values or values above 99.

        The planned skill rating should go from 0 to 99 in this simulation, just like other soccer games do.
        """
        if mu is None:
            mu = 50

        if sigma is None:
            sigma = 20

        attributes = 0  # just to stop the IDE from complaining
        match positions[0]:
            case Positions.GK:
                attributes = self.get_gk_attributes(mu, sigma)
            case Positions.DF:
                attributes = self.get_df_attributes(mu, sigma)
            case Positions.MF:
                attributes = self.get_mf_attributes(mu, sigma)
            case Positions.FW:
                attributes = self.get_fw_attributes(mu, sigma)

        return attributes


class PlayerGenerator(Generator):
    def __init__(
        self,
        today: Union[datetime, date] = date.today(),
        max_age: int = 35,
        min_age: int = 16,
        max_skill_lvl: int = 99,
    ):
        if min_age > max_age:
            raise GeneratePlayerError(
                "Minimum age must not be greater than maximum age!"
            )

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
            return reg if reg["region"] == region else random.choice(self.names)

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
        short_name = f"{last_name}"
        # TODO: Generate some nicknames for players, but for now just keep it that way
        return first_name, last_name, short_name

    def generate_potential_skill(
        self, skill: PlayerAttributes, positions: list[Positions], age: int
    ) -> int:
        """
        Generates the player's potential skill.
        """
        age_diff = int((self.max_age.days * 365.25) - age)
        age_diff = max(age_diff, 0)
        ovr = skill.get_overall(positions[0])

        if age_diff == 0:
            potential = ovr
        else:
            potential = ovr + random.randint(0, 20)
            potential = min(potential, 99)
        return potential

    def generate_positions(
        self, desired_pos: Optional[list[Positions]]
    ) -> list[Positions]:
        if (
            desired_pos
        ):  # might be useful if we want to generate teams later, so we don't get entirely random positions
            return desired_pos
        positions = list(Positions)
        return random.choices(
            positions
        )  # very naive implementation, I will improve it later

    @staticmethod
    def generate_preferred_foot() -> PreferredFoot:
        return random.choice(list(PreferredFoot))

    def generate_player_value(
        self, skill: int, age: int, potential_skill: int, international_rep: int
    ) -> float:
        """
        Should return how much a player's worth.

        Right now I'm just going to implement a basic value. It's not too important to come up
        with an algorithm for that at the moment.
        """
        age_diff = int((self.max_age.days * 365.25) - age)
        age_diff = max(age_diff, 0)
        pot_skill = potential_skill
        base_value = random.randint(55, 80) * 100

        return round(
            (
                base_value
                + (international_rep * 150)
                + (age_diff * 100)
                + (skill * 50)
                + (pot_skill * 10)
            ),
            2,
        )

    def generate_international_reputation(self, max_skill: int) -> int:
        """
        Returns the player's international reputation. This number ranges from 0 to 5.
        """
        if max_skill < 65:
            return 0
        elif max_skill < 70:
            return 1
        elif max_skill < 75:
            return 2
        elif max_skill < 82:
            return 3
        elif max_skill < 90:
            return 4
        else:
            return 5

    def get_players_dictionaries(self) -> List[dict]:
        if not self.players_obj:
            raise GeneratePlayerError("Players objects were not generated!")
        return [player.serialize() for player in self.players_obj]

    def generate_player_form(self) -> float:
        form = round(random.random(), 2)
        form = max(0.45, form)
        return form

    def generate_player_fitness(self) -> float:
        fitness = round(random.random() * 100, 2)
        fitness = max(35.0, fitness)
        return fitness

    def generate_player(
        self,
        region: Optional[str] = None,
        mu: Optional[int] = 50,
        sigma: Optional[int] = 20,
        desired_pos: Optional[List[Positions]] = None,
    ) -> Player:
        attr_gen = PlayerAttributeGenerator(self.max_skill_lvl)
        player_id = self.generate_id()
        nationality = self.generate_nationality(region)
        first_name, last_name, short_name = self.generate_name(region)
        dob = self.generate_dob()
        age = int((self.today - dob).days * 0.0027379070)
        positions = self.generate_positions(desired_pos)
        preferred_foot = self.generate_preferred_foot()
        attributes = attr_gen.generate(positions, mu, sigma)
        potential_skill = self.generate_potential_skill(attributes, positions, age)
        international_reputation = self.generate_international_reputation(
            attributes.get_overall(positions[0]),
        )
        value = self.generate_player_value(
            attributes.get_overall(positions[0]),
            age,
            potential_skill,
            international_reputation,
        )
        form = self.generate_player_form()
        fitness = self.generate_player_fitness()

        return Player(
            player_id,
            nationality,
            dob,
            first_name,
            last_name,
            short_name,
            positions,
            fitness,
            100.0,
            form,
            attributes,
            potential_skill,
            international_reputation,
            preferred_foot,
            value,
        )

    def generate(
        self,
        amount: int,
        region: Optional[str] = None,
        desired_pos: Optional[List[Positions]] = None,
    ):
        self.players_obj = [
            self.generate_player(region, desired_pos) for _ in range(amount)
        ]


class GenerateSquadError(Exception):
    pass


class TeamGenerator(Generator):
    """
    Teams are defined in a definition file.

    The definition file is a list of teams. However, teams do not contain a squad by default,
    and a squad should be generated for each team.
    """

    def __init__(
        self,
        club_definitions: list[dict],
        fifa_confederations: list[dict],
        season_start: date = date.today(),
    ) -> None:
        self.fifa_confederations = fifa_confederations
        self.club_definitions = club_definitions
        self.season_start = season_start
        self.player_gen = PlayerGenerator()

    def _get_nationalities(
        self, country: str, countries: list
    ) -> Tuple[list[str], list[float]]:
        nationalities = []
        probabilities = []
        # native
        native: float = 0.85
        nationalities.append(country)
        probabilities.append(native)
        # foreigner
        foreigner: float = 1 - native
        coeff = int(foreigner / 0.05)
        mini_list = random.sample(countries, coeff)
        for ele in mini_list:
            nationalities.append(ele)
            probabilities.append(foreigner)

        return nationalities, probabilities

    def generate_player_contract(self, player: Player) -> PlayerContract:
        wage = player.value / 12
        contract_started = self.season_start
        contract_length = random.randint(1, 4) * timedelta(
            seconds=31556952
        )  # pick a contract length in years
        contract_end = contract_started + contract_length
        bonus_for_goal = 0
        bonus_for_def = 0
        position = player.get_best_position()
        if any(x in player.positions for x in [Positions.FW, Positions.MF]):
            bonus_for_goal = player.value * (
                (player.attributes.get_overall(position) / 2) / 100
            )
        if any(x in player.positions for x in [Positions.GK, Positions.DF]):
            bonus_for_def = player.value * (
                (player.attributes.get_overall(position) / 2) / 100
            )

        return PlayerContract(
            wage, contract_started, contract_end, bonus_for_goal, bonus_for_def
        )

    def generate_player_team(
        self,
        mu: int,
        sigma: int,
        nationality: str,
        team_id: uuid.UUID,
        shirt_number: int,
        positions: Optional[list[Positions]],
    ) -> PlayerTeam:
        player = self.player_gen.generate_player(nationality, mu, sigma, positions)
        return PlayerTeam(
            player, team_id, shirt_number, self.generate_player_contract(player)
        )

    def generate_squad(
        self, team_id: uuid.UUID, country: str, squad_definition: dict, countries: list
    ) -> list[PlayerTeam]:
        # A team must have some options for the bench, 22-23 players at least
        # Using 4-4-2 as default formation in this case, but teams might have different
        # default formations
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
            Positions.GK,
            Positions.DF,
            Positions.DF,
            Positions.DF,
            Positions.DF,
            Positions.MF,
            Positions.MF,
            Positions.MF,
            Positions.FW,
            Positions.FW,
        ]

        # Variables for player generation
        shirt_number = range(1, len(needed_positions) + 1)
        mu = squad_definition["mu"]
        sigma = squad_definition["sigma"]
        nationalities, probabilities = self._get_nationalities(country, countries)

        return [
            self.generate_player_team(
                mu,
                sigma,
                random.choices(nationalities, probabilities)[0],
                team_id,
                shirt_number[i],
                [needed_positions[i]],
            )
            for i, _ in enumerate(needed_positions)
        ]

    def extract_confederation(
        self, country: str, confederation: list[dict]
    ) -> Tuple[str, list]:
        country_conf: str = ""
        countries_list = []
        for element in confederation:
            for local in element["countries"]:
                countries_list.append(local)
            if country in element["countries"]:
                country_conf = element["region"]
        # remove club's country from list
        if country in countries_list:
            countries_list.remove(country)
        return country_conf, countries_list

    def generate(self, *args) -> list[Club]:
        clubs = []
        for club in self.club_definitions:
            club_id = uuid.uuid4()
            club.update({"id": club_id.int})
            country_conf, countries_list = self.extract_confederation(
                club["country"], self.fifa_confederations
            )
            squad = self.generate_squad(
                club_id, club["country"], club["squads_def"], countries_list
            )
            club_obj = Club.get_from_dict(club, squad)
            clubs.append(club_obj)

        return clubs
