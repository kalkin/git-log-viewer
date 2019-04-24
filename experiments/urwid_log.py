#!/usr/bin/env python3
""" A tig replacement
"""

import os
import sys
from datetime import datetime
from pathlib import Path

import babel.dates
import urwid
import urwidtrees
# pylint: disable=no-name-in-module
from pygit2 import Commit, Repository, discover_repository

HOME = str(Path.home())

GIT_DIR = discover_repository(os.getcwd())
REPO = Repository(GIT_DIR)

# define some colours
PALETTE = [
    ('body', 'default', 'default'),
    ('focus', 'white', 'dark green', 'bold'),
    ('hash', 'dark magenta', 'default'),
    ('type', 'dark blue', 'default'),
    ('subject', 'default', 'default'),
    ('commiter', 'dark green', 'default'),
    ('footer', 'white', 'dark blue', 'bold'),
    ('bars', 'dark blue', 'light gray', ''),
    ('arrowtip', 'light blue', 'light gray', ''),
    ('connectors', 'light red', 'light gray', ''),
]

MAILMAP_FILE = HOME + "/.config/git/mailmap"


def relative_date(t: int) -> str:
    ''' Translates a uniq timestamp to a relative and short date string '''
    # pylint: disable=invalid-name
    delta = datetime.now() - datetime.fromtimestamp(t)
    return babel.dates.format_timedelta(delta, format='short').strip('.')


def replace_name(email: str, name: str = None) -> str:
    """ Returns a string usable as author identification for display """
    return name + " <" + email + ">"


class CommitRowWidget(urwid.WidgetWrap):
    """Widget to display a commit as a row"""

    # pylint: disable=invalid-name

    def __init__(self, c: Commit):
        row = urwid.Columns([
            CommitRowWidget._hash_widget(c),
            CommitRowWidget._date(c),
            self._commiter(c),
            CommitRowWidget._type(c),
            CommitRowWidget._subject_widget(c)
        ])
        w = urwid.AttrMap(row, None, 'focus')
        w.set_focus_map({
            'hash': 'focus',
            'subject': 'focus',
            'type': 'focus',
            'commiter': 'focus'
        })
        urwid.WidgetWrap.__init__(self, w)

    def _commiter(self, c: Commit):
        signature = c.committer
        msg = replace_name(signature.email, signature.name)
        return ('pack', urwid.Text(('commiter', " " + msg + " "),
                                   urwid.CENTER))

    @staticmethod
    def _date(c: Commit):
        timestamp: int = c.committer.time
        msg: str = relative_date(timestamp)
        return (6, urwid.Text(('type', msg)))

    @staticmethod
    def _hash_widget(c: Commit):
        ''' Helper method for decorating commit id. '''
        hash_id = str(c.id)[0:7] + " "
        return ('pack', urwid.Text(('hash', hash_id)))

    @staticmethod
    def _subject_widget(c: Commit):
        ''' Helper method for decorating commit message. '''
        msg = c.message.strip().splitlines()[0]
        return urwid.Text(('subject', msg))

    @staticmethod
    def _type(c: Commit) -> str:
        ''' Helper method for displaying commit type.

            Currently there are three types of commits:
            - Initial commit (no parents)
            - Normal commit (1 parent)
            - Merge commit (> 1 parent)
        '''
        if not c.parents:
            msg = "◎  "
        elif len(c.parents) == 1:
            msg = "●  "
        else:
            msg = "●━ "

        return ('pack', urwid.Text(('type', msg)))

    def selectable(self):
        return True

    def keypress(self, _, key):
        return key


class CommitTree(urwidtrees.tree.Tree):
    """ A custom Tree representing our commit history structure. """

    root = REPO.revparse_single("HEAD")

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.cache = {}

    def first_child_position(self, pos):
        return None

    def last_child_position(self, pos):
        return None

    def next_sibling_position(self, pos):
        try:
            result = pos.parents[0]
            self.cache[result.id] = pos
        except IndexError:
            result = None
        return result

    def prev_sibling_position(self, pos):
        if pos.id in self.cache:
            result = self.cache[pos.id]
        else:
            result = None
        return result

    def __getitem__(self, pos):
        return CommitRowWidget(pos)


def unhandled_input(k):  # pylint: disable=missing-docstring
    """ Keyboard handling """
    print(k)
    if k in ['q', 'Q']:
        raise urwid.ExitMainLoop()


def main() -> int:  # pylint: disable=missing-docstring
    tree_widget = urwidtrees.widgets.TreeBox(
        urwidtrees.decoration.ArrowTree(CommitTree()))
    footer = urwid.AttrMap(urwid.Text('Q to quit'), 'footer')
    frame = urwid.Frame(tree_widget, footer=footer)

    urwid.MainLoop(frame, palette=PALETTE).run()

    return 0


if __name__ == "__main__":
    exit_code: int = main()
    REPO.free()
    sys.exit(exit_code)
