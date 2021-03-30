# pylint: disable=missing-docstring
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
import itertools
import logging
import os
import re
import sys
from datetime import datetime, timezone
from threading import Thread
from typing import Any, List, Optional, Tuple

import babel.dates
import pkg_resources
from prompt_toolkit import shortcuts
from prompt_toolkit.buffer import Buffer
from prompt_toolkit.data_structures import Point
from prompt_toolkit.formatted_text import StyleAndTextTuples
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout import (BufferControl, Dimension, HSplit, UIContent,
                                   Window)
from prompt_toolkit.layout.controls import SearchBufferControl
from prompt_toolkit.search import SearchDirection, SearchState
from prompt_toolkit.widgets import SearchToolbar

from glv import NoPathMatches, NoRevisionMatches, Repo, proxies, utils, vcs
from glv.commit import Commit, child_history, is_folded
from glv.icon import ASCII
from glv.ui.status import STATUS, STATUS_WINDOW
from glv.utils import ModuleChanges, mod_changes, parse_args

LOG = logging.getLogger('glv')


def icon_collection():
    name = vcs.CONFIG['history']['icon_set']
    result = None
    for entry_point in pkg_resources.iter_entry_points(group='glv_icons'):
        if entry_point.name == name:
            try:
                result = entry_point.load()
            except ModuleNotFoundError as exc:
                LOG.error(exc)

    if not result:
        result = ASCII
    return result


def has_component(subject: str) -> bool:
    return re.match(r'^\w+\([\w\d_-]+\)[\s:]\s*.*', subject, flags=re.I)


def parse_component(subject: str) -> Optional[str]:
    tmp = re.findall(r'^\w+\(([\w\d_-]+)\):.*', subject)
    if tmp:
        return tmp[0]
    return None


def is_hex(subject: str) -> bool:
    return re.match(r'^[0-9a-f]+$', subject, flags=re.I)


def remove_component(subject: str) -> bool:
    return re.sub(r'^(\w+)\([\w\d_-]+\)', '\\1', subject, flags=re.I, count=1)


def parse_verb(subject: str) -> Optional[str]:
    tmp = re.findall(r'^(\w+)(?:\([\w\d_-]+\)\s*:)?', subject, re.I)
    if tmp:
        return tmp[0]
    return None


def remove_verb(subject: str) -> bool:
    return re.sub(r'^(\w+)((?=\()|\s*:|\s)\s*',
                  '',
                  subject,
                  flags=re.I,
                  count=1)


class LogEntry:
    def __init__(self, commit: Commit, working_dir: str) -> None:
        self.commit = commit
        self._working_dir = working_dir

    @property
    def author_date(self) -> str:
        delta = datetime.now(timezone.utc) - datetime.fromisoformat(
            self.commit.author_date)
        _format = vcs.CONFIG['history']['author_date_format']
        try:
            return babel.dates.format_timedelta(delta, format=_format)
        except KeyError as exc:
            if delta.total_seconds() < 60:
                return f'{round(delta.total_seconds())} s'
            raise exc

    @property
    def committer_date(self) -> str:
        ''' Returns relative commiter date '''
        # pylint: disable=invalid-name
        delta = datetime.now(timezone.utc) - datetime.fromisoformat(
            self.commit.committer_date)
        _format = vcs.CONFIG['history']['author_date_format']
        try:
            return babel.dates.format_timedelta(delta, format=_format)
        except KeyError as e:
            if delta.total_seconds() < 60:
                return f'{round(delta.total_seconds())} s'
            raise e

    @property
    def modules(self) -> Tuple[str, str]:
        try:
            config = vcs.CONFIG['history']['modules_content']
        except KeyError:
            config = 'modules-component'

        try:
            modules_max_width = vcs.CONFIG['history']['modules_max_width']
        except KeyError:
            modules_max_width = 35

        changes: ModuleChanges = mod_changes(self._working_dir)
        modules = changes.commit_modules(self.commit)

        subject = self.commit.subject

        if config == 'modules-component' and not modules \
                and has_component(subject):
            parsed_module = parse_component(subject)
            if parsed_module and parsed_module not in modules and not is_hex(
                    parsed_module):
                modules.append(parsed_module)

        if config == 'component':
            modules = []
            if has_component(subject):
                parsed_module = parse_component(subject)
                if parsed_module:
                    modules = [parsed_module]

        text = ', '.join([':' + x for x in modules])
        if len(text) > modules_max_width:
            text = ':(%d modules)' % len(modules)
        return text

    @property
    def author_name(self):
        width = 10
        name = self.commit.author_name
        tmp = textwrap.shorten(name, width=width, placeholder="…")
        if tmp == '…':
            return name[0:width - 1] + '…'
        return tmp

    @property
    def short_id(self):
        return self.commit.short_id

    @property
    def icon(self) -> Tuple[str, str]:
        subject = self.commit.subject
        for (regex, icon) in icon_collection():
            if re.match(regex, subject, flags=re.I):
                return icon
        return '  '

    @property
    def subject(self) -> Tuple[str, str]:
        try:
            parts = vcs.CONFIG['history']['subject_parts'].split()
        except KeyError:
            parts = ['component', 'verb']

        subject = self.commit.subject
        if has_component(subject):
            component = parse_component(subject)
            if component and not is_hex(component):
                if 'modules-component' in parts:
                    modules = vcs.modules(self._working_dir)
                    if not modules or component in modules:
                        subject = remove_component(subject)
                elif 'component' not in parts:
                    subject = remove_component(subject)

        if 'icon-or-verb' in parts:
            if self.icon[1] != '  ':
                subject = remove_verb(subject)
        elif 'verb' not in parts:
            subject = remove_verb(subject)

        return subject

    @property
    def type(self):
        ''' Return the graph icon '''
        if self.commit.bellow is None:
            result = "◉"
        elif self.commit.is_commit_link:
            result = "⭞"
        else:
            result = "●"

        level = ''
        if self.commit.level > 0:
            level = self.commit.level * '│ '

        return level + result + self._arrows

    @property
    def _arrows(self) -> str:
        if self.commit.is_merge:
            if self.commit.subject.startswith('Update :') \
                    or ' Import ' in self.commit.subject:
                if self.commit.is_fork_point:
                    return "⇤┤"
                return '⇤╮'
            if self.commit.is_fork_point:
                return "─┤"
            return "─┐"
        if self.commit.is_fork_point:
            return "─┘"
        return ''


