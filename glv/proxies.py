import itertools
import logging
import re
from functools import lru_cache
from typing import List, Tuple

from prompt_toolkit.formatted_text import StyleAndTextTuples
from prompt_toolkit.search import SearchState

from glv import vcs

LOG = logging.getLogger('glv')


class BaseProxy:
    def __init__(self, objekt: object):
        self._wrapped: object = objekt

    def __getattr__(self, attr):
        if hasattr(self._wrapped, attr) and \
                callable(getattr(self._wrapped, attr)):
            return getattr(self._wrapped, attr)()

        return getattr(self._wrapped, attr)


class ColorProxy(BaseProxy):
    searchable_fileds = ['id', 'modules', 'author_name', 'subject']
    non_italic_fields = ['type']

    def __init__(self, objekt: object, colors: dict[str, str],
                 search_state: SearchState, date_max_len, name_max_len):
        super().__init__(objekt)
        self._colors = colors
        self._search_state = search_state
        self._date_max_len = date_max_len
        self._name_max_len = date_max_len = name_max_len

    @lru_cache
    def __getattr__(self, attr):
        key = '%s_color' % attr
        try:
            colorname = self._colors[key]
        except KeyError:
            colorname = ''
        result = getattr(self._wrapped, attr)
        if attr == 'author_name':
            result = result.ljust(self._date_max_len, " ")
        elif attr == 'author_date':
            result = result.ljust(self._name_max_len, " ")

        if self._search_state is None or attr not in self.searchable_fileds:
            result = (colorname, result)
        else:
            # handle search highlighting
            result = highlight_substring(self._search_state,
                                         (colorname, result))

        if self._wrapped.is_commit_link and attr not in self.non_italic_fields:
            if isinstance(result, list):
                result = [('italic ' + x[0], x[1]) for x in result]
            else:
                result = ('italic ' + result[0], result[1])

        return result

    @property
    @lru_cache
    def references(self) -> List[Tuple[str, str]]:
        branches = self._wrapped.references
        if branches == ['']:
            return []
        color = vcs.CONFIG['history']['branches_color']
        branch_tupples = [[('', ' '), (color, '«%s»' % name)]
                          for name in branches
                          if not name.startswith('patches/')]
        return list(itertools.chain(*branch_tupples))


def highlight_substring(search: SearchState,
                        parts: Tuple[str, str]) -> StyleAndTextTuples:
    needle: str = search.text
    haystack = parts[1]
    matches = list(re.finditer(re.escape(needle), haystack))
    if not matches:
        return parts

    original_h = parts[0]
    new_h = parts[0] + ' ansired bold'
    cur = 0
    result = []
    if matches[0].start() == 0:
        match = matches[0]
        result = [(new_h, needle)]
        cur = len(needle)
        matches = matches[1:]

    for match in matches:
        result += [(original_h, haystack[cur:match.start()])]
        result += [(new_h, haystack[match.start():match.end()])]
        cur = match.end()

    if cur < len(haystack):
        result += [(original_h, haystack[cur:])]
    return result
