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

use crossterm::event::Event;

use crate::search::highlight_search_line;
use crate::ui::base::data::{DataAdapter, SearchProgress};
use crate::ui::base::paging::Paging;
use crate::ui::base::{shorten_line, Area, Drawable, HandleEvent, Selectable, StyledArea};
use crate::ui::search::SearchWidget;
use std::sync::mpsc::Receiver;

use super::base::StyledLine;

#[allow(clippy::module_name_repetitions)]
pub struct ListWidget<T> {
    adapter: Box<dyn DataAdapter<T>>,
    paging: Paging,
    search_input: SearchWidget,
    search_progress_tx: Option<Receiver<SearchProgress>>,
}

impl<T> ListWidget<T> {
    #[must_use]
    pub fn new(adapter: Box<dyn DataAdapter<T>>) -> Self {
        let search_input = SearchWidget::default();
        Self {
            adapter,
            paging: Paging::default(),
            search_input,
            search_progress_tx: None,
        }
    }

    fn highlight_search(&self, input: &mut StyledArea<String>) -> StyledArea<String> {
        let mut new_result = Vec::with_capacity(input.len());
        let search_state = self.search_input.needle();
        for row in input {
            new_result.push(highlight_search_line(row, &search_state));
        }
        new_result
    }
}

impl<T> Selectable<T> for ListWidget<T> {
    fn selected_item(&mut self) -> &T {
        self.adapter.get_data(self.paging.selected())
    }
}

impl<T> Drawable for ListWidget<T> {
    fn render(&mut self, area: &Area) -> StyledArea<String> {
        let mut result: StyledArea<String> = vec![];
        #[allow(clippy::arithmetic)]
        // arithmetic: we assume that `height >= 4`.
        let page_height = if self.search_input.is_visible() {
            area.height() - 1
        } else {
            area.height()
        };
        if let Some(needle) = self.search_input.search_value() {
            let tx = self.adapter.search(needle, self.paging.selected());
            self.search_progress_tx = Some(tx);
        }
        if let Some(responses) = &self.search_progress_tx {
            for progress in responses.try_iter() {
                self.search_input.consume(progress);
            }
        }
        self.paging.page_height(page_height, self.adapter.len());
        if let Some(selected) = self.search_input.selected() {
            self.paging.set_selected(selected.0[0]);
        }

        for i in self.paging.top()..=self.paging.bottom() {
            let line = self.adapter.get_line(i, i == self.paging.selected());
            result.push(shorten_line(line, area.width()));
        }

        if result.len() < page_height {
            for _ in result.len()..page_height {
                result.push(StyledLine::empty());
            }
        }

        if self.search_input.is_visible() {
            result = self.highlight_search(&mut result);
            result.push(self.search_input.render(area.width()));
        }
        result
    }

    fn on_event(&mut self, event: &Event) -> HandleEvent {
        match self.search_input.on_event(event) {
            HandleEvent::Handled => HandleEvent::Handled,
            HandleEvent::Ignored => self.paging.on_event(event),
        }
    }
}

#[cfg(test)]
mod test_list_widget {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use pretty_assertions::assert_eq;

    use crate::ui::base::{Area, Drawable, HandleEvent, ListWidget};
    use crate::ui::list::example_content;

    #[test]
    fn render_short_list() {
        let list = &mut example_content();
        let expected = 35;
        let area = Area::new(80, expected);
        let actual = list.render(&area).len();
        assert_eq!(
            actual, expected,
            "Extend rendered content to fill the screen"
        );
    }

    #[test]
    fn search_visibility() {
        let list = &mut example_content();
        let area = Area::new(80, 25);
        {
            let rendered = list.render(&area);
            let prefix = rendered
                .last()
                .expect("last line")
                .content
                .first()
                .expect("first styled content");
            assert_ne!(prefix.content(), "/");
        }

        handle_event(list, KeyCode::Char('/'));
        {
            let rendered = list.render(&area);
            let prefix = rendered
                .last()
                .expect("last line")
                .content
                .first()
                .expect("first styled content");
            assert_eq!(prefix.content(), "/");
        }

        handle_event(list, KeyCode::Esc);
        handle_event(list, KeyCode::Char('?'));
        {
            let rendered = list.render(&area);
            let prefix = rendered
                .last()
                .expect("last line")
                .content
                .first()
                .expect("first styled content");
            assert_eq!(prefix.content(), "?");
        }
    }

    fn handle_event(pager: &mut ListWidget<String>, code: KeyCode) {
        let event = Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        assert_eq!(pager.on_event(&event), HandleEvent::Handled);
    }
}
#[cfg(test)]
fn example_content() -> ListWidget<String> {
    use crate::ui::base::test_helpers::lore_ipsum_lines;
    let adapter = crate::ui::base::VecAdapter::new(lore_ipsum_lines(30));
    let search_input = SearchWidget::default();
    let paging = Paging::new(25, 30);
    ListWidget {
        adapter: Box::new(adapter),
        paging,
        search_input,
        search_progress_tx: None,
    }
}
