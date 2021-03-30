import itertools
import logging
import re
import textwrap
from functools import lru_cache
from typing import List, Optional, Tuple

import pkg_resources
from prompt_toolkit.formatted_text import StyleAndTextTuples
from prompt_toolkit.search import SearchState

from glv import vcs
from glv.commit import Commit
from glv.icon import ASCII
from glv.utils import ModuleChanges, mod_changes

LOG = logging.getLogger('glv')


class LogEntry:
    searchable_fileds = ['id', 'modules', 'author_name', 'subject']
    non_italic_fields = ['type']

    def __init__(self, commit: Commit, working_dir: str,
                 search_state: SearchState) -> None:
        self.commit = commit
        self._working_dir = working_dir
        self.search_state = search_state
        self._colors: dict[str, str] = vcs.CONFIG['history']

    def __getattr__(self, attr: str):
        if attr.endswith('_colored'):
            return self._get_colored(attr)
        if hasattr(self.commit, attr):
            return getattr(self.commit, attr)
        raise RuntimeError("Dunno how to handle %s" % attr)

    def _get_colored(self, attr: str) -> StyleAndTextTuples:
        attr = attr.partition('_colored')[0]
        key = '%s_color' % attr
        try:
            colorname = self._colors[key]
        except KeyError:
            colorname = ''

        result = getattr(self, attr)

        return self._add_highlighting(attr, colorname, result)

    @property
    def modules(self) -> Tuple[str, str]:
        try:
            config = vcs.CONFIG['history']['modules_content']
        except KeyError:
            config = 'modules-component'

        try:
            modules_max_width = vcs.CONFIG['history']['modules_max_width']
        except KeyError:
            modules_max_width = 35

        changes: ModuleChanges = mod_changes(self._working_dir)
        modules = changes.commit_modules(self.commit)

        subject = self.commit.subject

        if config == 'modules-component' and not modules \
                and has_component(subject):
            parsed_module = parse_component(subject)
            if parsed_module and parsed_module not in modules and not is_hex(
                    parsed_module):
                modules.append(parsed_module)

        if config == 'component':
            modules = []
            if has_component(subject):
                parsed_module = parse_component(subject)
                if parsed_module:
                    modules = [parsed_module]

        text = ', '.join([':' + x for x in modules])
        if len(text) > modules_max_width:
            text = ':(%d modules)' % len(modules)
        return text

    @property
    def is_commit_link(self) -> bool:
        return self.commit.is_commit_link

    @property
    @lru_cache
    def author_name(self):
        width = 10
        name = self.commit.author_name
        tmp = textwrap.shorten(name, width=width, placeholder="…")
        if tmp == '…':
            return name[0:width - 1] + '…'
        return tmp

    @property
    @lru_cache
    def icon(self) -> Tuple[str, str]:
        subject = self.commit.subject
        for (regex, icon) in icon_collection():
            if re.match(regex, subject, flags=re.I):
                return icon
        return '  '

    @property
    def subject(self) -> Tuple[str, str]:
        try:
            parts = vcs.CONFIG['history']['subject_parts'].split()
        except KeyError:
            parts = ['component', 'verb']

        subject = self.commit.subject
        if has_component(subject):
            component = parse_component(subject)
            if component and not is_hex(component):
                if 'modules-component' in parts:
                    modules = vcs.modules(self._working_dir)
                    if not modules or component in modules:
                        subject = remove_component(subject)
                elif 'component' not in parts:
                    subject = remove_component(subject)

        if 'icon-or-verb' in parts:
            if self.icon[1] != '  ':
                subject = remove_verb(subject)
        elif 'verb' not in parts:
            subject = remove_verb(subject)

        return subject

    @property
    @lru_cache
    def type(self):
        return self.commit.type_icon + self._arrows

    @property
    def _arrows(self) -> str:
        if self.commit.is_merge:
            if self.commit.subject.startswith('Update :') \
                    or ' Import ' in self.commit.subject:
                if self.commit.is_fork_point:
                    return "⇤┤"
                return '⇤╮'
            if self.commit.is_fork_point:
                return "─┤"
            return "─┐"
        if self.commit.is_fork_point:
            return "─┘"
        return ''

    def _add_highlighting(self, attr: str, colorname: str,
                          value) -> StyleAndTextTuples:
        if self.search_state is None or attr not in self.searchable_fileds:
            result = (colorname, value)
        else:
            # handle search highlighting
            result = highlight_substring(self.search_state, (colorname, value))

        if self.commit.is_commit_link and attr not in self.non_italic_fields:
            if isinstance(result, list):
                result = [('italic ' + x[0], x[1]) for x in result]
            else:
                result = ('italic ' + result[0], result[1])
        return result

    @lru_cache
    def author_name_short(self, max_len: int) -> str:
        return self.author_name.ljust(max_len, " ")

    @lru_cache
    def author_date_short(self, max_len: int) -> str:
        return self.author_rel_date.ljust(max_len, " ")

    def author_name_short_colored(self, max_len: int) -> StyleAndTextTuples:
        value = self.author_name_short(max_len)
        try:
            colorname = self._colors['author_name_color']
        except KeyError:
            colorname = ''
        return self._add_highlighting('author_name', colorname, value)

    def author_date_short_colored(self, max_len: int) -> StyleAndTextTuples:
        value = self.author_date_short(max_len)
        try:
            colorname = self._colors['author_date_color']
        except KeyError:
            colorname = ''
        return self._add_highlighting('author_date', colorname, value)

    @property
    @lru_cache
    def references_colored(self) -> List[Tuple[str, str]]:
        branches = self.commit.references
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


def icon_collection():
    name = vcs.CONFIG['history']['icon_set']
    result = None
    for entry_point in pkg_resources.iter_entry_points(group='glv_icons'):
        if entry_point.name == name:
            try:
                result = entry_point.load()
            except ModuleNotFoundError:
                pass

    if not result:
        result = ASCII
    return result


def has_component(subject: str) -> bool:
    return re.match(r'^\w+\([\w\d_-]+\)[\s:]\s*.*', subject, flags=re.I)


def parse_component(subject: str) -> Optional[str]:
    tmp = re.findall(r'^\w+\(([\w\d_-]+)\):.*', subject)
    if tmp:
        return tmp[0]
    return None


def is_hex(subject: str) -> bool:
    return re.match(r'^[0-9a-f]+$', subject, flags=re.I)


def remove_component(subject: str) -> bool:
    return re.sub(r'^(\w+)\([\w\d_-]+\)', '\\1', subject, flags=re.I, count=1)


def parse_verb(subject: str) -> Optional[str]:
    tmp = re.findall(r'^(\w+)(?:\([\w\d_-]+\)\s*:)?', subject, re.I)
    if tmp:
        return tmp[0]
    return None


def remove_verb(subject: str) -> bool:
    return re.sub(r'^(\w+)((?=\()|\s*:|\s)\s*',
                  '',
                  subject,
                  flags=re.I,
                  count=1)
