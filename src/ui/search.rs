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

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::style;

use crate::ui::base::data::SearchProgress;
use crate::ui::base::search::{Direction, Needle, SearchResult, State};
use crate::ui::base::{search, shorten_line, Drawable, HandleEvent, StyledLine};
use crate::ui::input::InputLine;

use super::base::search::ResultManager;

#[allow(clippy::module_name_repetitions)]
pub struct SearchWidget {
    input: InputLine,
    needle: Option<Needle>,
    goto: Option<SearchResult>,
    capture: search::NeedleCapture,
    direction: Direction,
    results: ResultManager,
}

impl Default for SearchWidget {
    fn default() -> Self {
        Self {
            input: InputLine::default(),
            needle: None,
            goto: None,
            capture: search::NeedleCapture::default(),
            direction: Direction::Forward,
            results: ResultManager::default(),
        }
    }
}

impl SearchWidget {
    pub fn render(&mut self, width: usize) -> StyledLine<String> {
        let mut line = StyledLine {
            content: Vec::with_capacity(3),
        };
        match self.direction {
            Direction::Forward => {
                line.content.push(style("/".to_owned()));
            }
            Direction::Backward => {
                line.content.push(style("?".to_owned()));
            }
        }
        line.content.push(style(self.input.text().to_string()));
        line.content.push(style(format!(
            "\tFound({}) / Seen({})",
            self.results.results().len(),
            self.results.seen()
        )));
        shorten_line(line, width)
    }

    #[must_use]
    pub fn needle(&self) -> Needle {
        Needle::smart_case(self.input.text(), self.direction)
    }

    pub fn search_value(&mut self) -> Option<Needle> {
        let result = self.needle.clone();
        self.needle = None;
        result
    }

    pub fn selected(&mut self) -> Option<SearchResult> {
        match self.capture.state() {
            State::Search(_) => {
                if self.goto.is_some() {
                    let result = self.goto.clone();
                    self.goto = None;
                    return result;
                }
                None
            }
            _ => None,
        }
    }

    pub fn on_event(&mut self, event: &Event) -> HandleEvent {
        match self.capture.state() {
            State::Hidden => self.hiden_state_on_event(event),
            State::CaptureNeedle(dir) => match self.input.on_event(event) {
                HandleEvent::Handled => {
                    let text = self.input.text().clone();
                    if !text.is_empty() {
                        self.needle = Some(Needle::smart_case(&text, *dir));
                    }
                    self.results = ResultManager::default();
                    HandleEvent::Handled
                }
                HandleEvent::Ignored => match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                        ..
                    }) => {
                        let text = self.input.text().clone();
                        self.needle = Some(Needle::smart_case(&text, *dir));
                        self.results = ResultManager::default();
                        self.capture.on_event(search::Event::Text(text));
                        HandleEvent::Handled
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        modifiers: KeyModifiers::NONE,
                        ..
                    }) => {
                        self.capture.on_event(search::Event::Cancel);
                        self.reset();
                        HandleEvent::Handled
                    }
                    _ => HandleEvent::Ignored,
                },
            },
            State::Search(_) => self.search_on_event(event),
        }
    }

    fn search_on_event(&mut self, event: &Event) -> HandleEvent {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.capture.on_event(search::Event::Cancel);
                self.reset();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.results.next();
                self.goto = self.results.selected();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('N'),
                modifiers: KeyModifiers::SHIFT,
                ..
            }) => {
                self.results.prev();
                self.goto = self.results.selected();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.direction = Direction::Forward;
                self.capture
                    .on_event(search::Event::Activate(search::Direction::Forward));
                self.reset();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.direction = Direction::Backward;
                self.capture
                    .on_event(search::Event::Activate(search::Direction::Backward));
                self.reset();
                HandleEvent::Handled
            }
            _ => HandleEvent::Ignored,
        }
    }

    fn hiden_state_on_event(&mut self, event: &Event) -> HandleEvent {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.capture
                    .on_event(search::Event::Activate(Direction::Forward));
                self.direction = Direction::Forward;
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.direction = Direction::Backward;
                self.capture
                    .on_event(search::Event::Activate(Direction::Backward));
                HandleEvent::Handled
            }
            _ => HandleEvent::Ignored,
        }
    }

    pub fn consume(&mut self, event: SearchProgress) {
        match event {
            SearchProgress::Found(_) => {
                let was_empty = self.results.results().is_empty();
                self.results.consume(event);
                if was_empty {
                    self.goto = self.results.selected();
                }
            }
            _ => {
                self.results.consume(event);
            }
        }
    }

    fn reset(&mut self) {
        self.needle = None;
        self.results = ResultManager::default();
        self.input = InputLine::default();
    }

    pub fn is_visible(&self) -> bool {
        *self.capture.state() != search::State::Hidden
    }
}
