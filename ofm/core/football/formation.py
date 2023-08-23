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
from dataclasses import dataclass, field
from typing import Optional

from .player import PlayerSimulation, PlayerTeam, Positions

FORMATION_STRINGS = [
    "3-4-3",
    "3-5-2",
    "3-6-1",
    "4-4-2",
    "4-3-3",
    "4-5-1",
    "5-4-1",
    "5-3-2",
]


class FormationError(Exception):
    pass


@dataclass
class Formation:
    formation_string: str
    gk: Optional[PlayerSimulation] = None
    df: list[PlayerSimulation] = field(default_factory=list)
    mf: list[PlayerSimulation] = field(default_factory=list)
    fw: list[PlayerSimulation] = field(default_factory=list)
    bench: list[PlayerSimulation] = field(default_factory=list)
    _players: list[PlayerSimulation] = field(default_factory=list)
    _all_players: list[PlayerSimulation] = field(default_factory=list)

    def __post_init__(self):
        if not self.validate_formation():
            raise FormationError("Invalid formation string!")

    @property
    def players(self):
        if self.gk is None:
            self._players = []
            return self._players

        self._players = [self.gk]
        self._players.extend(self.df)
        self._players.extend(self.mf)
        self._players.extend(self.fw)
        return self._players

    @property
    def all_players(self):
        self._all_players = self.players
        self._all_players.extend(self.bench)
        return self._all_players

    def get_num_players(self) -> tuple[int, int, int]:
        defenders, midfielders, forwards = self.formation_string.split("-")
        return int(defenders), int(midfielders), int(forwards)

    def get_best_players_per_position(
        self, players: list[PlayerTeam], position: Positions
    ) -> list[PlayerTeam]:
        if players_in_position := [
            player
            for player in players
            if player.details.get_best_position() == position
        ]:
            players_in_position.sort(
                key=lambda x: x.details.attributes.get_overall(position), reverse=True
            )
            return players_in_position

        raise FormationError("Invalid position.")

    def get_best_players(self, players: list[PlayerTeam]):
        df, mf, fw = self.get_num_players()
        for position in range(11):
            if position == 0:
                pos = Positions.GK
            elif 0 < position <= df and len(self.df) < df:
                pos = Positions.DF
            elif df < position <= df + mf and len(self.mf) < mf:
                pos = Positions.MF
            elif position <= df + mf + fw and len(self.fw) < fw:
                pos = Positions.FW
            else:
                raise FormationError("Unable to get best players!")

            player = self.get_best_players_per_position(players.copy(), pos)[0]

            if player:
                self.add_player(position, player)
                players.remove(player)

        self.bench = [
            PlayerSimulation(
                player, player.details.get_best_position(), player.details.stamina
            )
            for player in players
        ]
        self.bench.sort(key=lambda x: x.current_position)

    def add_player(self, position: int, player: PlayerTeam):
        player_sim = PlayerSimulation(player, Positions.GK, player.details.stamina)
        df, mf, fw = self.get_num_players()
        if position == 0:
            self.gk = player_sim
        elif 0 < position <= df and len(self.df) < df:
            player_sim.current_position = Positions.DF
            self.df.append(player_sim)
        elif df < position <= df + mf and len(self.mf) < mf:
            player_sim.current_position = Positions.MF
            self.mf.append(player_sim)
        elif df + mf < position <= df + mf + fw and len(self.fw) < fw:
            player_sim.current_position = Positions.FW
            self.fw.append(player_sim)
        else:
            player_sim.current_position = player_sim.player.details.get_best_position()
            self.bench.append(player_sim)

    def substitute_player(
        self, player_out: PlayerSimulation, player_in: PlayerSimulation
    ):
        current_position = player_out.current_position
        player_out.subbed = True

        player_in.current_position = current_position
        if current_position == Positions.GK:
            self.gk = player_in
            index = self.bench.index(player_in)
            self.bench[index] = player_out
        elif current_position == Positions.DF:
            index = self.df.index(player_out)
            index_out = self.bench.index(player_in)
            self.df[index] = player_in
            self.bench[index_out] = player_out
        elif current_position == Positions.MF:
            index = self.mf.index(player_out)
            index_out = self.bench.index(player_in)
            self.mf[index] = player_in
            self.bench[index_out] = player_out
        elif current_position == Positions.FW:
            index = self.fw.index(player_out)
            index_out = self.bench.index(player_in)
            self.fw[index] = player_in
            self.bench[index_out] = player_out

    def validate_formation(self) -> bool:
        return self.formation_string in FORMATION_STRINGS
