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
from abc import ABC
from dataclasses import asdict, dataclass

from .positions import Positions


class Attributes(ABC):
    @classmethod
    def get_from_dict(cls, attributes: dict[str, int]):
        return cls(**attributes)

    def serialize(self) -> dict[str, int]:
        return asdict(self)

    def get_overall(self) -> int:
        attrs = asdict(self)
        return int(sum(attrs.values()) / len(attrs))


@dataclass
class OffensiveAttributes(Attributes):
    shot_power: int
    shot_accuracy: int
    free_kick: int
    penalty: int
    positioning: int


@dataclass
class PhysicalAttributes(Attributes):
    strength: int
    aggression: int
    endurance: int


@dataclass
class DefensiveAttributes(Attributes):
    tackling: int
    interception: int
    positioning: int


@dataclass
class IntelligenceAttributes(Attributes):
    vision: int
    passing: int
    crossing: int
    ball_control: int
    dribbling: int
    skills: int
    team_work: int


@dataclass
class GkAttributes(Attributes):
    reflexes: int
    jumping: int
    positioning: int
    penalty: int

    def get_general_overall(self) -> int:
        return int((self.reflexes + self.jumping + self.positioning) / 3)


@dataclass
class PlayerAttributes:
    offensive: OffensiveAttributes
    physical: PhysicalAttributes
    defensive: DefensiveAttributes
    intelligence: IntelligenceAttributes
    gk: GkAttributes

    @classmethod
    def get_from_dict(cls, attributes: dict[str, dict[str, int]]):
        offensive = OffensiveAttributes.get_from_dict(attributes["offensive"])
        physical = PhysicalAttributes.get_from_dict(attributes["physical"])
        defensive = DefensiveAttributes.get_from_dict(attributes["defensive"])
        intelligence = IntelligenceAttributes.get_from_dict(attributes["intelligence"])
        gk = GkAttributes.get_from_dict(attributes["gk"])
        return cls(
            offensive,
            physical,
            defensive,
            intelligence,
            gk,
        )

    def serialize(self) -> dict[str, dict[str, int]]:
        return {
            "offensive": self.offensive.serialize(),
            "physical": self.physical.serialize(),
            "defensive": self.defensive.serialize(),
            "intelligence": self.intelligence.serialize(),
            "gk": self.gk.serialize(),
        }

    def get_overall(self, position: Positions) -> int:
        match position:
            case Positions.GK:
                return self.get_gk_overall()
            case Positions.DF:
                return self.get_df_overall()
            case Positions.MF:
                return self.get_mf_overall()
            case Positions.FW:
                return self.get_fw_overall()

        return 0

    def get_gk_overall(self) -> int:
        return int(
            (
                self.gk.get_overall() * 3
                + self.defensive.get_overall() * 2
                + self.physical.get_overall()
                + self.intelligence.get_overall()
            )
            / 7
        )

    def get_df_overall(self) -> int:
        return int(
            (
                self.defensive.get_overall() * 3
                + self.physical.get_overall() * 2
                + self.intelligence.get_overall()
                + self.offensive.get_overall()
            )
            / 7
        )

    def get_mf_overall(self) -> int:
        return int(
            (
                self.defensive.get_overall()
                + self.physical.get_overall() * 2
                + self.intelligence.get_overall() * 3
                + self.offensive.get_overall()
            )
            / 7
        )

    def get_fw_overall(self) -> int:
        return int(
            (
                self.defensive.get_overall()
                + self.physical.get_overall()
                + self.intelligence.get_overall() * 2
                + self.offensive.get_overall() * 3
            )
            / 7
        )
