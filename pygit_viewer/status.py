''' Status bar ui stuff '''

from typing import List, Optional, Tuple

from prompt_toolkit.layout import BufferControl, UIContent
from prompt_toolkit.layout.controls import GetLinePrefixCallable


class StatusContent(UIContent):
    '''
        A status bar consisting of one line
    '''

    # pylint: disable=too-few-public-methods

    def __init__(self):
        def get_line(_: int) -> List[Tuple[str, str]]:
            result = self.text.strip()
            length = len(result)
            diff = self.width - length
            if diff >= 0:
                result += ' ' * diff
            else:
                result = result[:length - diff]
                result += '…'

            return [('bg:ansiblue ansiwhite', result)]

        super().__init__(line_count=1, get_line=get_line)
        self.text = ''
        self.width = 0

    def get_height_for_line(
            self,
            lineno: int,
            width: int,
            get_line_prefix: Optional[GetLinePrefixCallable],
            slice_stop: Optional[int] = None,
    ) -> int:
        return 1


class StatusBar(BufferControl):
    ''' Status bar implementation '''
    def __init__(self):
        super().__init__(focusable=False)
        self.content = StatusContent()

    def set_status(self, text: str) -> None:
        ''' Set some text to show in the status bar '''
        self.content.text = text

    def clear(self) -> None:
        ''' Clear the content of the status bar '''
        self.content.text = ''

    def create_content(self,
                       width: int,
                       height: int,
                       preview_search: bool = False) -> UIContent:
        self.content.width = width
        return self.content
