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

use crossterm::style::{style, Attribute};

use crate::search::search_line;
use crate::ui::base::search::{Direction, Needle, SearchResult};
use crate::ui::base::{Pos, StyledArea, StyledLine};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::thread::JoinHandle;

#[allow(clippy::module_name_repetitions)]
pub trait DataAdapter<T> {
    fn get_line(&mut self, i: Pos, selected: bool) -> StyledLine<String>;
    fn get_data(&mut self, i: Pos) -> &T;

    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn search(&mut self, needle: Needle, start: usize) -> Receiver<SearchProgress>;
}

impl DataAdapter<String> for VecAdapter {
    fn get_line(&mut self, i: usize, selected: bool) -> StyledLine<String> {
        let text = &self.content[i];
        let mut content = style(text.clone());
        if selected {
            content.style_mut().attributes.set(Attribute::Reverse);
        }
        vec![content]
    }

    fn get_data(&mut self, i: usize) -> &String {
        &self.content[i]
    }

    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    fn len(&self) -> usize {
        self.content.len()
    }

    fn search(&mut self, _needle: Needle, _start: usize) -> Receiver<SearchProgress> {
        todo!()
    }
}

pub enum SearchProgress {
    Searched(usize),
    Found(SearchResult),
    Finished,
}

pub struct VecAdapter {
    content: Vec<String>,
}

impl VecAdapter {
    #[must_use]
    #[allow(dead_code)]
    pub fn new(content: Vec<String>) -> Self {
        Self { content }
    }
}

#[cfg(test)]
mod test_vec_adapter {
    use crossterm::style::Attribute;

    use crate::ui::base::data::DataAdapter;
    use crate::ui::base::data::VecAdapter;
    use crate::ui::base::test_helpers::lore_ipsum_lines;

    #[test]
    fn foo() {
        let adapter = &mut VecAdapter::new(lore_ipsum_lines(30));
        assert_eq!(adapter.len(), 30);
        assert!(!adapter.is_empty());
        let line = adapter.get_line(0, false);
        for sc in line {
            assert!(!sc.style().attributes.has(Attribute::Reverse));
        }
        let selected_line = adapter.get_line(0, true);
        for sc in selected_line {
            assert!(sc.style().attributes.has(Attribute::Reverse));
        }
    }
}

pub struct StyledAreaAdapter {
    pub content: StyledArea<String>,
    pub thread: Option<JoinHandle<()>>,
}

impl DataAdapter<String> for StyledAreaAdapter {
    fn get_line(&mut self, i: usize, selected: bool) -> StyledLine<String> {
        let mut content: StyledLine<String> = self.content[i].clone();
        if selected {
            for c in &mut content {
                c.style_mut().attributes.set(Attribute::Reverse);
            }
        }
        content
    }

    fn get_data(&mut self, _i: usize) -> &String {
        todo!()
    }

    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    fn len(&self) -> usize {
        self.content.len()
    }

    fn search(&mut self, needle: Needle, start: usize) -> Receiver<SearchProgress> {
        let (rx, tx) = mpsc::channel::<SearchProgress>();
        let cloned = self.content.clone();
        let thread = thread::spawn(move || {
            let mut range = (start..cloned.len()).collect::<Vec<usize>>();
            let rest = (0..start).collect::<Vec<usize>>();
            range.extend(rest);
            if *needle.direction() == Direction::Backward {
                range = range.into_iter().rev().collect::<Vec<_>>();
            }
            for i in range {
                let line = &cloned[i];
                for m in search_line(line, &needle) {
                    let mut f = m.0.clone();
                    f.insert(0, i);
                    if rx.send(SearchProgress::Found(SearchResult(f))).is_err() {
                        return;
                    }
                }
                if rx.send(SearchProgress::Searched(1)).is_err() {
                    return;
                }
            }

            #[allow(unused_must_use)]
            {
                rx.send(SearchProgress::Finished);
            }
        });
        self.thread = Some(thread);
        tx
    }
}
