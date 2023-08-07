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
from enum import Enum


class PlayerInjury(str, Enum):
    NO_INJURY = "No injury"
    SHOULDER_DS = "Dislocated shoulder"
    ANKLE_SP = "Ankle sprain"
    KNEE_SP = "Knee sprain"
    CALF_ST = "Calf strain"
    KNEECAP_BURSITIS = "Kneecap bursitis"
    RIB_BROK = "Broken rib"
    HIP_BROK = "Broken hip"
    CLAVICLE_FRAC = "Fractured clavicle"
    ARM_FRAC = "Fractured arm"
    FOOT_FRAC = "Fractured foot"
    WRIST_FRAC = "Fractured wrist"
    ANKLE_FRAC = "Fractured ankle"
    CONCUSSION = "Concussion"
    LIGAMENT_TORN = "Torn ligament"
    MENISCAL_TORN = "Torn meniscal"
