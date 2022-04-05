// Copyright (C) 2021  Bahtiar `kalkin-` Gadimov <bahtiar@gadimov.de>
//
// This file is part of git-log-viewer
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::ui::base::{Area, Drawable, HandleEvent, StyledArea, StyledLine};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::style;
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

#[allow(clippy::module_name_repetitions)]
pub struct InputLine(String);

impl InputLine {
    pub const fn text(&self) -> &String {
        &self.0
    }
}

impl Drawable for InputLine {
    fn render(&mut self, _area: &Area) -> StyledArea<String> {
        vec![StyledLine {
            content: vec![style(self.0.clone())],
        }]
    }

    fn on_event(&mut self, event: Event) -> HandleEvent {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            }) => {
                self.0.push(c);
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
            }) => {
                let cur = UnicodeWidthStr::width(self.0.as_str());
                if cur > 0 {
                    let string = self.0.clone();
                    let (tmp, _) = string.unicode_truncate(cur - 1);
                    self.0 = tmp.to_owned();
                }
                HandleEvent::Handled
            }
            _ => HandleEvent::Ignored,
        }
    }
}

impl Default for InputLine {
    fn default() -> Self {
        Self("".to_owned())
    }
}

#[cfg(test)]
mod test_input_widget {
    use crate::ui::base::{Drawable, HandleEvent};
    use crate::ui::input::InputLine;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn small_characters() {
        let input = &mut InputLine::default();
        handle_event(
            input,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE,
            }),
        );
        assert_eq!(input.text(), "c");
    }
    #[test]
    fn big_characters() {
        let input = &mut InputLine::default();
        handle_event(
            input,
            Event::Key(KeyEvent {
                code: KeyCode::Char('C'),
                modifiers: KeyModifiers::SHIFT,
            }),
        );
        assert_eq!(input.text(), "C");
    }
    #[test]
    fn backspace() {
        let input = &mut InputLine::default();
        handle_event(
            input,
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
            }),
        );
        assert_eq!(input.text(), "");
        handle_event(
            input,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE,
            }),
        );
        assert_eq!(input.text(), "c");
        handle_event(
            input,
            Event::Key(KeyEvent {
                code: KeyCode::Char('y'),
                modifiers: KeyModifiers::NONE,
            }),
        );
        assert_eq!(input.text(), "cy");
        handle_event(
            input,
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
            }),
        );
        assert_eq!(input.text(), "c");
    }

    fn handle_event(input: &mut InputLine, event: Event) {
        assert_eq!(input.on_event(event), HandleEvent::Handled);
    }
}
