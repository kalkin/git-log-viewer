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

use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Needle {
    text: String,
    direction: Direction,
}

impl Default for Needle {
    fn default() -> Self {
        Self {
            text: "".to_owned(),
            direction: Direction::Forward,
        }
    }
}

impl Needle {
    pub fn new(text: &str, dir: Direction) -> Self {
        Self {
            text: text.to_owned(),
            direction: dir,
        }
    }
    pub fn text(&self) -> &String {
        &self.text
    }

    pub fn direction(&self) -> &Direction {
        &self.direction
    }
}

impl Display for Needle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut text = "".to_owned();
        match self.direction {
            Direction::Forward => text.push('/'),
            Direction::Backward => text.push('?'),
        }
        text.push_str(&self.text);

        f.write_str(&text)
    }
}

impl Debug for Needle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Needle({})", self))
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
    pub fn state(&self) -> &State {
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
                Event::Text(text) => self.0 = State::Search(Needle::new(&text, *dir)),
            },
            State::Search(needle) => match event {
                Event::Activate(dir) => {
                    self.0 = State::CaptureNeedle(dir);
                }
                Event::Cancel => self.0 = State::Hidden,
                Event::Text(text) => {
                    self.0 = State::Search(Needle::new(&text, *needle.direction()));
                }
            },
        }
    }
}

#[cfg(test)]
mod test_needle_capture {
    use crate::ui::base::search::{Direction, Event, Needle, NeedleCapture, State};

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
        capture.on_event(Event::Text("asd".to_string()));
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
        capture.on_event(Event::Text("foo".to_string()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle {
                text: "foo".to_string(),
                direction: Direction::Backward
            }),
            "Reached Search state"
        );
    }
    #[test]
    fn search_state() {
        let mut capture = NeedleCapture::default();
        capture.on_event(Event::Activate(Direction::Forward));
        capture.on_event(Event::Text("foo".to_string()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle {
                text: "foo".to_string(),
                direction: Direction::Forward
            }),
            "Reached Search state"
        );
        capture.on_event(Event::Activate(Direction::Backward));
        assert_eq!(
            *capture.state(),
            State::CaptureNeedle(Direction::Backward),
            "Reached CaptureNeedle state"
        );
        capture.on_event(Event::Text("bar".to_string()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle {
                text: "bar".to_string(),
                direction: Direction::Backward
            }),
            "Change back to search text"
        );
        capture.on_event(Event::Text("foo".to_string()));
        assert_eq!(
            *capture.state(),
            State::Search(Needle {
                text: "foo".to_string(),
                direction: Direction::Backward
            }),
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