def format_branches(branches) -> List[Tuple[str, str]]:
    if branches == ['']:
        return []
    color = vcs.CONFIG['history']['branches_color']
    branch_tupples = [[('', ' '), (color, '«%s»' % name)] for name in branches
                      if not name.startswith('patches/')]
    return list(itertools.chain(*branch_tupples))


def highlight_substring(search: SearchState,
                        parts: Tuple[str, str]) -> StyleAndTextTuples:
    needle: str = search.text
    haystack = parts[1]
    matches = list(re.finditer(re.escape(needle), haystack))
    if not matches:
        return parts

    original_h = parts[0]
    new_h = parts[0] + ' ansired bold'
    cur = 0
    result = []
    if matches[0].start() == 0:
        match = matches[0]
        result = [(new_h, needle)]
        cur = len(needle)
        matches = matches[1:]

    for match in matches:
        result += [(original_h, haystack[cur:match.start()])]
        result += [(new_h, haystack[match.start():match.end()])]
        cur = match.end()

    if cur < len(haystack):
        result += [(original_h, haystack[cur:])]
    return result


class History(UIContent):
    # pylint: disable=too-many-instance-attributes
    def __init__(self, arguments: dict) -> None:
        try:
            self.path, self.revision, self.files = parse_args(**arguments)
            repo = Repo(path=self.path)

            path = repo.working_dir.replace(os.path.expanduser('~'), '~', 1)
            self.working_dir = repo.working_dir.replace(
                os.path.expanduser('~'), '~', 1)
            revision = self.revision[0]
            if self.revision == 'HEAD':
                revision = repo._nrepo.head.ref.name
            title = '%s \uf418 %s' % (path.rstrip('/'), revision)

            shortcuts.set_title('%s - Git Log Viewer' % title)
        except NoRevisionMatches:
            print('No revisions match the given arguments.', file=sys.stderr)
            sys.exit(1)
        except NoPathMatches:
            print("No paths match the given arguments.", file=sys.stderr)
            sys.exit(1)

        self.date_max_len = 0
        self.name_max_len = 0
        self._repo = repo
        self.line_count = self._repo.count_commits(self.revision[0])
        self.commit_list: List[Commit] = []
        self.log_entry_list: List[Commit] = []
        self.search_state: Optional[SearchState] = None
        self._search_thread: Optional[Thread] = None
        super().__init__(line_count=self.line_count,
                         get_line=self.get_line,
                         show_cursor=False)
        self.fill_up(utils.screen_height())

    def apply_search(self,
                     search_state: SearchState,
                     include_current_position=True,
                     count=1):
        if self._search_thread is not None and self._search_thread.is_alive():
            try:
                self._search_thread._stop()  # pylint: disable=protected-access
            except Exception:  # nosec pylint: disable=broad-except
                pass
            finally:
                STATUS.clear()

        args = (search_state, include_current_position, count)
        self._search_thread = Thread(target=self.search,
                                     args=args,
                                     daemon=True)
        self._search_thread.start()

    def current(self, index: int) -> Optional[Commit]:
        LOG.debug("Fetching current for index %d", index)
        try:
            commit = self.commit_list[index]
            return commit
        except IndexError:
            LOG.info("No index %d in commit list", index)
            return None

    def search(self,
               search_state: SearchState,
               include_current_position=True,
               count=1):
        LOG.debug('applying search %r, %r, %r', search_state,
                  include_current_position, count)
        self.search_state = search_state
        index = self.cursor_position.y
        new_position = self.cursor_position.y
        LOG.debug('Current position %r', index)
        needle = self.search_state.text
        STATUS.set_status("Searching for '%s'" % needle)
        if self.search_state.direction == SearchDirection.FORWARD:
            if not include_current_position:
                index += 1
            while True:
                try:
                    commit = self.commit_list[index]
                except IndexError:
                    if not self.fill_up(utils.screen_height()):
                        break

                    commit = self.commit_list[index]

                if needle in commit.short_id or needle in commit.subject \
                        or needle in commit.author_name \
                        or any(needle in haystack for haystack in commit.branches):
                    new_position = index
                    break

                index += 1
        else:
            if not include_current_position and index > 0:
                index -= 1
            while index >= 0:
                commit = self.commit_list[index]
                if needle in commit.short_id() or needle in commit.subject \
                        or needle in commit.author_name():
                    new_position = index
                    break

                index -= 1

        if new_position != self.cursor_position.y:
            self.cursor_position = Point(x=self.cursor_position.x, y=index)
        STATUS.clear()

    def get_line(self, line_number: int) -> List[tuple]:  # pylint: disable=method-hidden
        length = len(self.commit_list)
        if length - 1 < line_number:
            amount = line_number - length + 1
            self.fill_up(amount)

        try:
            commit = self.commit_list[line_number]
        except IndexError:
            return [("", "")]

        return self._render_commit(commit, line_number)

    def _render_commit(self, commit: Commit, line_number: int) -> List[tuple]:
        colors = vcs.CONFIG['history']
        try:
            entry = proxies.ColorProxy(self.log_entry_list[line_number],
                                       colors)
        except KeyError:
            self.log_entry_list[line_number] = LogEntry(
                commit, self._repo.working_dir)
            entry = proxies.ColorProxy(self.log_entry_list[line_number],
                                       colors)

        _id = entry.short_id
        author_date = (entry.author_date[0],
                       entry.author_date[1].ljust(self.date_max_len, " "))
        author_name = (entry.author_name[0],
                       entry.author_name[1].ljust(self.name_max_len, " "))
        module = entry.modules
        subject = entry.subject
        branches = format_branches(commit.references)

        if self.search_state and self.search_state.text in _id[1]:
            _id = highlight_substring(self.search_state, _id)

        if self.search_state and self.search_state.text in module[1]:
            module = highlight_substring(self.search_state, module)

        if self.search_state and self.search_state.text in author_name[1]:
            author_name = highlight_substring(self.search_state, author_name)

        if self.search_state and self.search_state.text in subject[1]:
            subject = highlight_substring(self.search_state, subject)

        if commit.is_commit_link:
            _id = ('italic ' + _id[0], _id[1])
            module = ('italic ' + module[0], module[1])
            subject = ('italic ' + subject[0], subject[1])
            author_name = ('italic ' + author_name[0], author_name[1])
            author_date = ('italic ' + author_date[0], author_date[1])

        tmp = [
            _id, author_date, author_name, entry.icon, entry.type, module,
            subject
        ]
        result: List[tuple] = []
        for sth in tmp:
            if isinstance(sth, tuple):
                result += [sth, ('', ' ')]
            else:
                result += sth
                result += [('', ' ')]

        if branches:
            result += branches

        if line_number == self.cursor_position.y:
            result = [('reverse ' + x[0], x[1]) for x in result]

        return [(x[0], x[1]) for x in result]

    def toggle_fold(self, line_number):
        commit = self.commit_list[line_number]
        if not commit.is_merge:
            return

        if is_folded(self.commit_list, line_number):
            self._unfold(line_number, commit)
        else:
            self._fold(line_number + 1, commit)

    def _fold(self, pos: int, commit: Commit) -> Any:
        LOG.info('Expected level %s', commit.level)
        for _, cur in enumerate(self.commit_list[pos:]):
            LOG.info('Checking %s', cur)
            if commit.level < cur.level:
                del self.commit_list[pos]
                del self.log_entry_list[pos]
            else:
                break

    def _unfold(self, line_number: int, commit: Commit) -> Any:
        index = 1
        for _ in child_history(self._repo.working_dir, commit):
            entry = LogEntry(_, self._repo.working_dir)
            if len(entry.author_date) > self.date_max_len:
                self.date_max_len = len(entry.author_date)
            if len(entry.author_name) > self.name_max_len:
                self.name_max_len = len(entry.author_name)
            self.commit_list.insert(line_number + index, _)
            self.log_entry_list.insert(line_number + index, entry)
            index += 1

        self.line_count += index

    def fill_up(self, amount: int) -> int:
        if amount <= 0:
            raise ValueError('Amount must be ≤ 0')

        commits = self._repo.iter_commits(
            rev_range=self.revision[0],
            skip=len([x for x in self.commit_list if x.level == 0]),
            max_count=amount,
            paths=self.files)
        for commit in commits:
            self.commit_list.append(commit)
            entry = LogEntry(commit, self._repo.working_dir)
            self.log_entry_list.append(entry)
            if len(entry.author_date) > self.date_max_len:
                self.date_max_len = len(entry.author_date)
            if len(entry.author_name) > self.name_max_len:
                self.name_max_len = len(entry.author_name)
        return len(commits)


