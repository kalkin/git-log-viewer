#!/usr/bin/env python3
"""pygit-viewer

Usage:
    pygit_viewer [--workdir=DIR] [REVISION] [-d | --debug] [[--] <path>...]
    pygit_viewer --version

Options:
    REVISION        A branch, tag or commit [default: HEAD]
    --workdir=DIR   Directory where the git repository is
    -d --debug      Enable sending debuggin output to journalctl
                    (journalctl --user -f)
"""  # pylint: disable=missing-docstring,fixme

import logging
import os
import re
import subprocess
import sys
from typing import Any, List

from docopt import docopt
from prompt_toolkit import Application
from prompt_toolkit.application.current import get_app
from prompt_toolkit.buffer import Buffer
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.key_binding.key_processor import KeyPressEvent
from prompt_toolkit.layout import (BufferControl, HSplit, Layout, UIContent,
                                   Window)
from prompt_toolkit.layout.controls import SearchBufferControl
from prompt_toolkit.layout.margins import ScrollbarMargin
from prompt_toolkit.layout.screen import Point
from prompt_toolkit.output.defaults import get_default_output
from prompt_toolkit.search import SearchState
from prompt_toolkit.widgets import SearchToolbar

from pygit_viewer import (Commit, CommitLink, Foldable, NoPathMatches,
                          NoRevisionMatches, Repo)

ARGUMENTS = docopt(__doc__, version='v0.6.0', options_first=True)
DEBUG = ARGUMENTS['--debug']

LOG = logging.getLogger('pygit-viewer')

if DEBUG:
    LOG.setLevel(logging.DEBUG)
    try:

        def add_journal_handler():
            from systemd.journal import JournalHandler
            journald_handler = JournalHandler()
            # set a formatter to include the level name
            journald_handler.setFormatter(
                logging.Formatter('[%(levelname)s] %(message)s'))

            # add the journald handler to the current logger
            LOG.addHandler(journald_handler)

        add_journal_handler()

        # optionally set the logging level
    except:  # pylint: disable=bare-except
        print("No systemd journal bindings", file=sys.stderr)

# get an instance of the logger object this module will use

KB = KeyBindings()


def highlight(parts, needle):
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


def highlight_substring(search: SearchState, text: tuple) -> list:
    return highlight(text, search.text)


class History(UIContent):
    def __init__(self, repo: Repo) -> None:
        self.date_max_len = 0
        self.name_max_len = 0
        self._repo = repo
        self.line_count = len(list(self._repo.walker()))
        self.commit_list: List[Commit] = []
        self.search_state = None
        super().__init__(
            line_count=self.line_count,
            get_line=self.get_line,
            show_cursor=False)

    def apply_search(self,
                     search_state,
                     include_current_position=True,
                     count=1):
        LOG.debug('appying search %r, %r, %r', search_state,
                  include_current_position, count)
        self.search_state = search_state

    def get_line(self, line_number: int):  # pylint: disable=method-hidden
        length = len(self.commit_list)
        if length - 1 < line_number:
            amount = line_number - length + 1
            self.fill_up(amount)

        try:
            commit = self.commit_list[line_number]
        except IndexError:
            return [("", "")]

        rendered = commit.render()
        _id = rendered.short_id
        author_date = (rendered.author_date[0], rendered.author_date[1].ljust(
            self.date_max_len, " "))
        author_name = (rendered.author_name[0], rendered.author_name[1].ljust(
            self.name_max_len, " "))
        icon = rendered.type
        module = rendered.modules
        subject = rendered.subject
        branches = rendered.branches

        if isinstance(commit, CommitLink):
            if isinstance(subject, tuple):
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
        result = []
        for sth in tmp:
            if isinstance(sth, tuple):
                result += [sth]
            else:
                result += sth

        if branches:
            result += branches

        if line_number == self.cursor_position.y:
            result = [('reverse ' + x[0], x[1]) for x in result]

        result = [(x[0], x[1] + ' ') for x in result]

        return result

    def toggle_fold(self, line_number):
        commit = self.commit_list[line_number]
        if not isinstance(commit, Foldable):
            return

        if commit.is_folded:
            self._unfold(line_number, commit)
        else:
            self._fold(line_number + 1, commit)

    def show_diff(self) -> Any:
        commit = self.commit_list[self.cursor_position.y]
        command = "COLOR=1 vcs-show %s" % commit.oid
        open_in_pager(command)

    def _fold(self, line_number: int, commit: Foldable) -> Any:
        assert not commit.is_folded
        commit.fold()
        for _ in commit.child_log():
            cur_commit = self.commit_list[line_number]
            del self.commit_list[line_number]
            if isinstance(cur_commit, Foldable) and not cur_commit.is_folded:
                self._fold(line_number, cur_commit)
            self.line_count -= 1

    def _unfold(self, line_number: int, commit: Foldable) -> Any:
        assert commit.is_folded
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

    def fill_up(self, amount: int):
        assert amount > 0
        if not self.commit_list:
            self.walker = self._repo.walker()
            try:
                self.commit_list = [next(self.walker)]
            except Exception:
                return
            amount -= 1

            if len(self.commit_list[-1].author_date()) > self.date_max_len:
                self.date_max_len = len(self.commit_list[-1].author_date())
            if len(self.commit_list[-1].
                   short_author_name()) > self.name_max_len:
                self.name_max_len = len(
                    self.commit_list[-1].short_author_name())

        for _ in range(0, amount):
            try:
                commit: Commit = next(self.walker)  # type: ignore
            except Exception:
                return
            if not commit:
                break

            self.commit_list.append(commit)
            if len(commit.author_date()) > self.date_max_len:
                self.date_max_len = len(commit.author_date())
            if len(commit.short_author_name()) > self.name_max_len:
                self.name_max_len = len(commit.short_author_name())


