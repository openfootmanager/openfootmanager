#      Openfoot Manager - A free and open source soccer management game
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
import os
import yaml
from typing import Union
from pathlib import Path

from ofm.defaults import PROJECT_DIR


class Settings:
    def __init__(
            self,
            root_dir: Union[str, Path] = PROJECT_DIR,
            settings: Union[str, Path] = os.path.join(PROJECT_DIR, "settings.yaml")
    ):
        self.root_dir = root_dir
        self.res: str = os.path.join(root_dir, "res")
        self.images: str = os.path.join(root_dir, "images")
        self.db: str = os.path.join(self.res, "db")
        self.save: str = os.path.join(root_dir, "save")
        self.players_file: str = os.path.join(self.db, "players.json")
        self.teams_file: str = os.path.join(self.db, "teams.json")
        self.settings_file: str = settings

    def get_data(self) -> dict:
        return {
            "res": self.res,
            "images": self.images,
            "db": self.db,
            "save": self.save,
            "players": self.players_file,
            "teams": self.teams_file,
        }
    
    def parse_settings(self, data: dict) -> None:
        self.res = data['res']
        self.images = data['images']
        self.db = data['db']
        self.save = data['save']

    def load_settings(self) -> None:
        with open(self.settings_file, 'r') as fp:
            data = yaml.safe_load(fp)
            self.parse_settings(data)

    def create_settings(self) -> None:
        with open(self.settings_file, 'w') as fp:
            yaml.safe_dump(self.get_data(), fp)

    def get_settings(self) -> None:
        if os.path.exists(self.settings_file):
            self.load_settings()
        else:
            self.create_settings()

