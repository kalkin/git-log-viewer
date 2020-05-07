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
''' Icon defintion consisting of regex + icon '''

ASCII = [
    (r'^Revert:?\s*', 'R '),
    (r'^fixup!\s+', 'f '),
    (r'^(hot|bug)?fix(ing|ed)?(\(.+\))?[\/:\s]+', 'B '),  # B for bug
    (r'^add(ed)?(:|\s)', '+ '),
    (r'^feat:?\s*', '+ '),
    (r'^build:?\s*', 'b '),
    (r'^doc(s|umentation)?:?\s*', 'D '),
    (r'^style:?\s*', 's '),
    (r'^test(\(.+\))?:?\s*', 'T '),
]

NERDFONT = [
    (r'^Revert:?\s*', ' '),
    (r'^BREAKING CHANGE:?\s*', '⚠ '),
    (r'^fixup!\s+', '\uf0e3 '),
    (r'^ADD:\s?[a-z0-9]+', ' '),
    (r'^ref(actor)?:?\s*', '⮔ '),
    (r'^lang:?\s*', '\ufac9'),
    (r'^deps(\(.+\))?:?\s*', '\uf487 '),
    (r'^config:?\s*', '\uf462 '),
    (r'^test(\(.+\))?:?\s*', '\uf45e '),
    (r'^ci(\(.+\))?:?\s*', '\uf085 '),
    (r'^perf(\(.+\))?:?\s*', '\uf9c4'),
    (r'^(bug)?fix(ing|ed)?(\(.+\))?[\/:\s]+', '\uf188 '),
    (r'^doc(s|umentation)?:?\s*', '✎ '),
    (r'^improvement:?\s*', '\ue370 '),
    (r'^CHANGE/?:?\s*', '\ue370 '),
    (r'^hotfix:?\s*', '\uf490 '),
    (r'^feat:?\s*', '➕'),
    (r'^add:?\s*', '➕'),
    (r'^(release|bump):?\s*', '\uf412 '),
    (r'^build:?\s*', '🔨'),
    (r'.*\bchangelog\b.*', '✎ '),
    (r'^refactor:?\s*', '⮔ '),
    (r'^.* Import .*', '⮈ '),
    (r'^Split .*', '\uf403 '),
    (r'^Remove:?\s+.*', '\uf48e '),
    (r'^Update :\w+.*', '\uf419 '),
    (r'^style:?\s*', '♥ '),
    (r'^DONE:?\s?[a-z0-9]+', '\uf41d '),
    (r'^rename?\s*', '\uf044 '),
]
