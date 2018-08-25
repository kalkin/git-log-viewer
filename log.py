#!/usr/bin/env python3

# pylint: disable=missing-docstring

import os

from prompt_toolkit.application import Application
from prompt_toolkit.application.current import get_app
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout.containers import HSplit
from prompt_toolkit.layout.layout import Layout
from prompt_toolkit.widgets import TextArea
# pylint: disable=no-name-in-module
from pygit2 import Repository, discover_repository

GIT_DIR = discover_repository(os.getcwd())
REPO = Repository(GIT_DIR)
ROOT = REPO.revparse_single("HEAD")

TEXTFIELD = TextArea(read_only=True)
# Global key bindings.
bindings = KeyBindings()


@bindings.add('c-c')
def _(_):
    get_app().exit(result=False)


FIRST = True


@bindings.add('c-a')
def foo(_):
    global ROOT
    global FIRST
    if not FIRST:
        ROOT = ROOT.parents[0]
        msg = ROOT.message.strip().splitlines()[0]
        TEXTFIELD.text += "\n" + msg
    else:
        msg = ROOT.message.strip().splitlines()[0]
        TEXTFIELD.text = msg
        FIRST = False


ROOT_CONTAINER = HSplit([
    TEXTFIELD,
])

APPLICATION = Application(
    layout=Layout(ROOT_CONTAINER, ),
    key_bindings=bindings,
    mouse_support=True,
    full_screen=True)


def run():
    result = APPLICATION.run()
    print('You said: %r' % result)


if __name__ == '__main__':
    run()
