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
import logging
import os
import os.path
import subprocess  # nosec
from typing import Dict, List, Set

import git
import xdg

LOG = logging.getLogger('glv')

__all__ = [
    "changed_files",
    "CONFIG",
    "fetch_missing_data",
    "modules",
    "subtree_config_files",
]


def find_subtrees(items):
    ''' Filter trees for blobs with path ending in '.gitsubtrees'.  '''
    result = []
    for item in items:
        if isinstance(item, git.Tree):
            result += find_subtrees(item)
        elif isinstance(item, git.Blob) and item.path.endswith(".gitsubtrees"):
            result += [item]
    return result


@functools.lru_cache()
def subtree_config_files(working_dir: str) -> List[str]:
    ''' Return all the `.gitsubtree` files from a repository using git(1)‼ '''
    git_cmd = git.cmd.Git(working_dir=working_dir)
    return git_cmd.ls_files('--', '*.gitsubtrees').splitlines()


@functools.lru_cache()
def modules(working_dir: str) -> Dict[str, str]:
    ''' Return a dictionary of directories to module names '''

    files = subtree_config_files(working_dir)
    LOG.debug("Found subtree config files: %s", files)
    result: Dict[str, str] = {}
    for _file in files:
        conf = configparser.ConfigParser()
        conf.read(os.path.join(working_dir, _file))
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


def changed_files(commit: git.Commit, paths=None) -> Set[str]:
    ''' Return all files which were changed in the specified commit '''
    try:
        parent1 = commit.parents[0]
    except IndexError:
        return set()

    git_cmd = git.cmd.Git(working_dir=commit.repo.working_dir)
    return git_cmd.diff_tree(commit.tree,
                             parent1.tree,
                             "--",
                             *paths,
                             name_only=True,
                             no_renames=True,
                             no_color=True).splitlines()


def fetch_missing_data(commit: git.Commit, repo: git.Repo) -> bool:
    '''
        A workaround for fetching promisor data.

        When working in a repository which is partially cloned, then there will
        be commit objects, who are linking to locally non existing objects. By
        using git-show(1) we fetch all missing data.
    '''
    workdir = repo.working_dir
    gitdir = repo.git_dir
    oid = str(commit.oid)
    cmd = [
        'git', '--no-pager', '--git-dir', gitdir, '--work-tree', workdir,
        'show', oid
    ]
    LOG.info('Fetching missisng data for %s', oid)
    LOG.debug('Executing %s', ' '.join(cmd))
    try:
        subprocess.run(  # nosec
            cmd,
            capture_output=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=True)
    except subprocess.CalledProcessError:
        return False
    return True


def changed_modules(workdir: str, oid: str, bellow: str,
                    _modules: list[str]) -> list[str]:
    ''' Return all changed modules between two commits '''
    git_cmd = git.cmd.Git(workdir)
    revision = '%s..%s' % (bellow, oid)
    changed = git_cmd.diff(revision,
                           '--',
                           *_modules,
                           name_only=True,
                           no_renames=True,
                           no_color=True).splitlines()
    result = []
    for directory in sorted(_modules, reverse=True):
        for _file in changed:
            if _file.startswith(directory):
                result.append(directory)
                break
    return result


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
        'author_name_width': 10,
        'author_date_format': 'short',
    }
    conf.read(path)
    return conf


CONFIG = _config()
