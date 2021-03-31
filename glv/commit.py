#
# Copyright (c) 2021 Bahtiar `kalkin-` Gadimov.
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
''' New core api objects '''

import logging
from collections import namedtuple
from datetime import datetime, timezone
from typing import Optional

import babel
import git

from glv import vcs

LOG = logging.getLogger('glv')

Commit = namedtuple(
    'Commit',
    [
        'oid',
        'short_id',
        'author_name',
        'author_email',
        'author_date',
        'author_rel_date',
        'committer_name',
        'committer_email',
        'committer_date',
        'subject',
        'type_icon',
        # optional
        'above',
        'bellow',
        'branches',
        'children',
        'has_level',
        'is_commit_link',
        'is_fork_point',
        'is_head',
        'is_merge',
        'level',
        'references',
        'tags',
    ],
    defaults=[
        None,  # above
        None,  # bellow
        [],  # branches
        [],  # children
        False,  # has_level
        False,  # is_commit_link
        False,  # is_fork_point
        False,  # is_head
        False,  # is_merge
        0,  # level
        [],  # references
        [],  # tags
    ])

REV_FORMAT = '%h%x00'  # abbrev hash
REV_FORMAT += '%P%x00'  # parents
REV_FORMAT += '%D%x00'  # references
REV_FORMAT += '%aN%x00%aE%x00%aI%x00'  # mailmap author name/mail/date
REV_FORMAT += '%cN%x00%cE%x00%cI%x00'  # mailmap committer name/mail/date
REV_FORMAT += '%s%x00'  # subject


def to_commit(working_dir: str, oid: str, **kwargs) -> Commit:
    ''' Return data for one specific commit '''
    git_cmd = git.cmd.Git(working_dir=working_dir)
    git_p = git_cmd.rev_list(oid,
                             format=REV_FORMAT,
                             max_count=1,
                             as_process=True)
    stream = git_p.proc.stdout
    line1 = stream.readline()
    line2 = stream.readline()
    return parse_commit(working_dir, line1, line2, **kwargs)


def parse_commit(working_dir: str,
                 line1: bytes,
                 data_line: bytes,
                 above_commit: Commit = None,
                 level=-1,
                 is_commit_link=False) -> Commit:
    ''' Just a helpful wrapper.  '''

    # pylint: disable=too-many-locals,too-many-arguments
    assert line1.startswith(b'commit ')  # nosec
    oid = line1.partition(b' ')[2].decode().strip()
    short_id, parents_record, reference_record, \
            auth_name, auth_email, auth_date, \
            com_name, com_email, com_date, \
            subject, _ = [
            x.decode() for x in data_line.split(b'\0')
        ]

    auth_rel_date = _to_rel_date(datetime.fromisoformat(auth_date))
    is_head = False
    references = []
    branches = []
    tags = []
    for smth in reference_record.split(', '):
        if smth == 'HEAD':
            is_head = True
        elif smth.startswith('HEAD -> '):
            is_head = True
            _, _, branch = smth.partition(' -> ')
            references.append(branch)
            branches.append(branch)
        elif smth.startswith('tag: '):
            _, _, tag = smth.partition(': ')
            references.append(tag)
            tags.append(tag)
        else:
            references.append(smth)
            branches.append(smth)

    _children: list[str] = []
    parents = parents_record.split(' ')
    bellow = None
    children = []
    is_merge = False
    above = None
    if above_commit:
        above = above_commit.oid

    git_cmd = git.cmd.Git(working_dir)
    is_fork_point = False
    if above_commit and above_commit.children and above_commit.level == level:
        try:
            git_cmd.merge_base(oid, above_commit.children[0], is_ancestor=True)
            is_fork_point = True
        except git.GitCommandError:
            pass

    if parents:
        bellow = parents[0]
        if len(parents) > 1:
            children = parents[1:]
            is_merge = True

    type_icon_level = ''
    if level > 0:
        type_icon_level = level * '│ '
    type_icon = type_icon_level + _type_icon(bellow, is_commit_link)

    return Commit(oid,
                  short_id,
                  auth_name,
                  auth_email,
                  auth_date,
                  auth_rel_date,
                  com_name,
                  com_email,
                  com_date,
                  subject,
                  type_icon,
                  above=above,
                  bellow=bellow,
                  level=level,
                  has_level=level >= 0,
                  is_commit_link=is_commit_link,
                  is_fork_point=is_fork_point,
                  branches=branches,
                  children=children,
                  is_head=is_head,
                  is_merge=is_merge,
                  references=references,
                  tags=tags)


