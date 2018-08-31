# pylint: disable=missing-docstring,fixme
from datetime import datetime
from enum import Enum
from typing import Iterator

import babel.dates
from pygit2 import Commit as GitCommit  # pylint: disable=no-name-in-module
from pygit2 import Repository as GitRepo  # pylint: disable=no-name-in-module
from pygit2 import discover_repository  # pylint: disable=no-name-in-module


class CommitType(Enum):
    ''' Encodes the commit type.

        - UNKNOWN This is a commit which is the last one in a shallow repository
        - INITIAL The first commit in a repository
        - SIMPLE A commit with one parent
        - MERGE A commit with or more parents
    '''  # noqa: E501
    TOP = -2
    UNKNOWN = -1
    INITIAL = 0
    SIMPLE = 1
    MERGE = 2


class Commit:
    ''' Wrapper object around a pygit2.Commit object. '''

    def __init__(self, commit: GitCommit, parent=None, level: int = 1) -> None:
        self._commit = commit
        self._level = level
        self._parent = parent

    def commiter_name(self) -> str:
        ''' Returns commiter name with mail as string. '''
        commit = self._commit
        return commit.committer.name + " <" + commit.committer.email + ">"

    def commit_type(self) -> CommitType:
        ''' Returns the commit type. '''
        if not self._parent:
            return CommitType.TOP
        try:
            if not self._commit.parents:
                return CommitType.INITIAL
            elif len(self._commit.parents) == 1:
                return CommitType.SIMPLE
                # TODO Add support for ocotopus branch display
            return CommitType.MERGE
        except Exception:  # pylint: disable=broad-except
            return CommitType.UNKNOWN  # Happens in shallow repositories

    def commiter_date(self):
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        timestamp: int = self._commit.committer.time
        delta = datetime.now() - datetime.fromtimestamp(timestamp)
        return babel.dates.format_timedelta(delta, format='short').strip('.')

    @property
    def level(self):
        ''' Returns the commit’s level. '''
        return self._level

    def subject(self) -> str:
        ''' Returns the first line of the commit message. '''
        return self._commit.message.strip().splitlines()[0]

    def short_id(self, max_len: int = 8) -> str:
        ''' Returns a shortend commit id. '''
        return str(self._commit.id)[0:max_len - 1]

    def __repr__(self) -> str:
        return str(self._commit.id)

    def __str__(self):
        hash_id: str = self.short_id()
        rel_date: str = self.commiter_date()
        author = self.commiter_name().split()[0]
        return " ".join([hash_id, rel_date, author, self.subject()])

    @property
    def is_top(self) -> bool:
        return self._parent is not None


class Foldable(Commit):
    def __init__(self,
                 repo: GitRepo,
                 commit: GitCommit,
                 parent=None,
                 level: int = 1) -> None:
        super().__init__(commit, parent, level)
        self._folded = True
        self._repo = repo

    def children(self) -> Iterator[Commit]:
        ''' Get all the parent commits without the first parent. '''
        for commit in self._commit.parents[1:]:
            yield to_commit(self._repo, commit, self)

    def child_log(self) -> Iterator[Commit]:
        end = self._repo.merge_base(self._commit, self._commit.parents[1])
        for git_commit in self._repo.walker(self._commit.parents[1].id, end):
            yield to_commit(self._repo.repo, git_commit, self)

    @property
    def is_folded(self):
        return self._folded

    def unfold(self):
        self._folded = False

    def fold(self):
        self._folded = True


class InitialCommit(Commit):
    def __init__(self, commit: GitCommit, parent, level) -> None:
        super().__init__(commit, level, parent)


class LastCommit(Commit):
    def __init__(self, commit: GitCommit, parent, level) -> None:
        super().__init__(commit, level, parent)


class Merge(Foldable):
    pass


class Octopus(Foldable):
    pass


class Subtree(Foldable):
    def child_log(self) -> Iterator[Commit]:
        for commit in next_commit(self._repo, self._commit.parents[1], None,
                                  self):
            yield commit


def _calculate_level(parent: Commit) -> int:
    level = 1
    if parent is not None:
        level = parent.level
        if isinstance(parent, Foldable):
            level += 1
    return level


def to_commit(repo: GitRepo, git_commit: GitCommit, parent: Commit = None):
    level = 1
    if parent is not None:
        level = _calculate_level(parent)
    try:
        if not git_commit.parents:
            return InitialCommit(git_commit, parent, level)
    except Exception:  # pylint: disable=broad-except
        return LastCommit(git_commit, parent, level)

    parents_len = len(git_commit.parents)
    if parents_len == 1:
        return Commit(git_commit, parent, level)
    elif parents_len == 2 and not repo.merge_base(git_commit.parents[0],
                                                  git_commit.parents[1]):
        return Subtree(repo, git_commit, level=level, parent=parent)
    elif parents_len == 2:
        return Merge(repo, git_commit, level=level, parent=parent)
    elif parents_len > 2:
        return Octopus(repo, git_commit, level=level, parent=parent)


class Repo:
    ''' A wrapper around `pygit2.Repository`. '''

    def __init__(self, path):
        self._repo = GitRepo(discover_repository(path))

    def walker(self, start=None, end=None, parent=None):
        if not start:
            start = self._repo.head.target
        elif isinstance(start, str):
            start = self._repo.revparse_single(start).id

        if isinstance(end, str):
            end = self._repo.revparse_single(end).id

        print(start)
        print(end)
        walker = self._repo.walk(start)
        walker.simplify_first_parent()
        if end:
            walker.hide(end)
        for git_commit in walker:
            print(git_commit)
            yield to_commit(self._repo, git_commit, parent)

    def merge_base(self, a, b):
        return self._repo.merge_base(a.id, b.id)


def next_commit(repo, start=None, end=None,
                parent: Commit = None) -> Iterator[Commit]:
    walker = repo.walker(start, end)
    for commit in walker:
        result = to_commit(repo, commit, parent)
        yield result
        parent = result
