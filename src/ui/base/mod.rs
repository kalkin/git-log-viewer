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
use crossterm::{execute, queue, ErrorKind};
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
        )
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
            )
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
        return Err(ErrorKind::from(e));
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
        return Err(ErrorKind::from(e));
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
    line.iter().map(|sc| content_length(sc)).sum()
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
                let content = StyledContent::new(style, text.to_string());
                result.push(content);
                break;
            }
        }
    }

    result
}
