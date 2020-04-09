''' Diff View '''

from prompt_toolkit.buffer import Buffer
from prompt_toolkit.document import Document
from prompt_toolkit.filters import Condition
from prompt_toolkit.layout import BufferControl, ConditionalContainer, Window
from prompt_toolkit.lexers import PygmentsLexer
from pygit2 import Diff  # pylint: disable=no-name-in-module
from pygments.lexers.diff import DiffLexer


class DiffControl(BufferControl):
    ''' Controll for the diff buffer '''
    def __init__(self, buffer: Buffer):
        lexer = PygmentsLexer(DiffLexer)
        super().__init__(
            buffer=buffer,
            lexer=lexer,
            focusable=True,
            focus_on_click=True,
            key_bindings=None,
        )


class DiffView(ConditionalContainer):
    ''' Represents the hideable view for diffs which provides a read only
        buffer.
    '''
    def __init__(self):
        self._visible: bool = False

        @Condition
        def is_visible() -> bool:
            return self._visible

        buffer = Buffer(read_only=True)
        self.control = DiffControl(buffer)
        super().__init__(Window(self.control), is_visible)

    def show_diff(self, commit):
        ''' Command diff view to show a diff '''
        diff: Diff = commit.diff()
        if diff is None:
            raise ValueError('Got None instead of a Diff')
        text = ""
        # pylint: disable=protected-access
        text += "Commit:     %s\n" % commit._commit.oid
        text += "Author:     %s\n" % commit.short_author_name()
        text += "AuthorDate: %s ago\n" % commit.author_date()
        refs = ["«%s»" % name for name in commit.branches]
        if refs:
            text += "Refs:       %s\n" % ", ".join(refs)
        text += "\n"
        # pylint: disable=protected-access
        text += commit._commit.message
        text += "---\n"
        text += "\n\n"
        text += "\n\n".join([p.text for p in diff])
        doc = Document(text, cursor_position=0)

        self.control.buffer.set_document(doc, bypass_readonly=True)
        self._visible = True

    def hide(self):
        ''' Hide the view '''
        self._visible = False

    def preferred_height(self, width: int,
                         max_available_height: int) -> Dimension:
        dim = super().preferred_height(width, max_available_height)
        if self._visible:
            dim.preferred = max_available_height
        return dim
