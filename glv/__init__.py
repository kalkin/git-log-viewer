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
import pkg_resources
from pygit2 import GIT_DIFF_REVERSE  # pylint: disable=no-name-in-module
from pygit2 import Diff  # pylint: disable=no-name-in-module
from pygit2 import Mailmap  # pylint: disable=no-name-in-module
from pygit2 import Oid  # pylint: disable=no-name-in-module
from pygit2 import Tree  # pylint: disable=no-name-in-module
from pygit2 import discover_repository  # pylint: disable=no-name-in-module
from pygit2 import Commit as GitCommit  # pylint: disable=no-name-in-module
from pygit2 import Repository as GitRepo  # pylint: disable=no-name-in-module
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
    ''' Wrapper object around a pygit2.Commit object. '''
    def __init__(self,
                 repo,
                 commit: GitCommit,
                 parent: Optional['Commit'] = None,
                 level: int = 0) -> None:
        self._repo: Repo = repo
        self._commit: GitCommit = commit
        self.level: int = level
        self._parent: Optional['Commit'] = parent
        self._oid: Oid = commit.id
        self._fork_point: Optional[bool] = None
        self._subject: Optional[Future] = None

    @property
    def branches(self) -> List[str]:
        branches = self._repo.branches()
        return [
            name for name, commit in branches.items()
            if commit == self.raw_commit
        ]

    @functools.lru_cache()
    def author_name(self) -> str:
        ''' Returns author name with mail as string. '''
        commit = self._commit
        return commit.author.name + " <" + commit.author.email + ">"

    @functools.lru_cache()
    def is_fork_point(self) -> bool:
        if self._fork_point is None:
            self._fork_point = bool(self._parent \
                    and isinstance(self._parent, Merge) \
                    and self._parent.raw_commit.parents[0] == self._commit \
                    and self._parent.is_rebased())
        return bool(self._fork_point)

    @functools.lru_cache()
    def author_date(self) -> str:
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        timestamp: int = self._commit.author.time
        delta = datetime.now() - datetime.fromtimestamp(timestamp)
        _format = vcs.CONFIG['history']['author_date_format']
        try:
            return babel.dates.format_timedelta(delta, format=_format)
        except KeyError as e:
            if delta.total_seconds() < 60:
                return f'{round(delta.total_seconds())} s'
            raise e

    @property  # type: ignore
    @functools.lru_cache()
    def next(self) -> Optional['Commit']:
        raw_commit: GitCommit = self.raw_commit
        try:
            if not raw_commit.parents:
                return None
        except:  # pylint: disable=bare-except
            return None

        next_raw_commit: GitCommit = raw_commit.parents[0]
        return to_commit(self._repo, next_raw_commit, self)

    @functools.lru_cache()
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
    def raw_commit(self) -> GitCommit:
        return self._commit

    @property
    def oid(self) -> Oid:
        return self._oid

    @functools.lru_cache()
    def _first_subject_line(self) -> str:
        try:
            return self._commit.message.strip().splitlines()[0]
        except IndexError:
            return ""

    def subject(self) -> str:
        ''' Returns the first line of the commit message. '''
        if not self._repo.provider:
            return self._first_subject_line()

        if self._subject is None:
            subject = self._first_subject_line()
            self._subject = self._repo.provider.ask(subject, block=False)

        try:
            return self._subject.get(0)
        except Timeout:
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
        if not self._repo.has_modules:
            return []
        _id = str(self.oid)
        try:
            return self._repo.module_cache[_id]
        except KeyError:
            # pylint: disable=protected-access
            try:
                modules = list(
                    vcs.changed_modules(self._repo._repo, self._commit))
                self._repo.module_cache[_id] = modules
                return self._repo.module_cache[_id]
            except KeyError:
                pass
        return ''

    def short_id(self, max_len: int = 8) -> str:
        ''' Returns a shortend commit id. '''
        return str(self._commit.id)[0:max_len - 1]

    def short_author_name(self) -> str:
        width = vcs.CONFIG['history'].getint('author_name_width')
        signature = self._repo.mailmap.resolve_signature(self._commit.author)
        tmp = textwrap.shorten(signature.name, width=width, placeholder="…")
        if tmp == '…':
            return signature.name[0:width - 1] + '…'
        return tmp

    @property
    def author_signature(self) -> (str, str):
        return self._repo.mailmap.resolve_signature(self._commit.author)

    @property
    def committer_signature(self) -> (str, str):
        return self._repo.mailmap.resolve_signature(self._commit.committer)

    def __repr__(self) -> str:
        return str(self._commit.id)

    def __str__(self) -> str:
        hash_id: str = self.short_id()
        rel_date: str = self.author_date()
        author = self.short_author_name()
        return " ".join([hash_id, rel_date, author, self.subject()])

    @property
    def parent(self) -> Optional['Commit']:
        return self._parent

    def diff(self) -> Optional[Diff]:
        if self._commit.parents:
            a = self._commit  # pylint: disable=invalid-name
            b = self._commit.parents[0]  # pylint: disable=invalid-name
            # pylint: disable=protected-access
            return self._repo._repo.diff(a, b, None, GIT_DIFF_REVERSE)
        return self._commit.tree.diff_to_tree(flags=GIT_DIFF_REVERSE)

    @property
    def is_top(self) -> bool:
        return self._parent is not None


