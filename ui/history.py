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
import logging
import re
from threading import Thread
from typing import Any, List, Optional, Tuple

from prompt_toolkit.buffer import Buffer
from prompt_toolkit.data_structures import Point
from prompt_toolkit.formatted_text import StyleAndTextTuples
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout import BufferControl, HSplit, UIContent, Window
from prompt_toolkit.layout.controls import SearchBufferControl
from prompt_toolkit.search import SearchDirection, SearchState
from prompt_toolkit.widgets import SearchToolbar

from glv import Commit, CommitLink, Foldable, Repo, utils
from glv.ui.status import STATUS, STATUS_WINDOW

LOG = logging.getLogger('glv')


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
    def __init__(self, repo: Repo) -> None:
        self.date_max_len = 0
        self.name_max_len = 0
        self._repo = repo
        self.line_count = len(list(self._repo.walker()))
        self.commit_list: List[Commit] = []
        self.search_state: Optional[SearchState] = None
        self.walker = self._repo.walker()
        self._search_thread: Optional[Thread] = None
        super().__init__(line_count=self.line_count,
                         get_line=self.get_line,
                         show_cursor=False)
        self.fill_up(utils.screen_height())

    def apply_search(self,
                     search_state: SearchState,
                     include_current_position=True,
                     count=1):
        if self._search_thread is not None and self._search_thread.isAlive():
            try:
                self._search_thread._stop()  # pylint: disable=protected-access
            except Exception:  # pylint: disable=broad-except
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

                if needle in commit.short_id() or needle in commit.subject() \
                        or needle in commit.short_author_name() or needle in commit.modules():
                    new_position = index
                    break

                index += 1
        else:
            if not include_current_position and index > 0:
                index -= 1
            while index >= 0:
                commit = self.commit_list[index]
                if needle in commit.short_id() or needle in commit.subject() \
                        or needle in commit.short_author_name() or needle in commit.modules():
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
        rendered = commit.render()
        _id = rendered.short_id
        author_date = (rendered.author_date[0],
                       rendered.author_date[1].ljust(self.date_max_len, " "))
        author_name = (rendered.author_name[0],
                       rendered.author_name[1].ljust(self.name_max_len, " "))
        icon = rendered.type
        module = rendered.modules
        subject = rendered.subject
        branches = rendered.branches()

        if isinstance(commit, CommitLink):
            if isinstance(subject, tuple):
                module = ('italic ' + module[0], module[1])
                subject = ('italic ' + subject[0], subject[1])
            else:
                subject = ('italic', subject)

        if self.search_state and self.search_state.text in _id[1]:
            _id = highlight_substring(self.search_state, _id)

        if self.search_state and self.search_state.text in module[1]:
            module = highlight_substring(self.search_state, module)

        if self.search_state and self.search_state.text in author_name[1]:
            author_name = highlight_substring(self.search_state, author_name)

        if self.search_state and self.search_state.text in subject[1]:
            subject = highlight_substring(self.search_state, subject)

        tmp = [_id, author_date, author_name, icon, module, subject]
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
        if not isinstance(commit, Foldable):
            return

        if commit.is_folded:
            self._unfold(line_number, commit)
        else:
            self._fold(line_number + 1, commit)

    def _fold(self, line_number: int, commit: Foldable) -> Any:
        if commit.is_folded:
            raise ValueError('Received an already folded commit')
        commit.fold()
        for _ in commit.child_log():
            cur_commit = self.commit_list[line_number]
            del self.commit_list[line_number]
            if isinstance(cur_commit, Foldable) and not cur_commit.is_folded:
                self._fold(line_number, cur_commit)
            self.line_count -= 1

    def _unfold(self, line_number: int, commit: Foldable) -> Any:
        if not commit.is_folded:
            raise ValueError('Received an already unfolded commit')
        commit.unfold()
        index = 1
        for _ in commit.child_log():
            if len(_.author_date()) > self.date_max_len:
                self.date_max_len = len(_.author_date())
            if len(_.short_author_name()) > self.name_max_len:
                self.name_max_len = len(_.short_author_name())
            self.commit_list.insert(line_number + index, _)
            index += 1

        self.line_count += index

    def fill_up(self, amount: int) -> int:
        if amount <= 0:
            raise ValueError('Amount must be ≤ 0')

        result = 0
        for _ in range(0, amount):
            try:
                commit: Commit = next(self.walker)  # type: ignore
            except Exception:  # pylint: disable=broad-except
                return result
            if not commit:
                break

            self.commit_list.append(commit)
            result += 1
            if len(commit.author_date()) > self.date_max_len:
                self.date_max_len = len(commit.author_date())
            if len(commit.short_author_name()) > self.name_max_len:
                self.name_max_len = len(commit.short_author_name())
        return result


class HistoryControl(BufferControl):
    def __init__(self, search_buffer_control: SearchBufferControl,
                 key_bindings: Optional[KeyBindings], repo: Repo) -> None:
        buffer = Buffer(name='history')
        self.content = History(repo)
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
        if isinstance(commit, Foldable):
            return commit.is_folded
        return False

    def is_foldable(self, line_number: int) -> bool:
        commit = self.content.commit_list[line_number]
        return isinstance(commit, Foldable)

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
        return isinstance(commit, CommitLink)

    def go_to_link(self, line_number: int):
        commit = self.content.commit_list[line_number]

        if not isinstance(commit, CommitLink):
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
    def __init__(self, key_bindings, repo, right_margins=None):
        search = SearchToolbar(vi_mode=True)
        log_view = HistoryControl(search.control,
                                  key_bindings=key_bindings,
                                  repo=repo)
        window = Window(content=log_view, right_margins=right_margins)
        super().__init__([window, search, STATUS_WINDOW])
