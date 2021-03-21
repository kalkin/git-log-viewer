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

LOG = logging.getLogger('glv')


class NoPathMatches(Exception):
    ''' Thrown when there is no commit match a path filter.'''


class NoRevisionMatches(Exception):
    ''' Thrown when no revisions match the given arguments.'''


class Commit:
    ''' Wrapper object around a git.Commit object. '''
    def __init__(self,
                 repo,
                 commit: git.Commit,
                 parent: Optional['Commit'] = None,
                 level: int = 0,
                 branches: list[str] = None) -> None:
        self._commit: git.Commit = commit
        self.level: int = level
        self._parent: Optional['Commit'] = parent
        self._oid: str = commit.hexsha
        self._fork_point: Optional[bool] = None
        self._subject: Optional[Future] = None
        self._repo = repo
        self._branches: list[str] = branches or []

    @property
    def branches(self) -> List[str]:
        return self._branches

    def author_name(self) -> str:
        ''' Returns author name with mail as string. '''
        commit = self._commit
        return commit.author.name + " <" + commit.author.email + ">"

    def committer_name(self) -> str:
        ''' Returns author name with mail as string. '''
        commit = self._commit
        return commit.committer.name + " <" + commit.committer.email + ">"

    def is_fork_point(self) -> bool:
        # XXX Port to GitPython
        if self._fork_point is None:
            self._fork_point = bool(self._parent \
                    and isinstance(self._parent, Merge) \
                    and self._parent.raw_commit.parents[0] == self._commit \
                    and self._parent.is_rebased())
        return bool(self._fork_point)

    @property
    def author_date(self) -> str:
        return str(self._commit.authored_datetime)

    @property
    def author_unixdate(self) -> int:
        return self._commit.authored_date

    def committer_date(self) -> str:
        return str(self._commit.committed_datetime)

    def __stgit(self) -> bool:
        for name in self.branches:
            if name.startswith('patches/') \
                    and self.branches[name] == self.raw_commit:
                return True
        return False

    @property
    def icon(self) -> str:
        point = "●"
        if self.__stgit():
            point = "Ⓟ"

        return point

    @property  # type: ignore
    def arrows(self) -> str:
        if self.is_fork_point():
            return "─┘"

        return ''

    @property
    def raw_commit(self) -> git.Commit:
        # XXX Port to GitPython
        return self._commit

    @property
    def oid(self) -> str:
        return self._oid

    def _first_subject_line(self) -> str:
        try:
            return self._commit.message.strip().splitlines()[0]
        except IndexError:
            return ""

    def subject(self) -> str:
        ''' Returns the first line of the commit message. '''
        return self._first_subject_line()

    def modules(self) -> List[str]:
        warnings.warn("Use monorepo_modules instead", DeprecationWarning)
        return self.monorepo_modules()

    @functools.lru_cache()
    def monorepo_modules(self) -> List[str]:
        '''
            Return a list of monorepo modules touched by this commit.
            See vcs(1)
        '''
        # XXX Port to GitPython
        if not self._repo.has_modules:
            return []

        _id = str(self.oid)
        try:
            return self._repo.module_cache[_id]
        except KeyError:
            # pylint: disable=protected-access
            try:
                modules = list(
                    vcs.changed_modules(self._repo._nrepo, self._commit))
                self._repo.module_cache[_id] = modules
                return self._repo.module_cache[_id]
            except KeyError:
                pass

        return ''

    def short_id(self, max_len: int = 8) -> str:
        ''' Returns a shortend commit id. '''
        # XXX Port to GitPython
        return str(self._commit.hexsha)[0:max_len - 1]

    def __repr__(self) -> str:
        return str(self._commit.hexsha)

    def __str__(self) -> str:
        hash_id: str = self.short_id()
        rel_date: str = self.author_date
        author = self.author_name()
        return " ".join([hash_id, rel_date, author, self.subject()])

    @property
    def parent(self) -> Optional['Commit']:
        return self._parent

    def diff(self) -> str:
        other = None
        if self._commit.parents:
            other = self._commit.parents[0]
        git_cmd = git.cmd.Git()
        return git_cmd.diff('--stat', '-p', '-M', '--no-color', '--full-index',
                            other.hexsha, self.oid) + "\n"

    @property
    def is_top(self) -> bool:
        return self._parent is not None


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
        self.has_modules = vcs.modules(self._nrepo)

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

    def get(self, sth: Union[str, str]) -> Commit:
        try:
            git_commit = self._nrepo.commit(sth)
        except ValueError as exc:
            if not isinstance(sth, str):
                raise ValueError("Not found %s" % sth) from exc
            git_commit = self._nrepo.commit(sth)
        return to_commit(self, git_commit)

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
        return {b.name: b.commit.hexsha for b in self._nrepo.references}

    def count_commits(self, revision: str = 'HEAD') -> int:
        git_cmd = git.cmd.Git(self.working_dir)
        return int(
            git_cmd.rev_list(revision, first_parent=True, count=True).strip())

    def iter_commits(self,
                     revision: str = 'HEAD',
                     paths='',
                     **kwargs) -> Iterator[Commit]:
        parent = None
        for git_commit in self._nrepo.iter_commits(rev="%s" % revision,
                                                   paths=paths,
                                                   first_parent=True,
                                                   **kwargs):
            parent = to_commit(self, git_commit, parent)
            yield parent

    def walker(self,
               start_c: Commit,
               end_c: Optional[Commit] = None,
               parent: Optional[Commit] = None) -> Iterator[Commit]:
        start = start_c.oid
        end = None

        if end_c:
            end = end_c.oid or None

        if end:
            commit_log = self._nrepo.iter_commits(rev="%s..%s" % (end, start),
                                                  first_parent=True)
        else:
            commit_log = self._nrepo.iter_commits(rev="%s" % start,
                                                  first_parent=True)
        for git_commit in commit_log:
            parent = to_commit(self, git_commit, parent)
            yield parent  # type: ignore


