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
import logging

import git
import pykka
from prompt_toolkit.application import get_app

from glv.providers import Provider

LOG = logging.getLogger('glv')


class ProviderActor(pykka.ThreadingActor):
    def __init__(self, provider: Provider):
        super().__init__()
        self._provider = provider
        self.use_daemon_thread = True

    def on_receive(self, message: str) -> str:
        if message.startswith("Merge pull request #") \
                and self._provider.has_match(message):
            try:
                return self._provider.provide(message)
            except Exception as exc:  # pylint: disable=broad-except
                LOG.error("Error: %s", exc)
            finally:
                get_app().invalidate()

        return message


class ModuleActor(pykka.ThreadingActor):
    def __init__(self, working_dir: str, modules: list[str] = None):
        super().__init__()
        self._cache = {}
        self.git_cmd = git.cmd.Git(working_dir=working_dir)
        self.modules = modules or []
        self.use_daemon_thread = True

    def on_receive(self, message: tuple[str, str]) -> list[str]:
        bellow, oid = message
        revision = '%s..%s' % (bellow, oid)
        changed = self.git_cmd.diff(revision,
                                    '--',
                                    *self.modules,
                                    name_only=True,
                                    no_renames=True,
                                    no_color=True).splitlines()
        result = []
        for directory in sorted(self.modules, reverse=True):
            for _file in changed:
                if _file.startswith(directory):
                    result.append(directory)
                    break
        get_app().invalidate()
        return result