class LogView(BufferControl):
    def __init__(self, search_buffer_control: SearchBufferControl) -> None:
        buffer = Buffer()
        if ARGUMENTS['REVISION'] and ARGUMENTS['REVISION'] != '--':
            revision = ARGUMENTS['REVISION']
        else:
            revision = 'HEAD'
        path = ARGUMENTS['--workdir'] or '.'
        path = os.path.abspath(os.path.expanduser(path))
        self.content = History(Repo(path, revision, ARGUMENTS['<path>']))
        buffer.apply_search = self.content.apply_search  # type: ignore
        super().__init__(
            buffer=buffer, search_buffer_control=search_buffer_control)

    def is_focusable(self) -> bool:
        return True

    @property
    def current_line(self) -> int:
        return self.content.cursor_position.y

    def create_content(self, width, height, preview_search=False):
        return self.content

    def get_key_bindings(self):
        """
        The key bindings that are specific for this user control.

        Return a :class:`.KeyBindings` object if some key bindings are
        specified, or `None` otherwise.
        """
        return KB

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

    @property
    def path(self) -> str:
        return self.path


def screen_height() -> int:
    return get_default_output().from_pty(sys.stdout).get_size().rows


SEARCH = SearchToolbar(vi_mode=True)
try:
    LOG_VIEW = LogView(SEARCH.control)
except NoRevisionMatches:
    print('No revisions match the given arguments.', file=sys.stderr)
    sys.exit(1)
except NoPathMatches:
    print("No paths match the given arguments.", file=sys.stderr)
    sys.exit(1)

MAIN_VIEW = Window(
    content=LOG_VIEW, right_margins=[ScrollbarMargin(display_arrows=True)])
LAYOUT = Layout(HSplit([MAIN_VIEW, SEARCH]), focused_element=MAIN_VIEW)


@KB.add('down')
def down_key(_: KeyPressEvent):
    LOG_VIEW.move_cursor_down()


@KB.add('up')
def up_key(_: KeyPressEvent):
    LOG_VIEW.move_cursor_up()


@KB.add('pagedown')
def pagedown_key(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line + screen_height() * 2 - 1
    LOG_VIEW.goto_line(line_number)


@KB.add('pageup')
def pageup_key(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line - screen_height() * 2 + 1
    if line_number < 0:
        line_number = 0
    LOG_VIEW.goto_line(line_number)


@KB.add('tab')
def tab(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line
    LOG_VIEW.toggle_fold(line_number)
    get_app().invalidate()


@KB.add('enter')
def enter(_: KeyPressEvent):
    LOG_VIEW.content.show_diff()


@KB.add('/')
def search_forward(_: KeyPressEvent):
    LAYOUT.search_links = {SEARCH.control: LOG_VIEW}
    LAYOUT.focus(SEARCH.control)


def open_in_pager(command: str) -> Any:
    term = 'xterm'
    if 'TILIX_ID' in os.environ:
        term = 'tilix'
    elif 'TERM' in os.environ['TERM']:

        term = os.environ['TERM'].split('-')[0]
    cmd = [term, '-e', 'sh', '-c', command]

    subprocess.Popen(cmd, stdin=False, stdout=False, stderr=False)


@KB.add('c-c')
def _(_):
    get_app().exit(result=False)


@KB.add('home')
def first(_):
    LOG_VIEW.goto_line(0)


@KB.add('end')
def last(_):
    LOG_VIEW.goto_last()


def cli():
    app = Application(full_screen=True, layout=LAYOUT)
    app.run()


if __name__ == '__main__':
    cli()