def _commit_changed_files(commit: git.Commit, files: List[str]) -> bool:
    try:
        changed_files = vcs.changed_files(commit)  # pylint: disable=protected-access
        for _file in files:
            if _file in changed_files or [
                    x for x in changed_files if x.startswith(_file)
            ]:
                return True
    except KeyError:
        pass

    return False


def descendant_of(commit_a: git.Commit, commit_b: git.Commit) -> bool:
    ''' Implements a heuristic based on commit time '''
    try:
        while commit_a.commit_time >= commit_b.commit_time:  # type: ignore
            commit_a = commit_a.parents[0]
            if commit_a.id == commit_b.id:
                return True
    except:  # pylint: disable=bare-except
        return False
    return False


class Foldable(Commit):
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)
        self._folded = True
        self._rebased = None

    def child_log(self) -> Iterator[Commit]:
        start: Commit = to_commit(self._repo, self.raw_commit.parents[1], self)
        end: Optional[Commit] = self._repo.merge_base(
            self.raw_commit.parents[0], start.raw_commit)

        for commit in self._repo.walker(start, end, self):
            commit.level = self.level + 1
            try:
                if commit.raw_commit.parents and end \
                    and commit.raw_commit.parents[0] == end.raw_commit \
                    and self.raw_commit.parents[0] != end.raw_commit:
                    yield commit
                    yield CommitLink(self._repo, end.raw_commit, commit,
                                     commit.level)
                else:
                    yield commit
            except:  # pylint: disable=bare-except
                break

    def is_rebased(self):
        try:
            if self._rebased is None:
                # pylint: disable=protected-access
                self._rebased = len(self._commit.parents) >= 2 and \
                        self._repo._nrepo.is_ancestor(self._commit.parents[0],
                                self._commit.parents[1])
            return self._rebased
        except:  # pylint: disable=bare-except
            return False

    @property  # type: ignore
    def arrows(self) -> str:
        if self.subject().startswith('Update :'):
            if isinstance(self.parent, Foldable) and self.parent.is_rebased():
                return '⇤┤'

            return '⇤╮'

        if isinstance(self.parent, Foldable) \
            and self.parent.is_rebased():
            return "─┤"

        return "─┐"

    @property
    def is_folded(self) -> bool:
        return self._folded

    def unfold(self) -> Any:
        self._folded = False

    def fold(self) -> Any:
        self._folded = True


class InitialCommit(Commit):
    @property  # type: ignore
    def icon(self) -> str:
        return "◉"


class CommitLink(Commit):
    @property  # type: ignore
    def icon(self) -> str:
        return "⭞"

    @property  # type: ignore
    def arrows(self) -> str:
        return ""


class Merge(Foldable):
    pass


class Octopus(Foldable):
    # TODO Add support for ocotopus branch display
    pass


def _calculate_level(parent: Commit) -> int:
    level = 1
    if parent is not None:
        level = parent.level
        if isinstance(parent, Foldable):
            level += 1
    return level


# pylint: disable=too-many-return-statements
@functools.lru_cache(maxsize=512, typed=True)
def to_commit(repo: Repo,
              git_commit: git.Commit,
              parent: Optional[Commit] = None) -> Commit:
    level = 0
    branches = repo.branches_for_commit(git_commit)
    try:
        if not git_commit.parents:
            return InitialCommit(repo, git_commit, parent, level, branches)
    except Exception:  # pylint: disable=broad-except
        return Commit(repo, git_commit, parent, level, branches)

    parents_len = len(git_commit.parents)
    if parents_len == 1:
        return Commit(repo, git_commit, parent, level, branches)

    if parents_len == 2:
        return Merge(repo,
                     git_commit,
                     level=level,
                     parent=parent,
                     branches=branches)

    return Octopus(repo, git_commit, level=level, parent=parent)
