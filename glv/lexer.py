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
''' Custom lexers '''
from prompt_toolkit.lexers import PygmentsLexer
from pygments.lexer import bygroups
from pygments.lexers.diff import DiffLexer
from pygments.token import Generic, Text, Token


def _get_lexer() -> PygmentsLexer:
    token = Token.Commit
    DiffLexer.tokens['old_root'] = DiffLexer.tokens['root']
    DiffLexer.tokens['root'] = [
        (r'Author:\s+.*', token.Author),
        (r'AuthorDate:\s+.*', token.AuthorDate),
        (r'Commit:\s+.*', token.Id),
        (r'Committer:\s+.*', token.Committer),
        (r'CommitDate:\s+.*', token.CommitDate),
        (r'Modules:\s+.*', token.Modules),
        (r'Refs:\s+.*', token.Refs),
        (r'^---$', token.End, 'diff_stats'),
        (r'.*\n', Text),
    ]

    DiffLexer.tokens['diff_stats'] = [
        (r'^ \d+ files? changed.+\n', token.DiffSummary, 'old_root'),
        (r'^(.+)( \|\s*\d+\s*)', bygroups(token.FileName, Text)),
        (r'^‼.*', Text),
        (r'[+]+', Generic.Inserted),
        (r'[-]+', Generic.Deleted),
        (r'.*\n', Text),
    ]

    return PygmentsLexer(DiffLexer)


COMMIT_LEXER = _get_lexer()
