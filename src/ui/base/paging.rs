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

use crate::ui::base::{HandleEvent, Height, Pos};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::num::NonZeroUsize;

/// This structs helps to display only a `page_height` of data
pub struct Paging {
    top: usize,
    bottom: usize,
    page_height: Height,
    total_length: NonZeroUsize,
    selected: Pos,
}

impl Default for Paging {
    fn default() -> Self {
        Self {
            top: 0,
            bottom: 0,
            page_height: NonZeroUsize::new(1).unwrap(),
            total_length: NonZeroUsize::new(1).unwrap(),
            selected: 0,
        }
    }
}

impl Paging {
    #[cfg(test)]
    #[cfg(not(tarpaulin_include))]
    #[allow(clippy::arithmetic)]
    /// arithmetic: this is only used during testing
    pub const fn new(page_height: Height, total_length: NonZeroUsize) -> Self {
        let bottom = page_height.get() - 1;
        Self {
            top: 0,
            bottom,
            page_height,
            total_length,
            selected: 0,
        }
    }
    /// The top most visible data entry
    pub const fn top(&self) -> usize {
        self.top
    }

    /// The bottom most visible data entry
    pub const fn bottom(&self) -> usize {
        self.bottom
    }

    /// Scroll to next page and adjust the selected line accordingly
    fn next_page(&mut self) {
        if let Some(top) = self.top.checked_add(self.page_height.get()) {
            if top >= self.total_length.get() {
                self.selected = self.bottom;
            } else {
                self.top = top;
                self.bottom = self.top.saturating_add(self.page_height.get());
                self.selected = self.selected.saturating_add(self.page_height.get());
                #[allow(clippy::arithmetic)]
                // arithmetic: total_length is always >= 1, because it's a NonZeroUsize
                if self.bottom >= self.total_length.get() {
                    self.bottom = self.total_length.get() - 1;
                }
                #[allow(clippy::arithmetic)]
                // arithmetic: total_length is always >= 1, because it's a NonZeroUsize
                if self.selected >= self.total_length.get() {
                    self.selected = self.total_length.get() - 1;
                }
            }
        } else {
            self.selected = self.bottom;
        }
    }

    /// Scroll to prev page and adjust the selected line accordingly
    fn prev_page(&mut self) {
        self.top = self.top.saturating_sub(self.page_height.get());
        self.bottom = self
            .top
            .saturating_add(self.page_height.get().saturating_sub(1));
        self.selected = self.selected.saturating_sub(self.page_height.get());
    }

    /// Set current page height
    pub fn page_height(&mut self, height: Height, total_length: NonZeroUsize) {
        self.page_height = height;
        self.total_length = total_length;
        self.bottom = self
            .top
            .saturating_add(self.page_height.get().saturating_sub(1));
        #[allow(clippy::arithmetic)]
        // arithmetic: total_length is always >= 1, because it's a NonZeroUsize
        if self.bottom >= self.total_length.get() {
            self.bottom = self.total_length.get() - 1;
        }
    }

    /// Return the selected data entry index
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// Set the selected data entry index
    pub fn set_selected(&mut self, i: usize) {
        if self.total_length.get() <= i {
            log::error!(
                "Expected selected({}) < total_length({})",
                i,
                self.total_length
            );
            return;
        }
        if i < self.top {
            while i < self.top {
                self.prev_page();
            }
            self.selected = i;
        } else if i > self.bottom {
            while i > self.bottom {
                self.next_page();
            }
        } else {
            log::trace!("No paging needed");
        }
        self.selected = i;
    }

    /// Move selection to next data index
    fn select_next(&mut self) {
        self.selected = self.selected.saturating_add(1);

        #[allow(clippy::arithmetic)]
        // arithmetic: total_length is always >= 1, because it's a NonZeroUsize
        if self.selected >= self.total_length.get() {
            self.selected = self.total_length.get() - 1;
        }
        if self.bottom < self.selected {
            self.bottom = self.selected;
            self.top = self
                .bottom
                .saturating_sub(self.page_height.get())
                .saturating_add(1);
        }
    }

    /// Move selection to prev data index
    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        if self.selected < self.top {
            self.top = self.selected;
            self.bottom = self
                .top
                .saturating_add(self.page_height.get())
                .saturating_sub(1);
        }
    }

    pub fn on_event(&mut self, event: &Event) -> HandleEvent {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.select_prev();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.select_next();
                HandleEvent::Handled
            }

            Event::Key(KeyEvent {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.next_page();
                HandleEvent::Handled
            }
            Event::Key(KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.prev_page();
                HandleEvent::Handled
            }
            _ => HandleEvent::Ignored,
        }
    }

    pub fn set_total_length(&mut self, len: NonZeroUsize) {
        self.total_length = len;
    }
}

#[cfg(test)]
mod test_paging {
    use std::num::NonZeroUsize;

    use crate::ui::base::paging::Paging;
    use crate::ui::base::HandleEvent;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use pretty_assertions::assert_eq;

