#
# Copyright (c) 2018-2020 Bahtiar `kalkin-` Gadimov.
#
# This file is part of Git Log Viewer
# (see https://github.com/kalkin/git-log-viewer).
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.
#
''' Diff View '''
import datetime
from typing import Optional

from prompt_toolkit.buffer import Buffer
from prompt_toolkit.document import Document
from prompt_toolkit.formatted_text import AnyFormattedText
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout import AnyDimension, BufferControl, HSplit, Window
from prompt_toolkit.layout.controls import SearchBufferControl
from prompt_toolkit.layout.margins import ScrollbarMargin
from prompt_toolkit.widgets import Frame, SearchToolbar
from pygit2 import GIT_DIFF_STATS_FULL  # pylint: disable=no-name-in-module
from pygit2 import Diff  # pylint: disable=no-name-in-module
from pygit2 import Signature  # pylint: disable=no-name-in-module

from glv import Commit, vcs
from glv.lexer import COMMIT_LEXER
from glv.utils import screen_height, screen_width

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

    @staticmethod
    def _render_body(diff: Diff) -> Optional[str]:
        '''
            Renders diff stats and diff patches.

            May fail if local repository is missing objects, will return None on
            error.
        '''
        try:
            text = ''
            text += diff.stats.format(GIT_DIFF_STATS_FULL, screen_width() - 10)
            text += "\n"
            text += "\n\n".join([p.text for p in diff])
            return text
        except Exception:  # pylint: disable=broad-except
            return None

    def show_diff(self, commit: Commit):
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
            text += "Modules:    %s\n" % ', '.join(commit.modules())

        refs = ["«%s»" % name for name in commit.branches]
        if refs:
            text += "Refs:       %s\n" % ", ".join(refs)

        if committer.name != author.name:
            text += "Committer:     %s\n" % self.name_from_signature(committer)
        if committer.time != author.time:
            text += "CommitDate: %s\n" % self.date_from_signature(committer)
        # pylint: disable=protected-access
        text += "\n"
        body_lines = commit._commit.message.replace('\r', '').split("\n")
        text += " " + body_lines[0] + "\n"
        body_lines = body_lines[1:]
        if body_lines[0] == '' and len(body_lines) == 1:
            body_lines = body_lines[1:]
        if body_lines:
            text += "\n".join([" " + l for l in body_lines]) + "\n"

        text += "\n " + 26 * ' ' + "❦ ❦ ❦ ❦ \n\n"

        body = DiffControl._render_body(diff)
        if body is None:
            success = vcs.fetch_missing_data(commit._commit,
                                             commit._repo._repo)
            if success:
                body = DiffControl._render_body(diff)

        if body is None:
            body = "‼ Missing data for commit %s and failed to fetch it." % commit.oid

        text += body
        doc = DiffDocument(text, cursor_position=0)

        self.buffer.set_document(doc, bypass_readonly=True)

    @staticmethod
    def name_from_signature(sign: Signature) -> str:
        ''' Returns: Author Name <email> '''
        return "%s <%s>" % (sign.name, sign.email)

    @staticmethod
    def date_from_signature(sign: Signature) -> str:
        ''' Returns date formatted to current local and timezone'''
        date = datetime.datetime.fromtimestamp(sign.time, LOCAL_TZ)
        return date.strftime('%c')

    def preferred_width(self, max_available_width: int) -> Optional[int]:
        """
        This should return the preferred width.

        Note: We don't specify a preferred width according to the content,
              because it would be too expensive. Calculating the preferred
              width can be done by calculating the longest line, but this would
              require applying all the processors to each line. This is
              unfeasible for a larger document, and doing it for small
              documents only would result in inconsistent behaviour.
        """
        return max_available_width / 2

    def preferred_height(self, width: int, max_available_height: int,
                         wrap_lines: bool, get_line_prefix) -> Optional[int]:
        return screen_height() / 2


class DiffView(Frame):
    ''' Represents the hideable view for diffs which provides a read only
        buffer.
    '''  # pylint: disable=too-few-public-methods

    # pylint: disable=too-many-arguments
    def __init__(self,
                 title: AnyFormattedText = "",
                 style: str = "",
                 width: AnyDimension = None,
                 height: AnyDimension = None,
                 key_bindings: Optional[KeyBindings] = None,
                 modal: bool = False):
        buffer = Buffer(read_only=True, name='diff')
        self._search = SearchToolbar(vi_mode=True)
        self.control = DiffControl(buffer, self._search.control)
        body = HSplit([
            Window(self.control,
                   right_margins=[ScrollbarMargin(display_arrows=True)]),
            self._search
        ])
        super().__init__(body=body,
                         title=title,
                         style=style,
                         width=width,
                         height=height,
                         key_bindings=key_bindings,
                         modal=modal)
