#      Openfoot Manager - A free and open source soccer management game
#      Copyright (C) 2020-2022  Pedrenrique G. Guimarães
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
import json
import logging
from ofm import ROOT_DIR, RES_DIR


def write_to_file(
    contents: list,
    filename: str,
    folder: str = ROOT_DIR,
    res_folder: str = RES_DIR,
) -> None:
    try:
        filename = find_file(filename, folder)
    except FileNotFoundError:
        filename = os.path.join(res_folder, filename)
    finally:
        with open(filename, "w") as fp:
            json.dump(contents, fp, sort_keys=True, indent=4)


def find_file(filename: str, folder: str = ROOT_DIR) -> str:
    for root, _, files in os.walk(folder):
        if filename in files:
            return os.path.join(root, filename)
    else:
        raise FileNotFoundError("File not found!")


def get_list_from_file(filename: str) -> list:
    filename = find_file(filename)

    with open(filename, "r", encoding="utf-8") as fp:
        lst = fp.read().splitlines()
    
    return lst


def get_from_file(filename: str) -> list:
    with open(filename, "r", encoding="utf-8") as fp:
        content = json.load(fp)
    
    return content


def load_list_from_file(filepath: str, folder: str = ROOT_DIR) -> list:
    try:
        filename = find_file(filepath, folder)
    except FileNotFoundError as e:
        print("File was not found!")
        print("Error occured: {}".format(e.errno))
    else:
        return get_from_file(filename)