    #[test]
    fn next_page() {
        let pager = &mut Paging::new(
            NonZeroUsize::new(25).unwrap(),
            NonZeroUsize::new(30).unwrap(),
        );
        assert_eq!(pager.top(), 0);
        assert_eq!(pager.selected(), 0);
        assert_eq!(pager.bottom(), 24);
        handle_event(pager, KeyCode::PageDown);
        assert_eq!(pager.top(), 25);
        assert_eq!(pager.selected(), 25, "First visible row is selected");
        assert_eq!(pager.bottom(), 29, "bottom should be eq to length - 1");
        handle_event(pager, KeyCode::PageDown);
        assert_eq!(pager.top(), 25, "Top should not change");
        assert_eq!(pager.selected(), 29, "Selection should go to last");
        assert_eq!(pager.bottom(), 29, "Bottom should not change");
    }

    #[test]
    fn prev_page() {
        let pager = &mut Paging::new(
            NonZeroUsize::new(25).unwrap(),
            NonZeroUsize::new(30).unwrap(),
        );
        assert_eq!(pager.top(), 0);
        assert_eq!(pager.selected(), 0, "First visible row is selected");
        handle_event(pager, KeyCode::PageUp);
        assert_eq!(pager.selected(), 0, "First visible row is still selected");
        handle_event(pager, KeyCode::PageDown);
        assert_eq!(pager.selected(), 25, "First visible row should not change");
        handle_event(pager, KeyCode::PageUp);
        assert_eq!(pager.selected(), 0, "First visible row is selected");
    }

    #[test]
    fn selected() {
        let pager = &mut Paging::new(
            NonZeroUsize::new(25).unwrap(),
            NonZeroUsize::new(30).unwrap(),
        );
        assert_eq!(pager.selected(), 0, "Start with selection at position 0");
        handle_event(pager, KeyCode::Down);
        assert_eq!(pager.selected(), 1, "Next pos should be 1");
        handle_event(pager, KeyCode::Up);
        assert_eq!(pager.selected(), 0, "Next pos should be 1");
        handle_event(pager, KeyCode::Up);
        assert_eq!(pager.selected(), 0, "Position should not change");
        for _ in 0..10 {
            pager.select_next();
        }
        assert_eq!(
            pager.selected(),
            10,
            "Position should be one not existing on next page"
        );
        handle_event(pager, KeyCode::PageDown);
        assert_eq!(
            pager.selected(),
            29,
            "Position should be one not existing on next page"
        );
        handle_event(pager, KeyCode::PageUp);
        assert_eq!(pager.selected(), 4);
        for _ in 0..20 {
            pager.select_next();
        }
        assert_eq!(
            pager.selected(),
            pager.bottom(),
            "Last position on first page is selected"
        );
        handle_event(pager, KeyCode::Down);
        assert_eq!(
            pager.selected(),
            pager.bottom(),
            "Whole view scrolls one down"
        );
        assert_eq!(pager.top(), 1, "Whole view scrolls one down");
        assert_eq!(pager.bottom(), 25, "Whole view scrolls one down");
        for _ in 0..24 {
            pager.select_prev();
        }
        assert_eq!(
            pager.selected(),
            pager.top(),
            "The top visible row is selected"
        );
        assert_eq!(pager.top(), 1);
        handle_event(pager, KeyCode::Up);
        assert_eq!(pager.selected(), pager.top(), "Whole view scrolls one up");
        assert_eq!(pager.top(), 0, "Whole view scrolls one up");
        assert_eq!(pager.bottom(), 24, "Whole view scrolls one up");
    }

    #[test]
    fn page_height() {
        let pager = &mut Paging::new(
            NonZeroUsize::new(25).unwrap(),
            NonZeroUsize::new(30).unwrap(),
        );
        assert_eq!(pager.top(), 0);
        assert_eq!(pager.selected(), 0, "First visible row is selected");
        assert_eq!(pager.bottom(), 24);
        pager.page_height(
            NonZeroUsize::new(6).unwrap(),
            NonZeroUsize::new(30).unwrap(),
        );
        assert_eq!(pager.top(), 0, "Top did not change");
        assert_eq!(pager.selected(), 0, "First visible row is still selected");
        assert_eq!(pager.bottom(), 5, "Bottom shrinked");
    }

    #[test]
    fn move_selection_to_top() {
        let pager = &mut Paging::new(
            NonZeroUsize::new(10).unwrap(),
            NonZeroUsize::new(20).unwrap(),
        );
        assert_eq!(pager.selected(), 0, "Start with selection at position 0");
        assert_eq!(pager.top, 0);
        assert_eq!(pager.bottom, 9);
        pager.top = 4;
        pager.selected = 4;
        pager.bottom = 13;
        handle_event(pager, KeyCode::PageUp);
        assert_eq!(pager.top, 0);
        assert_eq!(pager.selected, 0);
        assert_eq!(pager.bottom, 9);
    }

    fn handle_event(pager: &mut Paging, code: KeyCode) {
        let event = Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        assert_eq!(pager.on_event(&event), HandleEvent::Handled);
    }
}
