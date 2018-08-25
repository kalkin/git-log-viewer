#!/usr/bin/env python3

import os

from prompt_toolkit.application import Application
from prompt_toolkit.application.current import get_app
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.key_binding.bindings.focus import (focus_next,
                                                       focus_previous)
from prompt_toolkit.layout.containers import Float, HSplit, VSplit
from prompt_toolkit.layout.dimension import D
from prompt_toolkit.layout.layout import Layout
from prompt_toolkit.layout.menus import CompletionsMenu
from prompt_toolkit.lexers import PygmentsLexer
from prompt_toolkit.styles import Style
from prompt_toolkit.widgets import (Box, Button, Checkbox, Dialog, Frame,
                                    Label, MenuContainer, MenuItem,
                                    ProgressBar, RadioList, TextArea)
# pylint: disable=no-name-in-module
from pygit2 import Commit, Repository, discover_repository
from pygments.lexers.html import HtmlLexer

GIT_DIR = discover_repository(os.getcwd())
REPO = Repository(GIT_DIR)
ROOT = REPO.revparse_single("HEAD")

textfield = TextArea(lexer=PygmentsLexer(HtmlLexer), read_only=True)
# Global key bindings.
bindings = KeyBindings()


@bindings.add('c-c')
def _(event):
    get_app().exit(result=False)

FIRST = True

@bindings.add('c-a')
def foo(event):
    global ROOT
    global FIRST
    if not FIRST:
        ROOT = ROOT.parents[0]
        msg = ROOT.message.strip().splitlines()[0]
        textfield.text += "\n" + msg
    else:
        msg = ROOT.message.strip().splitlines()[0]
        textfield.text = msg
        FIRST = False



root_container = HSplit([
    textfield,
])

application = Application(
    layout=Layout(root_container, ),
    key_bindings=bindings,
    mouse_support=True,
    full_screen=True)


def run():
    result = application.run()
    print('You said: %r' % result)


if __name__ == '__main__':
    run()
