import logging
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
    def __init__(self, objekt: object, colors: dict[str, str]):
        super().__init__(objekt)
        self._colors = colors

    def __getattr__(self, attr):
        key = '%s_color' % attr
        try:
            colorname = self._colors[key]
        except KeyError:
            colorname = ''
        result = getattr(self._wrapped, attr)
        return (colorname, result)
