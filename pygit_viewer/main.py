#!/usr/bin/env python3

# pylint: disable=missing-docstring,fixme

import os
from datetime import datetime

import babel.dates
from prompt_toolkit.application import Application
from prompt_toolkit.application.current import get_app
from prompt_toolkit.document import Document
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout.containers import HSplit
from prompt_toolkit.layout.layout import Layout
from prompt_toolkit.widgets import TextArea
# pylint: disable=no-name-in-module
from pygit2 import Commit, Repository, discover_repository

GIT_DIR = discover_repository(os.getcwd())
REPO = Repository(GIT_DIR)
ROOT = REPO.revparse_single("HEAD")
HISTORY = []

TEXTFIELD = TextArea(read_only=True, wrap_lines=False)
COMMIT_MAP = {}
# Global key bindings.
BINDINGS = KeyBindings()

ROOT_CONTAINER = HSplit([
    TEXTFIELD,
])

APPLICATION = Application(
    layout=Layout(ROOT_CONTAINER, ),
    key_bindings=BINDINGS,
    mouse_support=True,
    full_screen=True)


def commit_type(commit: Commit) -> str:
    ''' Helper method for displaying commit type.

        Currently there are three types of commits:
        - Initial commit (no parents)
        - Normal commit (1 parent)
        - Merge commit (> 1 parent)
    '''
    if not commit.parents:
        return "◎  "
    elif len(commit.parents) == 1:
        return "●  "
    # TODO Add support for ocotopus branch display
    return "●─╮"


def relative_date(commit: Commit) -> str:
    ''' Translates a unique timestamp to a relative and short date string '''
    # pylint: disable=invalid-name
    timestamp: int = commit.committer.time
    t = timestamp
    delta = datetime.now() - datetime.fromtimestamp(t)
    return babel.dates.format_timedelta(delta, format='short').strip('.')


@BINDINGS.add('c-c')
def _(_):
    get_app().exit(result=False)


def format_commit(commit: Commit) -> str:
    hash_id = str(commit.id)[0:7] + " "
    rel_date: str = relative_date(commit)
    author: str = commit.committer.name + " <" + commit.committer.email + ">"
    _type: str = commit_type(commit)
    subject: str = commit.message.strip().splitlines()[0]
    return " ".join([_type, hash_id, rel_date, author.split()[0], subject])


def current_row(textarea: TextArea) -> int:
    document: Document = textarea.document
    return document.cursor_position_row


def current_commit(pos: int) -> Commit:
    return HISTORY[pos]


@BINDINGS.add('enter')
def open_fold(_):
    row = current_row(TEXTFIELD)
    commit = current_commit(row)
    point = TEXTFIELD.buffer.cursor_position
    if len(commit.parents) >= 2 and commit.parents[1].id in COMMIT_MAP:
        fold_close(commit, row)
    elif len(commit.parents) >= 2:
        fold_open(commit, row)

    TEXTFIELD.buffer.cursor_position = point


def fold_close(commit: Commit, index: int):
    lines = TEXTFIELD.text.splitlines()
    del lines[index + 1]
    del HISTORY[index + 1]
    del COMMIT_MAP[commit.parents[1].id]
    TEXTFIELD.text = "\n".join(lines)


def fold_open(commit: Commit, index: int):
    msg = "  " + format_commit(commit.parents[1])
    lines = TEXTFIELD.text.splitlines()
    lines.insert(index + 1, msg)
    HISTORY.insert(index + 1, commit.parents[1])
    COMMIT_MAP[commit.parents[1].id] = commit.parents[1]
    TEXTFIELD.text = "\n".join(lines)


def cli():
    commit = ROOT
    try:
        while commit.parents:
            HISTORY.append(commit)
            COMMIT_MAP[commit.id] = commit
            msg = format_commit(commit)
            TEXTFIELD.text += msg + "\n"
            commit = commit.parents[0]
    except KeyError:
        pass

    result = APPLICATION.run()
    print('You said: %r' % result)


if __name__ == '__main__':
    cli()
