# pylint: disable=missing-docstring,fixme
from datetime import datetime
from typing import Iterator, Optional

import babel.dates
from pygit2 import Commit as GitCommit  # pylint: disable=no-name-in-module
from pygit2 import Oid  # pylint: disable=no-name-in-module
from pygit2 import discover_repository  # pylint: disable=no-name-in-module
from pygit2 import Repository as GitRepo  # pylint: disable=no-name-in-module


class Commit:
    ''' Wrapper object around a pygit2.Commit object. '''

    def __init__(self, commit: GitCommit, parent=None, level: int = 0) -> None:
        self._commit = commit
        self.level = level
        self._parent = parent
        self._oid = commit.id
        self.noffff = False

    def author_name(self) -> str:
        ''' Returns author name with mail as string. '''
        commit = self._commit
        return commit.author.name + " <" + commit.author.email + ">"

    def author_date(self):
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        timestamp: int = self._commit.author.time
        delta = datetime.now() - datetime.fromtimestamp(timestamp)
        return babel.dates.format_timedelta(delta, format='short').strip('.')

    def commiter_name(self) -> str:
        ''' Returns commiter name with mail as string. '''
        commit = self._commit
        return commit.committer.name + " <" + commit.committer.email + ">"

    def commiter_date(self):
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        timestamp: int = self._commit.committer.time
        delta = datetime.now() - datetime.fromtimestamp(timestamp)
        return babel.dates.format_timedelta(delta, format='short').strip('.')

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

    def __repr__(self) -> str:
        return str(self._commit.id)

    def __str__(self):
        hash_id: str = self.short_id()
        rel_date: str = self.author_date()
        author = self.author_name().split()[0]
        return " ".join([hash_id, rel_date, author, self.subject()])

    @property
    def parent(self):
        return self._parent

    @property
    def is_top(self) -> bool:
        return self._parent is not None


class Repo:
    ''' A wrapper around `pygit2.Repository`. '''

    def __init__(self, path):
        self._repo = GitRepo(discover_repository(path))

    def get(self, sth) -> Commit:
        try:
            git_commit = self._repo[sth]
        except ValueError:
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

    def is_connected(self, child: Commit, parent_child=1) -> bool:
        if not isinstance(child.parent, Foldable):
            return False
        base = self.merge_base(child.raw_commit,
                               child.parent.raw_commit.parents[parent_child])
        if not base:
            return False
        return base.oid == child.oid

    def walker(self, start=None, end=None, parent=None) -> Iterator[Commit]:
        if not start:
            start = self._repo.head.target
        elif isinstance(start, str):
            start = self._repo.revparse_single(start).id
        elif isinstance(start, Commit):
            start = start.oid
        elif isinstance(start, GitCommit):
            start = start.id

        if isinstance(end, str):
            end = self._repo.revparse_single(end).id
        elif isinstance(end, Commit):
            end = end.oid

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

    def revparse_single(self, text: str):
        git_commit = self._repo.revparse_single(text)
        return to_commit(self, git_commit)


class Foldable(Commit):
    def __init__(self,
                 repo: Repo,
                 commit: GitCommit,
                 parent=None,
                 level: int = 1) -> None:
        super().__init__(commit, parent, level)
        self._folded = True
        self._repo = repo

    def children(self) -> Iterator[Commit]:
        ''' Get all the parent commits without the first parent. '''
        for commit in self.raw_commit.parents[1:]:
            yield to_commit(self._repo, commit, self)

    def child_log(self) -> Iterator[Commit]:
        start = self.raw_commit.parents[1]
        end = self._repo.merge_base(self.raw_commit.parents[0],
                                    self.raw_commit.parents[1])
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
    def is_folded(self):
        return self._folded

    def unfold(self):
        self._folded = False

    def fold(self):
        self._folded = True


class InitialCommit(Commit):
    pass


class LastCommit(Commit):
    pass


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
    pass


def _calculate_level(parent: Commit) -> int:
    level = 1
    if parent is not None:
        level = parent.level
        if isinstance(parent, Foldable):
            level += 1
    return level


def to_commit(repo: Repo, git_commit: GitCommit, parent: Commit = None):
    level = 0
    try:
        if not git_commit.parents:
            return InitialCommit(git_commit, parent, level)
    except Exception:  # pylint: disable=broad-except
        return LastCommit(git_commit, parent, level)

    parents_len = len(git_commit.parents)
    if parents_len == 1:
        return Commit(git_commit, parent, level)
    elif parents_len == 2:
        return Merge(repo, git_commit, level=level, parent=parent)

    return Octopus(repo, git_commit, level=level, parent=parent)
