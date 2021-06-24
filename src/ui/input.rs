use crate::ui::base::{Area, Drawable, HandleEvent, StyledArea};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::style;
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

#[allow(clippy::module_name_repetitions)]
pub struct InputLine(String);

impl InputLine {
    pub fn text(&self) -> &String {
        &self.0
    }
}

impl Drawable for InputLine {
    fn render(&mut self, _area: &Area) -> StyledArea<String> {
        vec![vec![style(self.0.clone())]]
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
                    self.0 = tmp.to_string();
                }
                HandleEvent::Handled
            }
            _ => HandleEvent::Ignored,
        }
    }
}

impl Default for InputLine {
    fn default() -> Self {
        InputLine("".to_string())
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
