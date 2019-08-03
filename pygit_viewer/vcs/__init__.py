''' Helper functions for doing things vcs(1) does '''

import configparser
import os.path
import subprocess
import sys
from typing import List, Set

from pygit2 import Commit  # pylint: disable=no-name-in-module
from pygit2 import Repository  # pylint: disable=no-name-in-module


def modules(repo: Repository) -> List[str]:
    ''' Return list of all .gitsubtrees modules in repository '''

    def subtree_config_files() -> List[str]:
        result = subprocess.run(
            ['git', 'ls-files', '*/.gitsubtrees', '.gitsubtrees'],
            stdout=subprocess.PIPE,
            cwd=repo.workdir)
        if result.returncode != 0:
            raise Exception("No gitsubtree files")

        return result.stdout.decode('utf-8').splitlines()

    files = subtree_config_files()
    result: List[str] = []
    for _file in files:
        conf = configparser.ConfigParser()
        conf.read(os.path.join(repo.workdir, _file))
        path = ''
        if '/' in _file:
            parts = _file.split('/')[:-1]
            path = '/'.join(parts) + '/'
        result += ["%s%s" % (path, key) for key in conf.sections()]
    result.sort()
    return result


def changed_files(commit: Commit) -> Set[str]:
    ''' Return all files which were changed in the specified commit '''
    parent1 = commit.parents[0]
    deltas = commit.tree.diff_to_tree(parent1.tree).deltas
    result: List[str] = []
    for delta in deltas:
        result += [delta.old_file.path, delta.new_file.path]
    return set(result)


def changed_modules(repo: Repository, commit: Commit) -> Set[str]:
    ''' Return all .gisubtrees modules which were changed in the specified commit '''
    dirs = modules(repo)
    dirs.sort(reverse=True)
    files = {name: True for name in changed_files(commit)}
    result: List[str] = []
    for directory in dirs:
        matches = [_file for _file in files if _file.startswith(directory)]
        if matches:
            result.append(directory)
            for _file in matches:
                del files[_file]
    return set(result)
