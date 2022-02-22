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

#[derive(Default)]
struct ResultManager {
    finished: bool,
    selected: Option<usize>,
    results: Vec<SearchResult>,
    seen: usize,
}

impl ResultManager {
    fn consume(&mut self, event: SearchProgress) {
        match event {
            SearchProgress::Searched(n) => self.seen += n,
            SearchProgress::Found(result) => {
                if self.selected.is_none() {
                    self.selected = Some(0);
                }
                self.results.push(result);
            }
            SearchProgress::Finished => self.finished = true,
        }
    }

    fn next(&mut self) {
        if let Some(i) = self.selected {
            if i + 1 < self.results.len() {
                self.selected = Some(i + 1);
            } else {
                self.selected = Some(0);
            }
        } else if !self.results.is_empty() {
            self.selected = Some(0);
        }
    }

    fn prev(&mut self) {
        if let Some(i) = self.selected {
            if 0 < i {
                self.selected = Some(i - 1);
            } else if !self.results.is_empty() {
                self.selected = Some(self.results.len() - 1);
            }
        } else if !self.results.is_empty() {
            self.selected = Some(self.results.len());
        }
    }

    fn selected(&mut self) -> Option<SearchResult> {
        self.selected.and_then(|i| self.results.get(i).cloned())
    }
}

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
        let mut line = Vec::with_capacity(3);
        match self.direction {
            Direction::Forward => {
                line.push(style("/".to_string()));
            }
            Direction::Backward => {
                line.push(style("?".to_string()));
            }
        }
        line.push(style(self.input.text().to_string()));
        line.push(style(format!(
            "\tFound({}) / Seen({})",
            self.results.results.len(),
            self.results.seen
        )));
        shorten_line(line, width)
    }

    #[must_use]
    pub fn needle(&self) -> Needle {
        Needle::new(self.input.text(), self.direction)
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

    pub fn on_event(&mut self, event: Event) -> HandleEvent {
        match self.capture.state() {
            State::Hidden => self.hiden_state_on_event(event),
            State::CaptureNeedle(dir) => match self.input.on_event(event) {
                HandleEvent::Handled => {
                    let text = self.input.text().clone();
                    if !text.is_empty() {
                        self.needle = Some(Needle::new(&text, *dir));
                    }
                    self.results = ResultManager::default();
                    HandleEvent::Handled
                }
                HandleEvent::Ignored => match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                    }) => {
                        let text = self.input.text().clone();
                        self.needle = Some(Needle::new(&text, *dir));
                        self.results = ResultManager::default();
                        self.capture.on_event(search::Event::Text(text));
                        HandleEvent::Handled
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        modifiers: KeyModifiers::NONE,
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

    fn search_on_event(&mut self, event: Event) -> HandleEvent {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.capture.on_event(search::Event::Cancel);
                self.reset();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.results.next();
                self.goto = self.results.selected();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('N'),
                modifiers: KeyModifiers::SHIFT,
            }) => {
                self.results.prev();
                self.goto = self.results.selected();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
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

    fn hiden_state_on_event(&mut self, event: Event) -> HandleEvent {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.capture
                    .on_event(search::Event::Activate(Direction::Forward));
                self.direction = Direction::Forward;
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
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
                let was_empty = self.results.results.is_empty();
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
#[cfg(test)]
mod test_result_manager {
    use crate::ui::base::data::SearchProgress;
    use crate::ui::base::search::SearchResult;
    use crate::ui::search::ResultManager;

    #[test]
    fn empty() {
        let mut results = ResultManager::default();
        assert!(results.selected.is_none(), "Starts out empty");
        results.next();
        assert!(results.selected.is_none(), "No selected on empty");
        results.prev();
        assert!(results.selected.is_none(), "No selected on empty");
        results.consume(SearchProgress::Searched(23));
        assert!(results.selected.is_none(), "Still empty");
        results.consume(SearchProgress::Finished);
        assert!(results.selected.is_none(), "Still empty");
    }
    #[test]
    fn selecting_results() {
        let mut results = ResultManager::default();
        assert!(results.selected.is_none(), "Starts out empty");
        results.consume(SearchProgress::Found(SearchResult(vec![0])));
        assert!(results.selected.is_some(), "We have a selected");
        results.consume(SearchProgress::Found(SearchResult(vec![1])));
        results.consume(SearchProgress::Found(SearchResult(vec![2])));
        results.next();
        assert_eq!(results.selected().unwrap(), SearchResult(vec![1]));
        results.next();
        assert_eq!(results.selected().unwrap(), SearchResult(vec![2]));
        results.next();
        assert_eq!(
            results.selected().unwrap(),
            SearchResult(vec![0]),
            "Loop over the results"
        );
        results.prev();
        assert_eq!(results.selected().unwrap(), SearchResult(vec![2]));
        results.prev();
        assert_eq!(results.selected().unwrap(), SearchResult(vec![1]));
        results.prev();
        assert_eq!(results.selected().unwrap(), SearchResult(vec![0]));
    }
}
