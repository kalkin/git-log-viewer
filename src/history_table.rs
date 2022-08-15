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

use std::cmp::Ordering;
use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::StyledContent;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::history_adapter::HistoryAdapter;
use crate::history_entry::HistoryEntry;
use crate::search::highlight_search_line;
use crate::ui::base::data::{DataAdapter, SearchProgress};
use crate::ui::base::paging::Paging;
use crate::ui::base::{
    shorten_line, Area, Drawable, HandleEvent, Selectable, StyledArea, StyledLine,
};
use crate::ui::search::SearchWidget;
use std::sync::mpsc::Receiver;

#[derive(Copy, Clone)]
pub enum ColumnStyle {
    MaxWidth(usize),
    None,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Default)]
pub struct TableStyle(Vec<ColumnStyle>);

impl TableStyle {
    pub fn get(&self, col_number: usize) -> ColumnStyle {
        self.0
            .get(col_number)
            .map_or(ColumnStyle::None, |result| *result)
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct TableWidget {
    adapter: HistoryAdapter,
    style: TableStyle,
    paging: Paging,
    search_input: SearchWidget,
    search_progress_tx: Option<Receiver<SearchProgress>>,
}

impl TableWidget {
    pub fn new(adapter: HistoryAdapter) -> Self {
        let column_styles: Vec<ColumnStyle> = vec![
            ColumnStyle::None,
            ColumnStyle::None, // separator
            ColumnStyle::MaxWidth(20),
            ColumnStyle::None, // separator
            ColumnStyle::MaxWidth(12),
        ];
        let search_input = SearchWidget::default();
        Self {
            adapter,
            style: TableStyle(column_styles),
            paging: Paging::default(),
            search_input,
            search_progress_tx: None,
        }
    }
    pub fn default_action(&mut self) {
        self.adapter.default_action(self.paging.selected());
        self.paging.set_total_length(self.adapter.len());
    }
}

impl Drawable for TableWidget {
    fn render(&mut self, area: &Area) -> StyledArea<String> {
        let mut tmp: StyledArea<String> = vec![];
        let page_height = if self.search_input.is_visible() {
            area.height() - 1
        } else {
            area.height()
        };
        if let Some(needle) = self.search_input.search_value() {
            if !needle.text().is_empty() {
                let tx = self.adapter.search(needle, self.paging.selected());
                self.search_progress_tx = Some(tx);
            }
        }
        if let Some(responses) = &self.search_progress_tx {
            for progress in responses.try_iter() {
                self.search_input.consume(progress);
            }
        }
        self.paging.page_height(page_height, self.adapter.len());

        if let Some(result) = self.search_input.selected().as_ref() {
            let index = self.adapter.unfold_up_to(result);
            self.paging.set_total_length(self.adapter.len());
            self.paging.set_selected(index);
        }

        self.adapter.update();
        for i in self.paging.top()..=self.paging.bottom() {
            let line = self.adapter.get_line(i, i == self.paging.selected());
            tmp.push(line);
        }

        if tmp.len() < page_height {
            for _ in tmp.len()..page_height {
                tmp.push(StyledLine::empty());
            }
        }

        let mut max_column_widths = HashMap::new();
        {
            for (_, row) in tmp.iter().enumerate() {
                for (col_number, cell) in row.content.iter().enumerate() {
                    let text_len = UnicodeWidthStr::width(cell.content().as_str());
                    if let Some(max) = max_column_widths.get(&col_number) {
                        if text_len > *max {
                            max_column_widths.insert(col_number, text_len);
                        }
                    } else {
                        max_column_widths.insert(col_number, text_len);
                    }
                }
            }
        }

        let mut result = Vec::with_capacity(tmp.len());
        for row in tmp {
            let mut new_row = StyledLine {
                content: Vec::with_capacity(row.content.len()),
            };
            for (col_number, cell) in row.content.iter().enumerate() {
                match self.style.get(col_number) {
                    ColumnStyle::MaxWidth(style_max) => {
                        let mut max = *max_column_widths.get(&col_number).expect("max expected");
                        if max > style_max {
                            max = style_max;
                        }
                        let adjusted_content = adjust_string(cell.content(), max);
                        new_row
                            .content
                            .push(StyledContent::new(*cell.style(), adjusted_content));
                    }
                    ColumnStyle::None => {
                        new_row.content.push(cell.clone());
                    }
                }
            }

            result.push(shorten_line(new_row, area.width()));
        }

        if self.search_input.is_visible() {
            let mut new_result = Vec::with_capacity(result.len());
            for row in &mut result {
                new_result.push(highlight_search_line(row, &self.search_input.needle()));
            }
            new_result.push(self.search_input.render(area.width()));
            return new_result;
        }

        result
    }

    fn on_event(&mut self, event: Event) -> HandleEvent {
        match self.search_input.on_event(event) {
            HandleEvent::Handled => HandleEvent::Handled,
            HandleEvent::Ignored => match self.paging.on_event(event) {
                HandleEvent::Handled => HandleEvent::Handled,
                HandleEvent::Ignored => match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(' '),
                        modifiers: KeyModifiers::NONE,
                        ..
                    }) => {
                        self.default_action();
                        HandleEvent::Handled
                    }
                    _ => HandleEvent::Ignored,
                },
            },
        }
    }
}

impl Selectable<HistoryEntry> for TableWidget {
    fn selected_item(&mut self) -> &HistoryEntry {
        let tmp: &HistoryEntry = self.adapter.get_data(self.paging.selected());
        tmp
    }
}

// I'm not proud of this code. Ohh Omnissiah be merciful on my soul‼
fn adjust_string(text: &str, expected: usize) -> String {
    debug_assert!(expected > 0, "Minimal length should be 1");
    let length = unicode_width::UnicodeWidthStr::width(text);
    let mut result = String::from(text);
    match length.cmp(&expected) {
        Ordering::Less => {
            let actual = expected - length;
            for _ in 0..actual {
                result.push(' ');
            }
        }
        Ordering::Equal => {}
        Ordering::Greater => {
            result = "".to_owned();
            for w in text.unicode_words().collect::<Vec<&str>>() {
                let actual = UnicodeWidthStr::width(result.as_str()) + UnicodeWidthStr::width(w);
                if actual > expected {
                    break;
                }
                result.push_str(w);
                result.push(' ');
            }

            if result.is_empty() {
                let words = text.unicode_words().collect::<Vec<&str>>();
                result.push_str(words[0]);
            }

            let actual = UnicodeWidthStr::width(result.as_str());
            if actual > expected {
                let mut tmp = String::new();
                let mut i = 0;
                for g in result.as_str().graphemes(true) {
                    tmp.push_str(g);
                    i += 1;
                    if i == expected - 1 {
                        break;
                    }
                }
                result = tmp;
                result.push('…');
            } else {
                let end = expected - actual;
                for _ in 0..end {
                    result.push(' ');
                }
            }
        }
    }
    result
}
