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

from prompt_toolkit import __version__ as ptk_version
from prompt_toolkit.data_structures import Size
from prompt_toolkit.output.base import Output

from glv import Repo

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
