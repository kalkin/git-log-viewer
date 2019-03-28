# pylint: disable=missing-docstring,fixme
import random
import time
from datetime import datetime
from typing import Any, Iterator, Optional, Union

import babel.dates
from pygit2 import Commit as GitCommit  # pylint: disable=no-name-in-module
from pygit2 import Oid  # pylint: disable=no-name-in-module
from pygit2 import discover_repository  # pylint: disable=no-name-in-module
from pygit2 import Repository as GitRepo  # pylint: disable=no-name-in-module


class Commit:
    ''' Wrapper object around a pygit2.Commit object. '''

    def __init__(self,
                 commit: GitCommit,
                 parent: Optional['Commit'] = None,
                 level: int = 0) -> None:
        self._commit: GitCommit = commit
        self.level: int = level
        self._parent: Optional['Commit'] = parent
        self._oid: Oid = commit.id
        self.noffff: bool = False

    def author_name(self) -> str:
        ''' Returns author name with mail as string. '''
        commit = self._commit
        return commit.author.name + " <" + commit.author.email + ">"

    def author_date(self) -> str:
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        timestamp: int = self._commit.author.time
        delta = datetime.now() - datetime.fromtimestamp(timestamp)
        return babel.dates.format_timedelta(delta, format='short').strip('.')

    def commiter_name(self) -> str:
        ''' Returns commiter name with mail as string. '''
        commit = self._commit
        return commit.committer.name + " <" + commit.committer.email + ">"

    def commiter_date(self) -> str:
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        timestamp: int = self._commit.committer.time
        delta = datetime.now() - datetime.fromtimestamp(timestamp)
        return babel.dates.format_timedelta(delta, format='short').strip('.')

    @property
    def next(self) -> 'Commit':
        if self._commit.parents:
            commit = self._commit.parents[0]
            return Commit(commit, self, level=self.level)
        raise IndexError

    @property
    def icon(self) -> str:
        if self.noffff:
            return "……"

        return "●"

    @property
    def raw_commit(self) -> GitCommit:
        return self._commit

    @property
    def oid(self) -> Oid:
        return self._oid

    def subject(self) -> str:
        ''' Returns the first line of the commit message. '''
        try:
            return self._commit.message.strip().splitlines()[0]
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


class Repo:
    ''' A wrapper around `pygit2.Repository`. '''

    def __init__(self, path: str) -> None:
        self._repo = GitRepo(discover_repository(path))

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

    def is_connected(self, child: Commit, parent_child: int = 1) -> bool:
        if not isinstance(child.parent, Foldable):
            return False
        base = self.merge_base(child.raw_commit,
                               child.parent.raw_commit.parents[parent_child])
        if not base:
            return False
        return base.oid == child.oid

    def first_parent(self, commit: Commit) -> Commit:
        raw_commit: GitCommit = commit.raw_commit
        if not raw_commit.parents:
            raise Exception('No child commits')

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

    def revparse_single(self, text: str) -> Commit:
        git_commit = self._repo.revparse_single(text)
        return to_commit(self, git_commit)


class Foldable(Commit):
    def __init__(self,
                 repo: Repo,
                 commit: GitCommit,
                 parent: Optional[Commit] = None,
                 level: int = 1) -> None:
        super().__init__(commit, parent, level)
        self._folded = True
        self._repo = repo

    def children(self) -> Iterator[Commit]:
        ''' Get all the parent commits without the first parent. '''
        for commit in self.raw_commit.parents[1:]:
            yield to_commit(self._repo, commit, self)

    def child_log(self) -> Iterator[Commit]:
        start: GitCommit = self.raw_commit.parents[1]
        end: Optional[Commit] = self._repo.merge_base(
            self.raw_commit.parents[0], self.raw_commit.parents[1])
        not_first_merge = False

        for commit in self._repo.walker(start, end, self):
            commit.level = self.level + 1
            yield commit
            if end and commit.raw_commit.parents:
                if end.oid == commit.raw_commit.parents[0].id:
                    if not_first_merge:
                        end.level += 1
                        end.noffff = True
                        yield end
                    break
                elif end.oid in [_.id for _ in commit.raw_commit.parents]:
                    end = self._repo.merge_base(self.raw_commit.parents[0],
                                                commit.raw_commit.parents[0])
                    not_first_merge = True

    @property
    def icon(self) -> str:
        if self.noffff:
            return "……"
        # if isinstance(self.parent, Foldable) \
        # and self.oid != self.parent.raw_commit.parents[1].id \
        # and self._repo.is_connected(self, 1):
        # return "●─╯"
        if isinstance(self.parent, Foldable) \
        and self.oid == self.parent.raw_commit.parents[0].id:
            return "●─┤"

        return "●─╮"

    @property
    def is_folded(self) -> bool:
        return self._folded

    def unfold(self) -> Any:
        self._folded = False

    def fold(self) -> Any:
        self._folded = True


class InitialCommit(Commit):
    @property
    def icon(self) -> str:
        if self.noffff:
            return "……"

        return "◉"


class LastCommit(Commit):
    @property
    def icon(self) -> str:
        if self.noffff:
            return "……"

        return "✂"


class Merge(Foldable):
    def subject(self) -> str:
        ''' Returns the first line of the commit message. '''
        try:
            subject = self._commit.message.strip().splitlines()[0]
            if subject.startswith("Merge pull request #"):
                words = subject.split()
                subject = ' '.join(words[3:])
                subject = 'MERGE: ' + subject
            return subject
        except IndexError:
            return ""


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


def to_commit(repo: Repo,
              git_commit: GitCommit,
              parent: Optional[Commit] = None) -> Commit:
    level = 0
    try:
        if not git_commit.parents:
            return InitialCommit(git_commit, parent, level)
    except Exception:  # pylint: disable=broad-except
        return LastCommit(git_commit, parent, level)

    parents_len = len(git_commit.parents)
    if parents_len == 1:
        return Commit(git_commit, parent, level)

    if parents_len == 2:
        return Merge(repo, git_commit, level=level, parent=parent)

    return Octopus(repo, git_commit, level=level, parent=parent)
