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
''' Helper functions for doing things vcs(1) does '''
import configparser
import functools
import glob
import logging
import os
import os.path
import subprocess
import sys
from typing import Dict, List, Set

import xdg
from pygit2 import Commit  # pylint: disable=no-name-in-module
from pygit2 import Repository  # pylint: disable=no-name-in-module

LOG = logging.getLogger('glv')

__all__ = [
    "changed_files",
    "changed_modules",
    "CONFIG",
    "fetch_missing_data",
    "modules",
    "subtree_config_files",
]


@functools.lru_cache()
def subtree_config_files(repo: Repository) -> List[str]:
    ''' Return all the `.gitsubtree` files from a repository using git(1)‼ '''
    os.chdir(repo.workdir)
    files = glob.glob('**/.gitsubtrees', recursive=True)
    if os.path.exists('.gitsubtrees'):
        files += ['.gitsubtrees']
    return files


@functools.lru_cache()
def modules(repo: Repository) -> Dict[str, str]:
    ''' Return list of all .gitsubtrees modules in repository '''

    files = subtree_config_files(repo)
    LOG.debug("Found subtree config files: %s", files)
    result: Dict[str, str] = {}
    for _file in files:
        conf = configparser.ConfigParser()
        conf.read(os.path.join(repo.workdir, _file))
        path = ''
        if '/' in _file:
            parts = _file.split('/')[:-1]
            path = '/'.join(parts)
        for key in conf.sections():
            _path = os.path.join(path, key)
            name = _path
            result[_path] = name
            if conf[key].get('previous'):
                previous = [
                    x.strip() for x in conf[key].get('previous').split(',')
                ]
                for sth in previous:
                    if sth.startswith('/'):
                        _path = sth.lstrip('/')
                    else:
                        _path = os.path.join(path, sth)
                    result[_path] = name
    LOG.debug("Found subprojects in : %s", result.keys())
    return result


def changed_files(commit: Commit) -> Set[str]:
    ''' Return all files which were changed in the specified commit '''
    try:
        parent1 = commit.parents[0]
    except IndexError:
        return set()

    deltas = commit.tree.diff_to_tree(parent1.tree).deltas
    result: List[str] = []
    for delta in deltas:
        try:
            result += [delta.old_file.path, delta.new_file.path]
        except UnicodeDecodeError:
            pass

    return set(result)


def changed_modules(repo: Repository, commit: Commit) -> Set[str]:
    ''' Return all .gisubtrees modules which were changed in the specified commit '''
    _modules = modules(repo)
    changed = changed_files(commit)
    files = {name: True for name in changed}
    result: List[str] = []
    for directory in sorted(_modules, reverse=True):
        matches = [_file for _file in files if _file.startswith(directory)]
        if matches:
            result.append(_modules[directory])
            files = {k: True for k in files if k not in matches}

    return set(result)


def fetch_missing_data(commit: Commit, repo: Repository) -> bool:
    '''
        A workaround for fetching promisor data.

        When working in a repository which is partially cloned, then there will
        be commit objects, who are linking to localy non existing objects. By
        using git-show(1) we fetch all missing data.
    '''
    workdir = repo.workdir
    gitdir = repo.path
    oid = str(commit.oid)
    cmd = [
        'git', '--no-pager', '--git-dir', gitdir, '--work-tree', workdir,
        'show', oid
    ]
    LOG.info('Fetching missisng data for %s', oid)
    LOG.debug('Executing %s', ' '.join(cmd))
    try:
        subprocess.run(cmd,
                       capture_output=False,
                       stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL,
                       check=True)
    except subprocess.CalledProcessError:
        return False
    return True


def _config() -> configparser.ConfigParser:
    path = xdg.XDG_CONFIG_HOME.joinpath('glv', 'config')
    conf = configparser.ConfigParser()
    conf['history'] = {
        'author_date_color': 'ansiblue',
        'author_name_color': 'ansigreen',
        'branches_color': 'ansiyellow',
        'icon_color': 'bold',
        'modules_color': 'ansiyellow',
        'short_id_color': 'ansimagenta',
        'subject_color': '',
        'type_color': 'bold',
        'icon_set': 'ascii',
    }
    conf.read(path)
    return conf


CONFIG = _config()
