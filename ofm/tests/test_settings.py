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
import pytest
import os

from ofm.core.settings import Settings


@pytest.fixture
def settings(tmp_path):
    f = tmp_path / 'settings.yaml'
    return Settings(tmp_path, f)


def test_get_settings(settings, tmp_path):
    expected_data = {
        "res": os.path.join(tmp_path, "res"),
        "images": os.path.join(tmp_path, "images"),
        "db": os.path.join(tmp_path, "res", "db"),
        "save": os.path.join(tmp_path, "save"),
        "players": os.path.join(tmp_path, "res", "db", "players.json"),
        "teams": os.path.join(tmp_path, "res", "db", "teams.json")
    }
    settings.create_settings()
    settings.load_settings()
    assert settings.get_data() == expected_data
    