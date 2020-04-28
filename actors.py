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
