# pylint: disable=missing-docstring,fixme
#
# Copyright (c) 2021 Bahtiar `kalkin-` Gadimov.
#
# This file is part of Git Log Viewer
# (see https://github.com/kalkin/git-log-viewer).
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.
#
import json
import logging
import os
import pathlib
from typing import Any

LOG = logging.getLogger('glv')


class Cache:
    def __init__(self, file_path: str) -> None:
        self._storage: dict = {}
        cache_dir = os.path.dirname(file_path)
        self._cache_file = file_path
        self._ro_backend = False

        try:
            pathlib.Path(cache_dir).mkdir(parents=True, exist_ok=True)
            if os.path.isfile(file_path):  # restore cache
                with open(file_path, encoding='utf-8') as data_file:
                    self._storage = json.loads(data_file.read())
        except PermissionError:
            LOG.warning('Read only git-dir, no data will be cached')
            self._ro_backend = True
        except json.decoder.JSONDecodeError as exc:
            LOG.warning('Failed to parse %s: %s', data_file.name, exc.msg)

    def __contains__(self, key) -> bool:
        return key in self._storage

    def __getitem__(self, key) -> Any:
        return self._storage[key]

    def __setitem__(self, key, value):
        self._storage[key] = value
        if not self._ro_backend:
            with open(self._cache_file, 'w') as outfile:
                json.dump(self._storage, outfile)
