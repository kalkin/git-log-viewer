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


def repo_from_args(**kwargs) -> Repo:
    ''' Parse cli arguments to get the `Repo` object '''
    if kwargs['REVISION'] and kwargs['REVISION'] != '--':
        revision = kwargs['REVISION']
    else:
        revision = 'HEAD'
    path = kwargs['--workdir'] or '.'
    path = os.path.abspath(os.path.expanduser(path))
    return Repo(path, revision, kwargs['<path>'])


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