def providers():
    named_objects = {}
    for entry_point in pkg_resources.iter_entry_points(group='glv_providers'):
        named_objects.update({entry_point.name: entry_point.load()})
    return named_objects


class Repo:
    ''' A wrapper around `pygit2.Repository`. '''

    # pylint: disable=too-many-instance-attributes
    def __init__(self,
                 path: str,
                 revision: List[str] = None,
                 files: List[str] = None) -> None:
        self.provider: Optional[ProviderActor] = None
        self.files = files or []
        repo_path = discover_repository(path)
        if not repo_path:
            print(' Not a git repository', file=sys.stderr)
            sys.exit(2)
        self._repo = GitRepo(repo_path)
        self.mailmap = Mailmap.from_repository(self._repo)
        cache_path = self._repo.path + __name__ + '/modules.json'
        self.module_cache = Cache(cache_path)
        self.has_modules = False
        if vcs.modules(self._repo):
            self.has_modules = True
        # {str:pygit2.Object }
        self._branches = {
            r.shorthand: r.peel()
            for r in self._repo.references.objects
            if not r.shorthand.endswith('/HEAD')
        }
        parsed_results = parse_revisions(revision)
        if len(parsed_results) == 0:
            raise NoRevisionMatches
        if len(parsed_results) > 1:
            raise NotImplementedError('Multi branch support NYI')

        first_revision_result = parsed_results[0]
        self.revision = first_revision_result.input
        try:
            self.__start: GitCommit = self._repo.revparse_single(
                first_revision_result.start)

            if first_revision_result.end:
                self.__end: GitCommit = self._repo.revparse_single(
                    first_revision_result.end)
            else:
                self.__end = None
        except KeyError as exc:
            raise NoRevisionMatches from exc

        for provider in providers().values():
            if provider.enabled(self._repo):
                cache_dir = self._repo.path + __name__ + '/remotes/origin'
                self.provider = ProviderActor.start(
                    provider(self._repo, cache_dir))
                break

    def get(self, sth: Union[str, Oid]) -> Commit:
        try:
            git_commit = self._repo[sth]
        except ValueError:
            if not isinstance(sth, str):
                raise ValueError("Not found %s" % sth)
            git_commit = self._repo.revparse_single(sth)
        return to_commit(self, git_commit)

    def merge_base(self, oid1: GitCommit, oid2: GitCommit) -> Optional[Commit]:
        try:
            oid: Oid = self._repo.merge_base(oid1.id, oid2.id)
        except Exception:  # pylint: disable=broad-except
            return None
        if not oid:
            return None
        result = self._repo[oid]
        return to_commit(self, result)

    @functools.lru_cache()
    def branches(self) -> Dict[str, Any]:
        return self._branches

    def walker(self,
               start_c: Optional[Commit] = None,
               end_c: Optional[Commit] = None,
               parent: Optional[Commit] = None) -> Iterator[Commit]:
        if not start_c:
            start = self.__start.id
        else:
            start = start_c.oid

        end = None
        if end_c:
            end = end_c.oid or None
        elif self.__end:
            end = self.__end.oid

        git_commit = self._repo[start]
        try:
            while self.files and not _commit_changed_files(
                    git_commit, self.files):
                git_commit = git_commit.parents[0]
        except KeyError:
            raise NoPathMatches()

        parent = to_commit(self, git_commit, parent)
        yield parent  # type: ignore
        while True:
            try:
                if not git_commit.parents:
                    break
            except KeyError:
                break
            git_commit = git_commit.parents[0]
            if git_commit.id == end:
                break
            tmp = to_commit(self, git_commit, parent)
            if not self.files or _commit_changed_files(git_commit, self.files):
                yield tmp
            parent = tmp

    def __str__(self) -> str:
        path = self._repo.workdir.replace(os.path.expanduser('~'), '~', 1)
        revision = self.revision
        if self.revision == 'HEAD':
            revision = self._repo.head.shorthand
        return '%s \uf418 %s' % (path.rstrip('/'), revision)


def _commit_changed_files(commit: GitCommit, files: List[str]) -> bool:
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


def descendant_of(commit_a: GitCommit, commit_b: GitCommit) -> bool:
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
    def __init__(self,
                 repo: Repo,
                 commit: GitCommit,
                 parent: Optional[Commit] = None,
                 level: int = 1) -> None:
        super().__init__(repo, commit, parent, level)
        self._folded = True
        self._repo = repo
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
                self._rebased = len(self._commit.parents) >= 2 and \
                        descendant_of(self._commit.parents[1], self._commit.parents[0])
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
              git_commit: GitCommit,
              parent: Optional[Commit] = None) -> Commit:
    level = 0
    try:
        if not git_commit.parents:
            return InitialCommit(repo, git_commit, parent, level)
    except Exception:  # pylint: disable=broad-except
        return Commit(repo, git_commit, parent, level)

    parents_len = len(git_commit.parents)
    if parents_len == 1:
        return Commit(repo, git_commit, parent, level)

    if parents_len == 2:
        return Merge(repo, git_commit, level=level, parent=parent)

    return Octopus(repo, git_commit, level=level, parent=parent)
