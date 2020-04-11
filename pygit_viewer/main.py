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
import sys

from docopt import docopt
from prompt_toolkit import Application
from prompt_toolkit.application.current import get_app
from prompt_toolkit.enums import EditingMode
from prompt_toolkit.filters import Condition
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.key_binding.key_processor import KeyPressEvent
from prompt_toolkit.layout import ConditionalContainer, HSplit, Layout, Window
from prompt_toolkit.layout.margins import (ConditionalMargin, Margin,
                                           ScrollbarMargin)
from prompt_toolkit.output.color_depth import ColorDepth
from prompt_toolkit.search import SearchDirection, SearchState
from prompt_toolkit.styles import style_from_pygments_cls
from prompt_toolkit.widgets import SearchToolbar
from pygments.style import Style
from pygments.styles.solarized import SolarizedDarkStyle

from pygit_viewer import NoPathMatches, NoRevisionMatches
from pygit_viewer.ui.diff_view import DiffView
from pygit_viewer.ui.log import LogView
from pygit_viewer.ui.status import STATUS
from pygit_viewer.utils import repo_from_args, screen_height

ARGUMENTS = docopt(__doc__, version='v1.0.0', options_first=True)
DEBUG = ARGUMENTS['--debug']

LOG = logging.getLogger('pygit-viewer')

LOG.setLevel(logging.CRITICAL)
if DEBUG:
    LOG.setLevel(logging.DEBUG)
    try:

        def add_journal_handler():
            from systemd.journal import JournalHandler  # pylint: disable=import-outside-toplevel
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
        sys.exit(1)

# get an instance of the logger object this module will use

KB = KeyBindings()
KG = KeyBindings()

SEARCH = SearchToolbar(vi_mode=True)
try:
    REPO = repo_from_args(**ARGUMENTS)
    LOG_VIEW = LogView(SEARCH.control, key_bindings=KB, repo=REPO)
except NoRevisionMatches:
    print('No revisions match the given arguments.', file=sys.stderr)
    sys.exit(1)
except NoPathMatches:
    print("No paths match the given arguments.", file=sys.stderr)
    sys.exit(1)


@Condition
def statis_is_visible() -> bool:
    return bool(STATUS.content.text)


class MyMargin(Margin):
    def get_width(self, get_ui_content) -> int:
        return 1

    def create_margin(self, window_render_info: "WindowRenderInfo", width: int,
                      height: int):
        return [('', ' ')]


STATUS_WINDOW = ConditionalContainer(content=Window(content=STATUS,
                                                    height=1,
                                                    ignore_content_height=True,
                                                    wrap_lines=False),
                                     filter=statis_is_visible)
DIFF_VIEW = DiffView()


@Condition
def diff_visible() -> bool:
    return DIFF_VIEW.is_visible()


MAIN_VIEW = HSplit([
    Window(content=LOG_VIEW,
           right_margins=[
               ScrollbarMargin(display_arrows=True),
               ConditionalMargin(MyMargin(), filter=diff_visible)
           ]), SEARCH, STATUS_WINDOW
])
LAYOUT = Layout(HSplit([MAIN_VIEW, DIFF_VIEW]), focused_element=MAIN_VIEW)


@KB.add('j')
@KB.add('down')
def down_key(_: KeyPressEvent):
    LOG_VIEW.move_cursor_down()
    update_commit_bar()


@KB.add('k')
@KB.add('up')
def up_key(_: KeyPressEvent):
    LOG_VIEW.move_cursor_up()
    update_commit_bar()


