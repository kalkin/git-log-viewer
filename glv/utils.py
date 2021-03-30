#
# Copyright (c) 2018-2020 Bahtiar `kalkin-` Gadimov.
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
''' A collection of useful functions '''
import os
import sys

import pykka
from prompt_toolkit import __version__ as ptk_version
from prompt_toolkit.data_structures import Size
from prompt_toolkit.output.base import Output

from glv import Commit, Repo, actors, vcs

from .cache import Cache

if ptk_version.startswith('3.'):
    PTK_VERSION = 3
    # pylint: disable=no-name-in-module,ungrouped-imports
    from prompt_toolkit.output.defaults import create_output
elif ptk_version.startswith('2.'):
    PTK_VERSION = 2
    # pylint: disable=no-name-in-module,ungrouped-imports
    from prompt_toolkit.output.defaults import get_default_output
else:
    print("Unsupported prompt_toolkit version " + ptk_version, file=sys.stderr)
    sys.exit(1)


def parse_args(**kwargs) -> Repo:
    ''' Parse cli arguments to get the `Repo` object '''
    if '--all' in kwargs['<REVISION>']:
        revision = '*'
    elif kwargs['<REVISION>'] and kwargs['<REVISION>'] != '--':
        revision = kwargs['<REVISION>']
    else:
        revision = ['HEAD']
    path = kwargs['--workdir'] or '.'
    path = os.path.abspath(os.path.expanduser(path))
    return (path, revision, kwargs['<path>'])


def screen_height() -> int:
    ''' Returns the current terminal height '''
    return _screen_size().rows


def screen_width() -> int:
    ''' Returns the current terminal width '''
    return _screen_size().columns


def _screen_size() -> Size:
    ''' Return the screen size '''
    if PTK_VERSION == 2:
        output: Output = get_default_output()
    else:
        output: Output = create_output()

    return output.from_pty(sys.stdout).get_size()


class ModuleChanges:
    ''' Helper class for querying two trees have module changes.
        The query runs async and the results are cached.
    '''  # pylint: disable=too-few-public-methods

    def __init__(self, working_dir: str) -> None:
        modules = vcs.modules(working_dir)
        self._actor = actors.ModuleActor.start(working_dir, modules)
        self._cache: dict[tuple(str, str), set[str]] = {}
        cache_dir = os.path.join(working_dir, '.git', 'glv', 'modules.json')
        self._file_cache: dict[tuple(str, str), set[str]] = Cache(cache_dir)

    def commit_modules(self, commit: Commit) -> list[str]:
        ''' Handy wrapper around `self.changed_modules` '''
        # pylint: disable=protected-access
        if not commit.bellow:
            return []

        key = commit.oid
        if key in self._file_cache:
            return self._file_cache[key]

        if key not in self._cache:
            message = (commit.oid, commit.bellow)
            self._cache[key] = self._actor.ask(message, block=False)

        try:
            tmp = self._cache[key].get(0)
            self._file_cache[key] = tmp
            return tmp
        except pykka.Timeout:
            return []


_MOD_CHANGES_INSTANCES: dict[str, ModuleChanges] = {}


def mod_changes(working_dir: str) -> ModuleChanges:
    ''' Return the `ModuleChanges` instance for given working_dir '''
    if working_dir not in _MOD_CHANGES_INSTANCES:
        _MOD_CHANGES_INSTANCES[working_dir] = ModuleChanges(working_dir)
    return _MOD_CHANGES_INSTANCES[working_dir]
