''' Utilities for parsing data on the command line '''


class RevisionRange:  # pylint: disable=too-few-public-methods
    ''' Parsed revision range. '''
    def __init__(self, string: str):
        self.input = string
        self.end = None
        if '..' in string:
            self.end, self.start = string.split('..')
            if not self.start:
                self.start = "HEAD"
        else:
            self.start = string


def parse_revisions(revisions: list[str] = None) -> list[RevisionRange]:
    ''' Parses revision strings specified on the command line to RevisionRange
        objects .
    '''
    revisions = revisions or ['HEAD']
    return [RevisionRange(rev) for rev in revisions]
