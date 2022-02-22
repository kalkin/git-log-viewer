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
use std::io::Write;

use crossterm::cursor::{Hide, MoveDown, MoveTo, MoveToColumn, Show};
use crossterm::event::Event;
use crossterm::style::{PrintStyledContent, StyledContent};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, Clear, ClearType::FromCursorDown,
    EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use crossterm::Result;
use crossterm::{execute, queue};
use unicode_width::UnicodeWidthStr;

pub use data::DataAdapter;
pub use data::VecAdapter;

pub use crate::ui::base::area::Area;
pub use crate::ui::list::ListWidget;

mod area;
pub mod data;
pub mod paging;
pub mod search;
#[cfg(test)]
#[cfg(not(tarpaulin_include))]
pub mod test_helpers;

pub type Height = usize;
pub type Pos = usize;

#[derive(Eq, PartialEq, Debug)]
pub enum HandleEvent {
    Handled,
    Ignored,
}

pub type StyledLine<D> = Vec<StyledContent<D>>;
pub type StyledArea<D> = Vec<StyledLine<D>>;

pub trait Drawable {
    fn render(&mut self, area: &Area) -> StyledArea<String>;
    fn on_event(&mut self, event: Event) -> HandleEvent;
}

pub trait Selectable<T> {
    /// Return the currently selected item
    fn selected_item(&mut self) -> &T;
}

#[allow(clippy::ptr_arg)]
/// Renders ui to `stdout`
///
/// # Panics
///
/// When vector len does not match the `area.height` or any [`StyledContent`] is wider then `area.width`.
///
/// # Errors
///
/// Returns an error when something goes wrong
pub fn render(lines: &StyledArea<String>, area: &Area) -> Result<()> {
    let mut stdout = std::io::stdout();

    // Validate data {
    if area.height() < lines.len() {
        panic!(
            "Height does not match expected: {} got: {}",
            area.height(),
            lines.len()
        );
    }

    for rows in lines {
        let width = rows
            .iter()
            .map(|x| UnicodeWidthStr::width(x.content().as_str()))
            .sum::<usize>();
        if area.width() < width {
            panic!(
                "Width does not match expected: {} got: {}",
                area.width(),
                width
            );
        }
    }
    // End validate data }

    queue!(stdout, MoveTo(0, 0), Clear(FromCursorDown))?;

    for line in lines {
        for x in line.iter().cloned().map(PrintStyledContent) {
            queue!(stdout, x)?;
        }
        queue!(stdout, MoveDown(1), MoveToColumn(1))?;
    }

    stdout.flush()?;
    Ok(())
}

/// Run this before starting rendering
///
/// # Errors
///
/// Returns an error when something goes wrong
pub fn setup_screen(title: &str) -> Result<()> {
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(stdout, Hide)?;
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, SetTitle(title))?;
    if let Err(e) = stdout.flush() {
        return Err(e);
    }
    Ok(())
}

/// Run this before shutdown
///
/// # Errors
///
/// Returns an error when something goes wrong
pub fn shutdown_screen() -> Result<()> {
    let mut stdout = std::io::stdout();
    execute!(stdout, Show)?;
    execute!(stdout, SetTitle(""))?;
    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    if let Err(e) = stdout.flush() {
        return Err(e);
    }
    Ok(())
}

#[must_use]
pub fn new_area() -> Area {
    Area::from(size().expect("An area"))
}

#[must_use]
#[allow(clippy::ptr_arg)]
pub fn line_length(line: &StyledLine<String>) -> usize {
    line.iter().map(content_length).sum()
}

#[must_use]
pub fn content_length(styled_content: &StyledContent<String>) -> usize {
    UnicodeWidthStr::width(styled_content.content().as_str())
}

#[must_use]
pub fn shorten_line(line: StyledLine<String>, width: usize) -> StyledLine<String> {
    let mut result: StyledLine<String> = vec![];
    let mut i = 0;
    for styled_content in line {
        let length = i + content_length(&styled_content);
        match length.cmp(&width) {
            Ordering::Less => {
                result.push(styled_content.clone());
                i = length;
            }
            Ordering::Equal => {
                result.push(styled_content);
                break;
            }
            Ordering::Greater => {
                use unicode_truncate::UnicodeTruncateStr;
                let size = width - i;
                let (text, _) = styled_content.content().unicode_truncate(size);
                let style = *styled_content.style();
                let content = StyledContent::new(style, text.to_owned());
                result.push(content);
                break;
            }
        }
    }

    result
}
