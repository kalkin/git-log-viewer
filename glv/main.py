#!/usr/bin/env python3
"""glv - Git Log Viewer a TUI application with support for folding merges

Usage:
    glv [--workdir=DIR] [REVISION] [-d | --debug] [[--] <path>...]
    glv --version

Options:
    REVISION        A branch, tag or commit [default: HEAD]
    --workdir=DIR   Directory where the git repository is
    -d --debug      Enable sending debuggin output to journalctl
                    (journalctl --user -f)
"""  # pylint: disable=missing-docstring,fixme,global-statement

import logging
import sys

from docopt import docopt
from prompt_toolkit import Application, shortcuts
from prompt_toolkit.application.current import get_app
from prompt_toolkit.enums import EditingMode
from prompt_toolkit.filters import Condition
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.key_binding.key_processor import KeyPressEvent
from prompt_toolkit.layout import ConditionalContainer, HSplit, Layout
from prompt_toolkit.layout.margins import (ConditionalMargin, Margin,
                                           ScrollbarMargin)
from prompt_toolkit.output.color_depth import ColorDepth
from prompt_toolkit.search import SearchDirection, SearchState
from prompt_toolkit.styles import style_from_pygments_cls
from pygments.style import Style
from pygments.styles.solarized import SolarizedDarkStyle

from glv import NoPathMatches, NoRevisionMatches
from glv.ui.diff_view import DiffView
from glv.ui.history import HistoryContainer
from glv.utils import repo_from_args, screen_height

ARGUMENTS = docopt(__doc__, version='v1.2.0', options_first=True)
DEBUG = ARGUMENTS['--debug']

LOG = logging.getLogger('glv')

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

try:
    REPO = repo_from_args(**ARGUMENTS)
    shortcuts.set_title('%s - Git Log Viewer' % REPO)
except NoRevisionMatches:
    print('No revisions match the given arguments.', file=sys.stderr)
    sys.exit(1)
except NoPathMatches:
    print("No paths match the given arguments.", file=sys.stderr)
    sys.exit(1)


class MyMargin(Margin):
    def get_width(self, get_ui_content) -> int:
        return 1

    def create_margin(self, window_render_info: "WindowRenderInfo", width: int,
                      height: int):
        return [('', ' ')]


WINDOW_VISIBILITY = {
    'history': True,
    'diff': False,
}


@Condition
def diff_visible() -> bool:
    global WINDOW_VISIBILITY
    return WINDOW_VISIBILITY['diff']


MARGINS = [
    ScrollbarMargin(display_arrows=True),
    ConditionalMargin(MyMargin(), filter=diff_visible)
]

MAIN_VIEW = HistoryContainer(KB, REPO, right_margins=MARGINS)
LAYOUT = Layout(HSplit(
    [MAIN_VIEW,
     ConditionalContainer(DiffView(), filter=diff_visible)]),
                focused_element=MAIN_VIEW)


@KB.add('j')
@KB.add('down')
def down_key(_: KeyPressEvent):
    LAYOUT.current_control.move_cursor_down()


@KB.add('k')
@KB.add('up')
def up_key(_: KeyPressEvent):
    LAYOUT.current_control.move_cursor_up()


@KB.add('pagedown')
def pagedown_key(_: KeyPressEvent):
    control = LAYOUT.current_control
    line_number = control.current_line + screen_height() * 2 - 1
    control.goto_line(line_number)


@KB.add('pageup')
def pageup_key(_: KeyPressEvent):
    control = LAYOUT.current_control
    line_number = control.current_line - screen_height() * 2 + 1
    if line_number < 0:
        line_number = 0
    control.goto_line(line_number)


@KB.add('l')
@KB.add('right')
def fold(_: KeyPressEvent):
    control = LAYOUT.current_control
    line_number = control.current_line
    if control.is_link(line_number):
        control.go_to_link(line_number)
    elif control.is_foldable(line_number):
        if control.is_folded(line_number):
            control.toggle_fold(line_number)


@KB.add('h')
@KB.add('left')
def unfold(_: KeyPressEvent):
    control = LAYOUT.current_control
    line_number = control.current_line
    if control.is_foldable(line_number) and \
            not control.is_folded(line_number):
        control.toggle_fold(line_number)
    elif control.is_child(line_number):
        control.go_to_parent(line_number)


@KB.add('tab')
def tab(_: KeyPressEvent):
    control = LAYOUT.current_control
    line_number = control.current_line
    control.toggle_fold(line_number)
    get_app().invalidate()


@KB.add('enter')
def enter(_: KeyPressEvent):
    control = LAYOUT.current_control
    commit = control.current()
    if commit:
        global WINDOW_VISIBILITY
        WINDOW_VISIBILITY['diff'] = True
        buffer = LAYOUT.get_buffer_by_name('diff')
        LAYOUT.focus(buffer)
        LAYOUT.current_control.show_diff(commit)


@KB.add('/')
def search_forward(_: KeyPressEvent):
    control = LAYOUT.current_control
    search_control = control.search_buffer_control
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
        control.content.apply_search(search_state, False)


@KB.add('p')
def search_prev(_: KeyPressEvent):
    control = LAYOUT.current_control
    search = control.search_buffer_control
    search_state = search.searcher_search_state
    if search_state.text:
        search_state.direction = SearchDirection.BACKWARD
        control.content.apply_search(search_state, False)


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
def close(_):
    buffer = LAYOUT.current_buffer
    LOG.debug('closing buffer %s', buffer.name)
    if buffer.name == 'history':
        shortcuts.clear_title()
        get_app().exit(result=False)
    elif buffer.name:
        global WINDOW_VISIBILITY
        WINDOW_VISIBILITY[buffer.name] = False
        LAYOUT.focus(MAIN_VIEW)
    else:
        shortcuts.clear_title()
        get_app().exit(result=False)


@KG.add('c-c', is_global=True)
def _(_):
    shortcuts.clear_title()
    get_app().exit(result=False)


@KG.add('c-l', is_global=True)
def _(_):
    get_app().invalidate()


@KB.add('home')
def first(_):
    control = LAYOUT.current_control
    control.goto_line(0)


@KB.add('end')
def last(_):
    control = LAYOUT.current_control
    control.goto_last()


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
        ('pygments.commit.diffsummary', 'ansiyellow'),
        ('pygments.commit.filename', 'ansiblue'),
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
    app.run()
    shortcuts.clear_title()


if __name__ == '__main__':
    cli()
