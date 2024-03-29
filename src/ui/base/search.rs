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

use std::fmt::Debug;

use getset::Getters;

use super::data::SearchProgress;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Clone, Debug, Eq, Getters, PartialEq)]
pub struct Needle {
    #[getset(get = "pub")]
    text: String,
    #[getset(get = "pub")]
    direction: Direction,
    #[getset(get = "pub")]
    ignore_case: bool,
}

impl Default for Needle {
    fn default() -> Self {
        Self {
            text: "".to_owned(),
            direction: Direction::Forward,
            ignore_case: false,
        }
    }
}

impl Needle {
    pub fn smart_case(text: &str, dir: Direction) -> Self {
        Self {
            text: text.to_owned(),
            direction: dir,
            ignore_case: text.chars().all(char::is_lowercase),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult(pub Vec<usize>);

#[derive(Debug, Eq, PartialEq)]
pub enum State {
    Hidden,
    CaptureNeedle(Direction),
    Search(Needle),
}

#[derive(Debug, Clone)]
pub enum Event {
    Activate(Direction),
    Cancel,
    Text(String),
}

pub struct NeedleCapture(State);

impl Default for NeedleCapture {
    fn default() -> Self {
        Self(State::Hidden)
    }
}

impl NeedleCapture {
    pub const fn state(&self) -> &State {
        &self.0
    }

    pub fn on_event(&mut self, event: Event) {
        match &self.0 {
            State::Hidden => {
                if let Event::Activate(dir) = event {
                    self.0 = State::CaptureNeedle(dir);
                }
            }
            State::CaptureNeedle(dir) => match event {
                Event::Activate(new_dir) => {
                    if new_dir != *dir {
                        self.0 = State::CaptureNeedle(new_dir);
                    }
                }
                Event::Cancel => self.0 = State::Hidden,
                Event::Text(text) => self.0 = State::Search(Needle::smart_case(&text, *dir)),
            },
            State::Search(needle) => match event {
                Event::Activate(dir) => {
                    self.0 = State::CaptureNeedle(dir);
                }
                Event::Cancel => self.0 = State::Hidden,
                Event::Text(text) => {
                    self.0 = State::Search(Needle::smart_case(&text, *needle.direction()));
                }
            },
        }
    }
}

#[derive(Default, Getters)]
pub struct ResultManager {
    finished: bool,
    selected: Option<usize>,
    #[getset(get = "pub")]
    results: Vec<SearchResult>,
    #[getset(get = "pub")]
    seen: usize,
}

impl ResultManager {
    pub fn consume(&mut self, event: SearchProgress) {
        match event {
            SearchProgress::Searched(n) => {
                self.seen = self.seen.saturating_add(n);
            }
            SearchProgress::Found(result) => {
                if self.selected.is_none() {
                    self.selected = Some(0);
                }
                self.results.push(result);
            }
            SearchProgress::Finished => self.finished = true,
        }
    }

    pub fn next(&mut self) {
        if self.results.is_empty() {
            log::info!("No search results");
        } else {
            let new_selected = self
                .selected
                .and_then(|i| i.checked_add(1))
                .filter(|i| *i < self.results.len())
                .unwrap_or_default();
            self.selected = Some(new_selected);
        }
    }

    pub fn prev(&mut self) {
        if self.results.is_empty() {
            log::info!("No search results");
        } else {
            let new_selected = self
                .selected
                .map(|i| {
                    i.checked_sub(1)
                        .unwrap_or_else(|| self.results.len().saturating_sub(1))
                })
                .filter(|i| *i < self.results.len())
                .unwrap_or_else(|| self.results.len().saturating_sub(1));
            self.selected = Some(new_selected);
        }
    }

    pub fn selected(&mut self) -> Option<SearchResult> {
        self.selected.and_then(|i| self.results.get(i).cloned())
    }
}

#[cfg(test)]
mod test_needle_capture {
    use crate::ui::base::search::{Direction, Event, Needle, NeedleCapture, State};
    use pretty_assertions::assert_eq;

    #[test]
    fn hidden_state() {
        let mut capture = NeedleCapture::default();
        assert_eq!(*capture.state(), State::Hidden, "Starts in hidden state");
        capture.on_event(Event::Cancel);
        assert_eq!(
            *capture.state(),
            State::Hidden,
            "Ignores Cancel Event in hidden state"
        );
        capture.on_event(Event::Text("asd".to_owned()));
        assert_eq!(
            *capture.state(),
            State::Hidden,
            "Ignores Text Event in hidden state"
        );
        capture.on_event(Event::Activate(Direction::Forward));
        assert_eq!(
            *capture.state(),
            State::CaptureNeedle(Direction::Forward),
            "Leaves hidden state on an Activate Event"
        );
    }
    #[test]
    fn capture_needle_state() {
        let mut capture = NeedleCapture::default();
        capture.on_event(Event::Activate(Direction::Backward));
        assert_eq!(
            *capture.state(),
            State::CaptureNeedle(Direction::Backward),
            "Reached CaptureNeedle state"
        );
        capture.on_event(Event::Activate(Direction::Forward));
        assert_eq!(
            *capture.state(),
            State::CaptureNeedle(Direction::Forward),
            "Still in CaptureNeedle state, but direction changed"
        );
        capture.on_event(Event::Cancel);
        assert_eq!(
            *capture.state(),
            State::Hidden,
            "Cancel event moves us to hidden state"
        );
        capture.on_event(Event::Activate(Direction::Backward));
        assert_eq!(
            *capture.state(),
            State::CaptureNeedle(Direction::Backward),
            "Back in CaptureNeedle state"
        );
        capture.on_event(Event::Text("foo".to_owned()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle {
                text: "foo".to_owned(),
                direction: Direction::Backward,
                ignore_case: true,
            }),
            "Reached Search state"
        );
    }
    #[test]
    fn search_state() {
        let mut capture = NeedleCapture::default();
        capture.on_event(Event::Activate(Direction::Forward));
        capture.on_event(Event::Text("foo".to_owned()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle::smart_case("foo", Direction::Forward)),
            "Reached Search state"
        );
        capture.on_event(Event::Activate(Direction::Backward));
        assert_eq!(
            *capture.state(),
            State::CaptureNeedle(Direction::Backward),
            "Reached CaptureNeedle state"
        );
        capture.on_event(Event::Text("bar".to_owned()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle::smart_case("bar", Direction::Backward)),
            "Change back to search text"
        );
        capture.on_event(Event::Text("foo".to_owned()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle::smart_case("foo", Direction::Backward)),
            "Change text on Text event"
        );
        capture.on_event(Event::Cancel);
        assert_eq!(
            *capture.state(),
            State::Hidden,
            "Cancel event moves us to hidden state"
        );
    }
}

#[cfg(test)]
mod test_result_manager {
    use pretty_assertions::assert_eq;

    use crate::ui::base::{
        data::SearchProgress,
        search::{ResultManager, SearchResult},
    };

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
