''' A collection of useful functions '''
import os

from pygit_viewer import Repo


def repo_from_args(**kwargs) -> Repo:
    ''' Parse cli arguments to get the `Repo` object '''
    if kwargs['REVISION'] and kwargs['REVISION'] != '--':
        revision = kwargs['REVISION']
    else:
        revision = 'HEAD'
    path = kwargs['--workdir'] or '.'
    path = os.path.abspath(os.path.expanduser(path))
    return Repo(path, revision, kwargs['<path>'])
