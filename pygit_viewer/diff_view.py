''' Diff View '''

import datetime

from prompt_toolkit.buffer import Buffer
from prompt_toolkit.document import Document
from prompt_toolkit.filters import Condition
from prompt_toolkit.layout import BufferControl, ConditionalContainer, Window
from prompt_toolkit.layout.controls import SearchBufferControl
from prompt_toolkit.layout.dimension import Dimension
from pygit2 import Diff  # pylint: disable=no-name-in-module
from pygit2 import Signature  # pylint: disable=no-name-in-module
from pygit_viewer.lexer import COMMIT_LEXER

LOCAL_TZ = datetime.datetime.now(datetime.timezone(
    datetime.timedelta(0))).astimezone().tzinfo


class DiffControl(BufferControl):
    ''' Controll for the diff buffer '''
    def __init__(self, buffer: Buffer, search: SearchBufferControl):

        super().__init__(buffer=buffer,
                         lexer=COMMIT_LEXER,
                         focusable=True,
                         focus_on_click=True,
                         key_bindings=None,
                         search_buffer_control=search)


class DiffView(ConditionalContainer):
    ''' Represents the hideable view for diffs which provides a read only
        buffer.
    '''
    def __init__(self, search: SearchBufferControl):
        self._visible: bool = False

        @Condition
        def is_visible() -> bool:
            return self._visible

        buffer = Buffer(read_only=True)
        self.control = DiffControl(buffer, search)
        super().__init__(Window(self.control), is_visible)

    @staticmethod
    def name_from_signature(sign: Signature) -> str:
        ''' Returns: Author Name <email> '''
        return "%s <%s>" % (sign.name, sign.email)

    @staticmethod
    def date_from_signature(sign: Signature) -> str:
        ''' Returns date formatted to current local and timezone'''
        date = datetime.datetime.fromtimestamp(sign.time, LOCAL_TZ)
        return date.strftime('%c')

    def show_diff(self, commit):
        ''' Command diff view to show a diff '''
        diff: Diff = commit.diff()
        if diff is None:
            raise ValueError('Got None instead of a Diff')
        text = ""
        author: Signature = commit.author_signature
        committer: Signature = commit.committer_signature

        text += "Commit:     %s\n" % commit.raw_commit.oid
        text += "Author:     %s\n" % self.name_from_signature(author)
        text += "AuthorDate: %s\n" % self.date_from_signature(author)
        if commit.modules():
            text += "Modules:    %s\n" % commit.modules()
        refs = ["«%s»" % name for name in commit.branches]
        if committer.name != author.name:
            text += "Committer:     %s\n" % self.name_from_signature(committer)
        if committer.time != author.time:
            text += "CommitDate: %s\n" % self.date_from_signature(committer)
        text += "\n"
        # pylint: disable=protected-access
        text += commit._commit.message
        text += "\n---\n\n"
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
