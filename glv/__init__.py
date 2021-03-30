# pylint: disable=missing-docstring,fixme
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
import functools
import itertools
import logging
import os
import re
import sys
import textwrap
import time
import warnings
from datetime import datetime
from typing import Any, Dict, Iterator, List, Optional, Tuple, Union

import babel.dates
import git
import pkg_resources
from pykka import Future, Timeout

import glv.vcs as vcs
from glv.actors import ProviderActor
from glv.cli import parse_revisions
from glv.providers import Cache, Provider

from .commit import Commit, commits_for_range

LOG = logging.getLogger('glv')


class NoPathMatches(Exception):
    ''' Thrown when there is no commit match a path filter.'''


class NoRevisionMatches(Exception):
    ''' Thrown when no revisions match the given arguments.'''


def providers():
    named_objects = {}
    for entry_point in pkg_resources.iter_entry_points(group='glv_providers'):
        named_objects.update({entry_point.name: entry_point.load()})
    return named_objects


class Repo:
    ''' A wrapper around `git.Repo`. '''

    # pylint: disable=too-many-instance-attributes
    def __init__(self, path: Optional[str] = None) -> None:
        self.provider: Optional[ProviderActor] = None
        self._nrepo = git.Repo(path=path,
                               odbt=git.GitCmdObjectDB,
                               search_parent_directories=True)
        cache_path = os.path.join(self._nrepo.git_dir, __name__,
                                  'modules.json')
        self.module_cache = Cache(cache_path)
        self.has_modules = vcs.modules(self.working_dir)

        # for provider in providers().values():
        # if provider.enabled(self._nrepo):
        # cache_dir = os.path.join(self._nrepo.git_dir, __name__,
        # 'remotes', 'origin')
        # self.provider = ProviderActor.start(
        # provider(self._nrepo, cache_dir))
        # break

    def branches_for_commit(self, commit: git.Commit) -> list[str]:
        needle: str = commit.hexsha
        return [name for name, oid in self.branches().items() if oid == needle]

    @property
    def working_dir(self) -> str:
        return self._nrepo.working_dir

    def merge_base(self, oid1: git.Commit,
                   oid2: git.Commit) -> Optional[Commit]:
        try:
            oid: str = self._nrepo.merge_base(oid1, oid2)
            if not oid:
                return None
            result = self._nrepo.commit(oid[0])
            return to_commit(self, result)
        except git.BadName:
            return None

    @functools.lru_cache()
    def branches(self) -> Dict[str, str]:
        git_cmd = git.cmd.Git(self.working_dir)
        result = {}
        for line in git_cmd.show_ref().splitlines():
            oid, _, ref = line.partition(' ')
            try:
                result[ref.split("/", 2)[2]] = oid
            except IndexError:
                pass
        return result

    def count_commits(self, revision: str = 'HEAD') -> int:
        git_cmd = git.cmd.Git(self.working_dir)
        return int(
            git_cmd.rev_list(revision, first_parent=True, count=True).strip())

    def iter_commits(
        self,
        rev_range: str,
        skip: int,
        max_count: int,
        paths='',
    ) -> List[Commit]:
        return commits_for_range(self.working_dir,
                                 rev_range=rev_range,
                                 level=0,
                                 paths=paths,
                                 rev_list_args={
                                     "skip": skip,
                                     "max_count": max_count
                                 })
