#!/usr/bin/env python3

# pylint: disable=missing-docstring,fixme

import os

from prompt_toolkit.application import Application
from prompt_toolkit.application.current import get_app
from prompt_toolkit.buffer import Buffer
from prompt_toolkit.document import Document
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout.containers import Container, HSplit, Window
from prompt_toolkit.layout.controls import BufferControl
from prompt_toolkit.layout.layout import Layout
from prompt_toolkit.layout.margins import ScrollbarMargin
from pygit_viewer import Commit, Foldable, InitialCommit, LastCommit, Repo

HISTORY: list = []


class LogPager(object):
    """
    A simple input field.

    This contains a ``prompt_toolkit`` :class:`~prompt_toolkit.buffer.Buffer`
    object that hold the text data structure for the edited buffer, the
    :class:`~prompt_toolkit.layout.BufferControl`, which applies a
    :class:`~prompt_toolkit.lexers.Lexer` to the text and turns it into a
    :class:`~prompt_toolkit.layout.UIControl`, and finally, this
    :class:`~prompt_toolkit.layout.UIControl` is wrapped in a
    :class:`~prompt_toolkit.layout.Window` object (just like any
    :class:`~prompt_toolkit.layout.UIControl`), which is responsible for the
    scrolling.

    This widget does have some options, but it does not intend to cover every
    single use case. For more configurations options, you can always build a
    text area manually, using a :class:`~prompt_toolkit.buffer.Buffer`,
    :class:`~prompt_toolkit.layout.BufferControl` and
    :class:`~prompt_toolkit.layout.Window`.

    :param text: The initial text.
    :param width: Window width. (:class:`~prompt_toolkit.layout.Dimension` object.)
    :param height: Window height. (:class:`~prompt_toolkit.layout.Dimension` object.)
    :param style: A style string.
    """

    def __init__(self, text='', style=''):
        assert isinstance(text, str)

        self.buffer = Buffer(document=Document(text, 0), read_only=True)

        self.control = BufferControl(
            buffer=self.buffer, lexer=None, focusable=True)

        right_margins = [ScrollbarMargin(display_arrows=True)]
        style = 'class:text-area ' + style

        self.window = Window(
            dont_extend_height=False,
            dont_extend_width=False,
            content=self.control,
            style=style,
            wrap_lines=False,
            right_margins=right_margins)

    @property
    def text(self):
        return self.buffer.text

    @text.setter
    def text(self, value):
        self.buffer.set_document(Document(value, 0), bypass_readonly=True)

    @property
    def document(self):
        return self.buffer.document

    @document.setter
    def document(self, value):
        self.buffer.document = value

    def __pt_container__(self):
        return self.window


TEXTFIELD = LogPager()
# Global key bindings.
BINDINGS = KeyBindings()

ROOT_CONTAINER: Container = HSplit([
    TEXTFIELD,
])

APPLICATION = Application(
    layout=Layout(ROOT_CONTAINER, ),
    key_bindings=BINDINGS,
    mouse_support=True,
    full_screen=True)

REPO = Repo(os.getcwd())


def commit_type(line: Commit) -> str:
    ''' Helper method for displaying commit type.  '''
    # TODO Add support for ocotopus branch display
    if line.noffff:
        return "…… "
    if isinstance(line, Foldable):
        return foldable_type(line)
    elif isinstance(line, InitialCommit):
        return "◉  "
    elif isinstance(line, LastCommit):
        return "✂  "

    if isinstance(line.parent, Foldable) \
        and line.oid != line.parent.raw_commit.parents[1].id \
        and REPO.is_connected(line, 1):
        return "●─╯"
    return "●  "


def foldable_type(line: Foldable) -> str:
    if isinstance(line.parent, Foldable) \
    and line.oid == line.parent.raw_commit.parents[0].id:
        return "●─┤"

    return "●─╮"


@BINDINGS.add('c-c')
def _(_):
    get_app().exit(result=False)


def format_commit(line: Commit) -> str:
    return " ".join([commit_type(line), str(line)])


def current_row(textarea: LogPager) -> int:
    document: Document = textarea.document
    return document.cursor_position_row


def current_line(pos: int) -> Commit:
    return HISTORY[pos]


def open_in_pager(command: str):
    os.system('xterm -e "%s|LESS="-R" $PAGER"' % command)


def show_diff(commit: Commit):
    command = "COLOR=1 vcs-show %s" % commit.oid
    open_in_pager(command)


@BINDINGS.add('tab')
def toggle_fold(_):
    row = current_row(TEXTFIELD)
    line: Commit = current_line(row)
    point = TEXTFIELD.buffer.cursor_position
    if isinstance(line, Foldable):
        if line.is_folded:
            fold_open(line, row)
        else:
            fold_close(line, row)

    TEXTFIELD.buffer.cursor_position = point


@BINDINGS.add('enter')
def open_diff(_):
    row = current_row(TEXTFIELD)
    commit: Commit = current_line(row)
    show_diff(commit)


def fold_close(line: Foldable, index: int):
    lines = TEXTFIELD.text.splitlines()
    line.fold()
    level = line.level
    index += 1
    for commit in HISTORY[index:]:
        if commit.level <= level:
            break
        del lines[index]
        del HISTORY[index]
    TEXTFIELD.text = "\n".join(lines)


def fold_open(start: Foldable, index: int):
    lines = TEXTFIELD.text.splitlines()
    start.unfold()
    index += 1
    for commit in start.child_log():
        level = commit.level * '│ '
        HISTORY.insert(index, commit)
        msg = level + format_commit(commit)
        lines.insert(index, msg)
        index += 1
    TEXTFIELD.text = "\n".join(lines)


def cli():
    i = 0
    for commit in REPO.walker():
        i += 1
        msg = format_commit(commit)
        HISTORY.append(commit)
        TEXTFIELD.text += msg + "\n"
        if i > 100:
            break
    APPLICATION.run()


if __name__ == '__main__':
    cli()