@KB.add('pagedown')
def pagedown_key(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line + screen_height() * 2 - 1
    LOG_VIEW.goto_line(line_number)
    update_commit_bar()


@KB.add('pageup')
def pageup_key(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line - screen_height() * 2 + 1
    if line_number < 0:
        line_number = 0
    LOG_VIEW.goto_line(line_number)
    update_commit_bar()


@KB.add('l')
@KB.add('right')
def fold(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line
    if LOG_VIEW.is_link(line_number):
        LOG.debug("DRIN")
        LOG_VIEW.go_to_link(line_number)
        update_commit_bar()
    elif LOG_VIEW.is_foldable(line_number):
        if LOG_VIEW.is_folded(line_number):
            LOG_VIEW.toggle_fold(line_number)


@KB.add('h')
@KB.add('left')
def unfold(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line
    if LOG_VIEW.is_foldable(line_number) and \
            not LOG_VIEW.is_folded(line_number):
        LOG_VIEW.toggle_fold(line_number)
    elif LOG_VIEW.is_child(line_number):
        LOG_VIEW.go_to_parent(line_number)
        update_commit_bar()


@KB.add('tab')
def tab(_: KeyPressEvent):
    line_number = LOG_VIEW.current_line
    LOG_VIEW.toggle_fold(line_number)
    get_app().invalidate()


@KB.add('enter')
def enter(_: KeyPressEvent):
    commit = LOG_VIEW.current()
    if commit:
        DIFF_VIEW.show_diff(commit)
        LAYOUT.focus(DIFF_VIEW)


@KB.add('/')
def search_forward(_: KeyPressEvent):
    control = LAYOUT.current_control
    search_control = LOG_VIEW.search_buffer_control
    LAYOUT.search_links = {search_control: control}
    search_state = SearchState(direction=SearchDirection.FORWARD,
                               ignore_case=False)
    search_control.searcher_search_state = search_state
    LAYOUT.focus(search_control)


@KB.add('n')
def search_next(_: KeyPressEvent):
    control = LAYOUT.current_control
    search = control.search_buffer_control
    search_state = search.searcher_search_state
    if search_state.text:
        search_state.direction = SearchDirection.FORWARD
        LOG_VIEW.content.apply_search(search_state, False)
        update_commit_bar()


@KB.add('p')
def search_prev(_: KeyPressEvent):
    control = LAYOUT.current_control
    search = control.search_buffer_control
    search_state = search.searcher_search_state
    if search_state.text:
        search_state.direction = SearchDirection.BACKWARD
        LOG_VIEW.content.apply_search(search_state, False)
        update_commit_bar()


@KB.add('?')
def search_backward(_: KeyPressEvent):
    control = LAYOUT.current_control
    search_control = control.search_buffer_control
    LAYOUT.search_links = {search_control: control}
    search_state = SearchState(direction=SearchDirection.BACKWARD,
                               ignore_case=False)
    search_control.searcher_search_state = search_state
    LAYOUT.focus(search_control)


@KG.add('q', is_global=True, eager=True)
def qkb(_):
    LOG.debug('Hidding DIFF_VIEW')
    DIFF_VIEW.hide()
    LAYOUT.focus(LOG_VIEW)


@KG.add('c-c', is_global=True)
def _(_):
    get_app().exit(result=False)


@KG.add('c-l', is_global=True)
def _(_):
    get_app().invalidate()


@KB.add('home')
def first(_):
    LOG_VIEW.goto_line(0)
    update_commit_bar()


@KB.add('end')
def last(_):
    LOG_VIEW.goto_last()
    update_commit_bar()


def update_commit_bar() -> None:
    pass


def patched_style() -> Style:
    ''' Our patched solarized style.
    '''  # pylint: disable=protected-access
    style = style_from_pygments_cls(SolarizedDarkStyle)
    for i in range(len(style._style_rules)):
        tpl = style._style_rules[i]
        if tpl[0] == 'pygments.generic.heading':
            style._style_rules[i] = (tpl[0], 'nobold #b58900')
        if tpl[0] == 'pygments.generic.subheading':
            style._style_rules[i] = (tpl[0], 'nobold #d33682')

    style._style_rules += [
        ('pygments.commit', 'noinherit'),
        ('pygments.commit.author', 'ansigreen'),
        ('pygments.commit.authordate', 'ansiblue'),
        ('pygments.commit.id', 'ansimagenta'),
        ('pygments.commit.committer', 'italic ansigreen'),
        ('pygments.commit.commitdate', 'italic ansiblue'),
        ('pygments.commit.refs', 'ansiyellow'),
        ('pygments.commit.modules', 'ansiyellow'),
        ('pygments.commit.end', 'bold'),
    ]
    return style


def cli():
    app = Application(full_screen=True,
                      layout=LAYOUT,
                      style=patched_style(),
                      color_depth=ColorDepth.TRUE_COLOR,
                      key_bindings=KG)
    app.editing_mode = EditingMode.VI
    update_commit_bar()
    app.run()


if __name__ == '__main__':
    cli()