def commits_for_range(  # pylint: disable=too-many-arguments
        working_dir: str,
        rev_range: str,
        above_commit: str = None,
        level: int = 0,
        paths: list[str] = None,
        rev_list_args: dict = None) -> list[Commit]:
    ''' Return commits for a range '''
    # pylint: disable=too-many-locals

    rev_list_args = rev_list_args or {}
    git_cmd = git.cmd.Git(working_dir=working_dir)
    args = [rev_range]
    if paths:
        args.append('--')
        args += paths

    cmd_proc = git_cmd.rev_list(args,
                                first_parent=True,
                                format=REV_FORMAT,
                                **rev_list_args,
                                as_process=True)
    result = []
    parse_data = False
    stream = cmd_proc.proc.stdout
    above = above_commit
    while True:
        assert not parse_data  # nosec
        line1 = stream.readline()
        if not line1:
            break
        line2 = stream.readline()
        if not line2:
            break
        _commit = parse_commit(working_dir,
                               line1,
                               line2,
                               above_commit=above,
                               level=level)
        result.append(_commit)
        above = _commit

    return result


def merge_base(working_dir: str, *oids) -> Optional[str]:
    ''' Return the mergebase commit id '''
    git_cmd = git.cmd.Git(working_dir=working_dir)
    try:
        return git_cmd.merge_base(*oids).strip()
    except git.GitCommandError:
        return None


def child_history(working_dir: str, commit: Commit) -> list[Commit]:
    ''' Return history for the fist child commit '''
    assert commit.is_merge  # nosec
    bellow: str = commit.bellow
    first_child: str = commit.children[0]
    end: str = merge_base(working_dir, bellow, first_child)
    revision = '%s..%s' % (end, first_child)
    if not end:
        revision = '%s' % first_child
    LOG.debug('commits_for_range(%r)', revision)
    result = commits_for_range(working_dir,
                               revision,
                               above_commit=commit,
                               level=commit.level + 1)

    end_commit = result[-1]
    if end and end_commit.bellow != bellow:
        # Add CommitLink
        commit_link = to_commit(working_dir,
                                end_commit.bellow,
                                above_commit=end_commit,
                                level=commit.level + 1,
                                is_commit_link=True)
        result.append(commit_link)

    return result


def _to_rel_date(date) -> str:
    now = datetime.now(timezone.utc)
    delta = now - date
    _format = vcs.CONFIG['history']['author_date_format']
    try:
        return babel.dates.format_timedelta(delta, format=_format)
    except KeyError as exc:
        if delta.total_seconds() < 60:
            return f'{round(delta.total_seconds())} s'
        raise exc


def is_folded(commit_list: list[Commit], pos: int) -> bool:
    ''' Return true if the commit at pos is folded or not by looking at the next
        commit in the list.
    '''
    if pos >= len(commit_list):
        raise ValueError('Position %d should be less or equal the length %d' %
                         (pos, len(commit_list)))

    if pos == len(commit_list) - 1:
        return False
    assert commit_list[pos].is_merge  # nosec
    actual = commit_list[pos + 1].level
    expected = commit_list[pos].level
    return expected == actual


def _type_icon(bellow: Commit, is_commit_link: bool) -> str:
    ''' Return the graph icon '''
    if bellow is None:
        return "◉"
    if is_commit_link:
        return "⭞"
    return "●"


class CommitNotFound(Exception):
    ''' Thrown when following a link or searching fails '''


def find_non_link(working_dir: str, commit_list: list[Commit],
                  needle: Commit) -> int:
    '''
        Search recursively in a commit list for specified commit which is not a
        link. Will modify the commit_list if needed.
    '''
    needle_date = datetime.fromisoformat(needle.committer_date)
    for i, commit in enumerate(commit_list):
        if commit.is_commit_link:
            continue
        if commit.oid == needle.oid:
            return i
        commit_date = datetime.fromisoformat(commit.committer_date)
        if commit_date < needle_date:
            break
        if commit.is_merge and \
                merge_base(working_dir, needle.oid, commit.children[0]):
            # ↑ optimization for not descending just imported subtrees \
            children = child_history(working_dir, commit)
            try:
                tmp = find_non_link(working_dir, children, needle)
                commit_list[i + 1:i + 1] = children
                return tmp + i + 1
            except CommitNotFound:
                pass

    raise CommitNotFound


def follow(working_dir: str, commit_list: list[Commit], pos: int) -> int:
    '''
        Find link target in the commit_list. Will fill up commit list if
        needed.
    '''
    if pos > len(commit_list):
        raise ValueError('Position %d should be less then length %d' %
                         (pos, len(commit_list)))
    link: Commit = commit_list[pos]

    try:
        # May be the link is already in the commit list?
        return find_non_link(working_dir, commit_list, link)
    except CommitNotFound:
        pass

    last_commit = commit_list[-1]
    last_oid: str = last_commit.oid
    end = merge_base(working_dir, last_oid, link.oid)
    if not end:
        raise CommitNotFound('Commit has no mergebase')

    rev_range = "%s~1..%s~1" % (end, last_oid)
    tmp = commits_for_range(working_dir,
                            rev_range,
                            above_commit=last_commit,
                            level=commit_list[-1].level)
    if tmp:
        commit_list += tmp
        return find_non_link(working_dir, commit_list, link)

    raise CommitNotFound
