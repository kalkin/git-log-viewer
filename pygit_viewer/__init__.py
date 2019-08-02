# pylint: disable=missing-docstring,fixme
import functools
import itertools
import os
import sys
import time
from datetime import datetime
from typing import Any, Iterator, List, Optional, Union

import babel.dates
import pkg_resources
from pygit2 import Commit as GitCommit  # pylint: disable=no-name-in-module
from pygit2 import Oid  # pylint: disable=no-name-in-module
from pygit2 import discover_repository  # pylint: disable=no-name-in-module
from pygit2 import Repository as GitRepo  # pylint: disable=no-name-in-module

import pygit_viewer.vcs as vcs
from pygit_viewer.providers import Cache


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
        return babel.dates.format_timedelta(delta, format='short').strip('.')

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
    def __stgit(self) -> bool:  # ↓ FIXME ↓
        # pylint: disable=protected-access
        for name in self._repo._branches:
            if name.startswith('patches/') \
            and self._repo._branches[name] == self.raw_commit:
                return True
        return False

    @property  # type: ignore
    @functools.lru_cache()
    def icon(self) -> str:
        if self.noffff:
            return "……"

        point = "●"
        if self.__stgit():
            point = "Ⓟ"

        if self.is_fork_point():
            return point + "─┘"

        return point

    def render(self):
        return LogEntry(self)

    @property
    def raw_commit(self) -> GitCommit:
        return self._commit

    @property
    def oid(self) -> Oid:
        return self._oid

    @functools.lru_cache()
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
            elif subject.split()[0].startswith(':') and self.modules():
                words = subject.split()
                subject = ' '.join(words[1:])
            return subject
        except IndexError:
            return ""

    @functools.lru_cache()
    def modules(self) -> str:
        if not self._repo.has_modules:
            return ''
        _id = str(self.oid)
        try:
            modules = self._repo.module_cache[_id]
            return ', '.join([':' + x for x in modules])
        except KeyError:
            # pylint: disable=protected-access
            try:
                modules = list(
                    vcs.changed_modules(self._repo._repo, self._commit))
                self._repo.module_cache[_id] = modules
                return ', '.join([':' + x for x in modules])
            except KeyError:
                pass
        return ''

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


class LogEntry:
    def __init__(self, commit: Commit) -> None:
        self.commit = commit

    @property
    def author_date(self):
        return ("ansiblue", self.commit.author_date())

    @property
    def modules(self):
        modules = self.commit.modules()
        if modules != '':
            return ('ansiyellow', modules + ' ')

        return ('', ' ')

    @property
    def author_name(self):
        return ("ansigreen", self.commit.short_author_name())

    @property
    def short_id(self):
        return ("ansimagenta", self.commit.short_id())

    @property
    def subject(self):
        return ('', self.commit.subject())

    @property
    def type(self):
        level = self.commit.level * '│ '
        _type = level + self.commit.icon.ljust(4, " ")
        return ("bold", _type)

    @property
    def branches(self):
        branches = self.commit._repo.branches(self.commit)
        if branches:
            return branches
        return None


def providers():
    named_objects = {}
    for entry_point in pkg_resources.iter_entry_points(
            group='pygit_viewer_plugins'):
        named_objects.update({entry_point.name: entry_point.load()})
    return named_objects


class Repo:
    ''' A wrapper around `pygit2.Repository`. '''

    def __init__(self,
                 path: str,
                 revision: str = 'HEAD',
                 files: List[str] = None) -> None:
        self.provider = None
        self.files = files or []
        repo_path = discover_repository(path)
        if not repo_path:
            print(' Not a git repository', file=sys.stderr)
            sys.exit(2)
        self._repo = GitRepo(repo_path)
        self.module_cache = Cache(repo_path + '/pygit-viewer/modules.json')
        self.has_modules = False
        if vcs.modules(self._repo):
            self.has_modules = True
        self._branches = {
            r.shorthand: r.peel()
            for r in self._repo.references.objects
            if not r.shorthand.endswith('/HEAD')
        }
        self.__start: GitCommit = self._repo.revparse_single(revision)
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

    @functools.lru_cache()
    def branches(self, commit: Commit):
        branch_tupples = [[('', ' '), ('ansiyellow', '«%s»' % name)]
                          for name in self._branches
                          if self._branches[name] == commit.raw_commit
                          and not name.startswith('patches/')]
        return list(itertools.chain(*branch_tupples))

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

        git_commit = self._repo[start]
        while self.files and not _commit_changed_files(git_commit, self.files):
            git_commit = git_commit.parents[0]

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


def _commit_changed_files(commit: GitCommit, files: List[str]) -> bool:
    try:
        changed_files = vcs.changed_files(commit)  # pylint: disable=protected-access
        for _file in files:
            if _file in changed_files:
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
    @functools.lru_cache()
    def icon(self) -> str:
        if self.noffff:
            return "……"

        if self.subject().startswith('Update :'):
            if isinstance(self.parent, Foldable) and self.parent.is_rebased():
                return '●⇤┤'

            return '●⇤╮'

        if isinstance(self.parent, Foldable) \
            and self.parent.is_rebased():
            return "●─┤"

        return "●─┐"

    @property
    def is_folded(self) -> bool:
        return self._folded

    def unfold(self) -> Any:
        self._folded = False

    def fold(self) -> Any:
        self._folded = True


class ForkPoint(Commit):
    @property  # type: ignore
    @functools.lru_cache()
    def icon(self) -> str:
        return "●─┘"


class InitialCommit(Commit):
    @property  # type: ignore
    @functools.lru_cache()
    def icon(self) -> str:
        if self.noffff:
            return "……"

        return "◉"


class CommitLink(Commit):
    @property  # type: ignore
    @functools.lru_cache()
    def icon(self) -> str:
        return "↘"


class Merge(Foldable):
    pass


class Crossroads(Merge):
    @property  # type: ignore
    @functools.lru_cache()
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
