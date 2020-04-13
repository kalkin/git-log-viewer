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
