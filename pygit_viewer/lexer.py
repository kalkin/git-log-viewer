''' Custom lexers '''
import pygments.token
from prompt_toolkit.lexers import PygmentsLexer
from pygments.lexers.diff import DiffLexer


def _get_lexer() -> PygmentsLexer:
    token = pygments.token.Token.Commit
    DiffLexer.tokens['old_root'] = DiffLexer.tokens['root']
    DiffLexer.tokens['root'] = [
        (r'Author:\s+.*', token.Author),
        (r'AuthorDate:\s+.*', token.AuthorDate),
        (r'Commit:\s+.*', token.Id),
        (r'Committer:\s+.*', token.Committer),
        (r'CommitDate:\s+.*', token.CommitDate),
        (r'Modules:\s+.*', token.Modules),
        (r'Refs:\s+.*', token.Refs),
        ('\n---\n', token.End, 'old_root'),
        (r'.*\n', pygments.token.Text),
    ]

    return PygmentsLexer(DiffLexer)


COMMIT_LEXER = _get_lexer()
