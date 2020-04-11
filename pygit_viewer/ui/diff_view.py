''' Diff View '''
import datetime

from prompt_toolkit.buffer import Buffer
from prompt_toolkit.document import Document
from prompt_toolkit.filters import Condition
from prompt_toolkit.layout import (BufferControl, ConditionalContainer, HSplit,
                                   Window)
from prompt_toolkit.layout.controls import SearchBufferControl
from prompt_toolkit.layout.dimension import Dimension
from prompt_toolkit.layout.margins import ScrollbarMargin
from prompt_toolkit.widgets import Frame, SearchToolbar
from pygit2 import Diff  # pylint: disable=no-name-in-module
from pygit2 import GIT_DIFF_STATS_FULL  # pylint: disable=no-name-in-module
from pygit2 import Signature  # pylint: disable=no-name-in-module
from pygit_viewer.utils import screen_width

from pygit_viewer.lexer import COMMIT_LEXER

LOCAL_TZ = datetime.datetime.now(datetime.timezone(
    datetime.timedelta(0))).astimezone().tzinfo


class DiffDocument(Document):
    '''
        A wrapper around `Document` which provides own paragraph definition, for
        easy jumping with vi bindings '{' & '}'
    '''
    def start_of_paragraph(self, count: int = 1, before: bool = False) -> int:
        """
        Return the start of the current paragraph. (Relative cursor position.)
        """ # pylint: disable=invalid-unary-operand-type

        def match_func(text: str) -> bool:
            return text.startswith('@@')

        line_index = self.find_previous_matching_line(match_func=match_func,
                                                      count=count)

        if line_index:
            add = 0 if before else 1
            return min(0, self.get_cursor_up_position(count=-line_index) + add)

        return -self.cursor_position

    def end_of_paragraph(self, count: int = 1, after: bool = False) -> int:
        """
        Return the end of the current paragraph. (Relative cursor position.)
        """
        def match_func(text: str) -> bool:
            return text.startswith('@@')

        line_index = self.find_next_matching_line(match_func=match_func,
                                                  count=count)
        if line_index:
            add = 0 if after else 1
            return max(0,
                       self.get_cursor_down_position(count=line_index) - add)

        return len(self.text_after_cursor)


Document.start_of_paragraph = DiffDocument.start_of_paragraph
Document.end_of_paragraph = DiffDocument.end_of_paragraph


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
    def __init__(self):
        self._visible = False

        @Condition
        def is_visible() -> bool:
            return self._visible

        buffer = Buffer(read_only=True)
        self._search = SearchToolbar(vi_mode=True)
        self.control = DiffControl(buffer, self._search.control)
        body = HSplit([
            Window(self.control,
                   right_margins=[ScrollbarMargin(display_arrows=True)]),
            self._search
        ])
        super().__init__(Frame(body), is_visible)

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
        if refs:
            text += "Refs:       %s\n" % ", ".join(refs)

        if committer.name != author.name:
            text += "Committer:     %s\n" % self.name_from_signature(committer)
        if committer.time != author.time:
            text += "CommitDate: %s\n" % self.date_from_signature(committer)
        text += "\n"
        # pylint: disable=protected-access
        text += commit._commit.message
        text += "\n---\n\n"
        text += diff.stats.format(GIT_DIFF_STATS_FULL, screen_width() - 10)
        text += "\n"
        text += "\n\n".join([p.text for p in diff])
        doc = DiffDocument(text, cursor_position=0)

        self.control.buffer.set_document(doc, bypass_readonly=True)
        self._visible = True

    def is_visible(self) -> bool:
        ''' Return true if visible '''
        return self._visible

    def hide(self):
        ''' Hide the view '''
        self._visible = False

    def preferred_height(self, width: int,
                         max_available_height: int) -> Dimension:
        dim = super().preferred_height(width, max_available_height)
        if self._visible:
            dim.preferred = max_available_height / 2
            dim.max = max_available_height / 2
        return dim
