# pylint: disable=missing-docstring,fixme
import random
import sys
import time
from datetime import datetime
from typing import Any, Iterator, Optional, Union

import babel.dates
import pkg_resources
from pygit2 import Commit as GitCommit  # pylint: disable=no-name-in-module
from pygit2 import Oid  # pylint: disable=no-name-in-module
from pygit2 import discover_repository  # pylint: disable=no-name-in-module
from pygit2 import Repository as GitRepo  # pylint: disable=no-name-in-module

from pygit_viewer.providers import Atlassian


class Commit:
    ''' Wrapper object around a pygit2.Commit object. '''

    def __init__(self,
                 repo,
                 commit: GitCommit,
                 parent: Optional['Commit'] = None,
                 level: int = 0) -> None:
        self._repo = repo
        self._commit: GitCommit = commit
        self.level: int = level
        self._parent: Optional['Commit'] = parent
        self._oid: Oid = commit.id
        self.noffff: bool = False
        self._fork_point: Optional[bool] = None

    def author_name(self) -> str:
        ''' Returns author name with mail as string. '''
        commit = self._commit
        return commit.author.name + " <" + commit.author.email + ">"

    def is_fork_point(self) -> bool:
        if self._fork_point is None:
            self._fork_point = bool(self._parent \
                    and isinstance(self._parent, Merge) \
                    and self._parent.raw_commit.parents[0] == self._commit \
                    and self._parent.is_rebased())
        return bool(self._fork_point)

    def author_date(self) -> str:
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        timestamp: int = self._commit.author.time
        delta = datetime.now() - datetime.fromtimestamp(timestamp)
        return babel.dates.format_timedelta(delta, format='short').strip('.')

    @property
    def next(self) -> 'Commit':
        if self._commit.parents:
            commit = self._commit.parents[0]
            return Commit(self._repo, commit, self, level=self.level)
        raise IndexError

    @property
    def icon(self) -> str:
        if self.noffff:
            return "……"

        if self.is_fork_point():
            return "●─╯"

        return "○"

    def render(self):
        level = self.level * '│ '
        _type = level + self.icon.ljust(4, " ")
        return [("ansimagenta", self.short_id() + " "),
                ("ansiblue", self.author_date()),
                ("ansigreen", self.short_author_name()), ("bold", _type),
                (" ", self.subject())]

    @property
    def raw_commit(self) -> GitCommit:
        return self._commit

    @property
    def oid(self) -> Oid:
        return self._oid

    def subject(self) -> str:
        ''' Returns the first line of the commit message. '''
        try:
            subject = self._commit.message.strip().splitlines()[0]
            if subject.startswith("Merge pull request #"):
                if self._repo.provider \
                        and self._repo.provider.has_match(subject):
                    return self._repo.provider.provide(subject)
                words = subject.split()
                subject = ' '.join(words[3:])
                subject = 'MERGE: ' + subject
            return subject
        except IndexError:
            return ""

    def short_id(self, max_len: int = 8) -> str:
        ''' Returns a shortend commit id. '''
        return str(self._commit.id)[0:max_len - 1]

    def short_author_name(self) -> str:
        return self.author_name().split()[0]

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

    @property
    def is_top(self) -> bool:
        return self._parent is not None


def providers():
    named_objects = {}
    for entry_point in pkg_resources.iter_entry_points(
            group='pygit_viewer_plugins'):
        named_objects.update({entry_point.name: entry_point.load()})
    return named_objects


class Repo:
    ''' A wrapper around `pygit2.Repository`. '''

    def __init__(self, path: str) -> None:
        self.provider = None
        self._repo = GitRepo(discover_repository(path))
        for provider in providers().values():
            if provider.enabled(self._repo):
                cache_dir = self._repo.path + '/pygit-viewer/remotes/origin'
                self.provider = provider(self._repo, cache_dir)
                break

    def get(self, sth: Union[str, Oid]) -> Commit:
        try:
            git_commit = self._repo[sth]
        except ValueError:
            assert isinstance(sth, str)
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

    def first_parent(self, commit: Commit) -> Optional[Commit]:
        raw_commit: GitCommit = commit.raw_commit
        try:
            if not raw_commit.parents:
                return None
        except:  # pylint: disable=bare-except
            return None

        next_raw_commit: GitCommit = raw_commit.parents[0]
        return to_commit(self, next_raw_commit, commit)

    def walker(self,
               start_c: Optional[Commit] = None,
               end_c: Optional[Commit] = None,
               parent: Optional[Commit] = None) -> Iterator[Commit]:
        if not start_c:
            start = self._repo.head.target  # pylint: disable=no-member
        else:
            start = start_c.oid

        end = None
        if end_c:
            end = end_c.oid or None

        git_commit = self._repo[start]
        parent = to_commit(self, git_commit, parent)
        yield parent
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
            yield tmp
            parent = tmp


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

    def children(self) -> Iterator[Commit]:
        ''' Get all the parent commits without the first parent. '''
        for commit in self.raw_commit.parents[1:]:
            yield to_commit(self._repo, commit, self)

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
                        self._repo._repo.descendant_of(  # pylint: disable=protected-access
                            self._commit.parents[1].id,
                            self._commit.parents[0].id)
            return self._rebased
        except:  # pylint: disable=bare-except
            return False

    @property
    def icon(self) -> str:
        if self.noffff:
            return "……"

        if self.subject().startswith('Update :'):
            return '◎─╮'

        if isinstance(self.parent, Foldable) \
            and self.parent.is_rebased():
            return "●─┤"

        return "●─╮"

    @property
    def is_folded(self) -> bool:
        return self._folded

    def unfold(self) -> Any:
        self._folded = False

    def fold(self) -> Any:
        self._folded = True


class ForkPoint(Commit):
    @property
    def icon(self) -> str:
        return "●─╯"


class InitialCommit(Commit):
    @property
    def icon(self) -> str:
        if self.noffff:
            return "……"

        return "◉"


class CommitLink(Commit):
    @property
    def icon(self) -> str:
        return "└─"


class Merge(Foldable):
    pass


class Crossroads(Merge):
    @property
    def icon(self) -> str:
        return "●─┤"


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
