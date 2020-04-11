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
        ('\n---\n', token.End, 'diff_stats'),
        (r'.*\n', Text),
    ]

    DiffLexer.tokens['diff_stats'] = [
        (r'^ \d+ files? changed.+\n', token.DiffSummary, 'old_root'),
        (r'^(.+)( \|\s*\d+\s*)', bygroups(token.FileName, Text)),
        (r'[+]+', Generic.Inserted),
        (r'[-]+', Generic.Deleted),
        (r'.*\n', Text),
    ]

    return PygmentsLexer(DiffLexer)


COMMIT_LEXER = _get_lexer()