class HistoryControl(BufferControl):
    def __init__(self, search_buffer_control: SearchBufferControl,
                 key_bindings: Optional[KeyBindings], arguments: dict) -> None:
        buffer = Buffer(name='history')
        self.content = History(arguments)
        buffer.apply_search = self.content.apply_search  # type: ignore
        super().__init__(buffer=buffer,
                         search_buffer_control=search_buffer_control,
                         focus_on_click=True,
                         key_bindings=key_bindings)

    def is_focusable(self) -> bool:
        return True

    @property
    def current_line(self) -> int:
        return self.content.cursor_position.y

    def create_content(self, width, height, preview_search=False):
        return self.content

    def current(self) -> Optional[Commit]:
        return self.content.current(self.current_line)

    @property
    def working_dir(self) -> str:
        return self.content.working_dir

    def move_cursor_down(self):
        old_point = self.content.cursor_position
        if old_point.y + 1 < self.content.line_count:
            new_position = Point(x=old_point.x, y=old_point.y + 1)
            self.content.cursor_position = new_position

    def move_cursor_up(self):
        old_point = self.content.cursor_position
        if old_point.y > 0:
            new_position = Point(x=old_point.x, y=old_point.y - 1)
            self.content.cursor_position = new_position

    def goto_line(self, line_number):
        if line_number < 0:
            line_number = self.content.line_count + 1 - line_number
            if line_number < 0:
                line_number = 0
        elif line_number >= self.content.line_count:
            line_number = self.content.line_count - 1

        if self.current_line != line_number:
            old_point = self.content.cursor_position
            new_position = Point(x=old_point.x, y=line_number)
            self.content.cursor_position = new_position

    def goto_last(self):
        old_point = self.content.cursor_position
        if old_point.y < self.content.line_count:
            new_position = Point(x=old_point.x, y=self.content.line_count - 1)
            self.content.cursor_position = new_position

    def toggle_fold(self, line_number):
        self.content.toggle_fold(line_number)

    def is_folded(self, line_number: int) -> bool:
        commit = self.content.commit_list[line_number]
        if commit.is_merge:
            return commit.is_folded
        return False

    def is_foldable(self, line_number: int) -> bool:
        commit = self.content.commit_list[line_number]
        return commit.is_merge

    def is_child(self, line_number: int) -> bool:
        commit = self.content.commit_list[line_number]
        return commit.level > 0

    def go_to_parent(self, line_number: int):
        commit = self.content.commit_list[line_number]
        if commit.level > 0 and line_number > 0:
            i = line_number - 1
            while i >= 0:
                candidat = self.content.commit_list[i]
                if candidat.level < commit.level:
                    self.goto_line(i)
                    break
                i -= 1

    def is_link(self, line_number: int) -> bool:
        commit = self.content.commit_list[line_number]
        return commit.is_commit_link

    def go_to_link(self, line_number: int):
        commit = self.content.commit_list[line_number]

        if not commit.is_commit_link:
            raise ValueError('Expected CommitLinkt got %s' % commit)

        i = line_number + 1
        while i < line_number + 400:
            try:
                candidat = self.content.commit_list[i]
            except IndexError:
                self.content.fill_up(utils.screen_height())

            if candidat.short_id() == commit.short_id():
                self.goto_line(i)
                break
            i += 1

    @property
    def path(self) -> str:
        return self.path


class HistoryContainer(HSplit):
    def __init__(self, key_bindings, arguments, right_margins=None):
        search = SearchToolbar(vi_mode=True)
        log_view = HistoryControl(search.control,
                                  key_bindings=key_bindings,
                                  arguments=arguments)
        window = Window(content=log_view, right_margins=right_margins)
        super().__init__([window, search, STATUS_WINDOW])

    def preferred_width(self, max_available_width: int) -> Dimension:
        _min = 40
        preferred = 80
        if max_available_width / 2 >= 80:
            preferred = max_available_width / 2

        return Dimension(min=_min, preferred=preferred)
