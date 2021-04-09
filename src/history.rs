use cursive::direction::Direction;
use cursive::theme::*;
use cursive::utils::span::{SpannedStr, SpannedString};
use cursive::{Rect, Vec2, XY};
use unicode_width::UnicodeWidthStr;

use glv_core::*;
use posix_errors::PosixError;

use monorepo::SubtreeConfig;

use crate::scroll::{MoveDirection, ScrollableSelectable};

pub struct History {
    range: String,
    history: Vec<Commit>,
    selected: usize,
    length: usize,
    working_dir: String,
}

impl History {
    pub fn new(working_dir: &str, range: &str) -> Result<History, PosixError> {
        let history = commits_for_range(working_dir, range, 0, None, vec![], Some(0), Some(25))?;
        let length = history_length(working_dir, range, vec![])?;
        assert!(!history.is_empty());
        Ok(History {
            range: range.to_string(),
            history,
            selected: 0,
            length,
            working_dir: working_dir.to_string(),
        })
    }

    fn render_commit(
        commit: &Commit,
        default_style: Style,
        max_author: usize,
        max_date: usize,
    ) -> SpannedString<Style> {
        let mut buf = SpannedString::new();
        let id_style = id_style(&default_style);
        let date_style = date_style(&default_style);
        buf.append_styled(commit.short_id(), id_style);
        buf.append_styled(" ", default_style);
        {
            buf.append_styled(commit.author_rel_date(), date_style);
            let date_len = UnicodeWidthStr::width(commit.author_rel_date().as_str());
            if date_len < max_date {
                let result = max_date - date_len;
                for _ in 0..result {
                    buf.append_styled(" ", date_style)
                }
            }
        }
        buf.append_styled(" ", default_style);
        {
            let name_style = name_style(&default_style);
            buf.append_styled(commit.author_name(), name_style);
            if commit.author_name().len() <= max_author {
                let author_len = UnicodeWidthStr::width(commit.author_name().as_str());
                let result = max_author - author_len;
                for _ in 0..result {
                    buf.append_styled(" ", name_style)
                }
            }
        }
        buf.append_styled(" ", default_style);
        buf.append_styled(commit.subject(), default_style);
        buf.append_styled(" ", default_style);
        for r in commit.references() {
            buf.append_styled(r.to_string(), ref_style(&default_style));
        }
        buf
    }
}

fn id_style(default: &Style) -> Style {
    let mut id_style = default.clone();
    id_style.color = ColorStyle::new(Color::Dark(BaseColor::Magenta), Color::TerminalDefault);
    id_style
}

fn date_style(default: &Style) -> Style {
    let mut date_style = default.clone();
    date_style.color = ColorStyle::new(Color::Dark(BaseColor::Blue), Color::TerminalDefault);
    date_style
}

fn name_style(default: &Style) -> Style {
    let mut name_style = default.clone();
    name_style.color = ColorStyle::new(Color::Dark(BaseColor::Green), Color::TerminalDefault);
    name_style
}

fn ref_style(default: &Style) -> Style {
    let mut ref_style = default.clone();
    ref_style.color = ColorStyle::new(Color::Dark(BaseColor::Yellow), Color::TerminalDefault);
    ref_style
}

impl cursive::view::View for History {
    fn draw(&self, printer: &cursive::Printer) {
        let default_style: Style = Style {
            color: ColorStyle::terminal_default(),
            ..Default::default()
        };
        assert!(
            printer.content_offset.y <= self.selected
                && self.selected < printer.content_offset.y + printer.size.y,
            "Wrong `draw()` call. Selected '{}' is not visible",
            self.selected
        );
        let start = printer.content_offset.y;
        let end = start + printer.size.y;
        let (max_author, max_date) = self.calc_max_name_date(end);

        for x in start..end {
            if let Some(commit) = self.history.get(x) {
                let buf;
                if x == self.selected {
                    let mut hl_style = default_style;
                    hl_style.effects |= Effect::Reverse;
                    buf = History::render_commit(commit, hl_style, max_author, max_date);
                } else {
                    buf = History::render_commit(commit, default_style, max_author, max_date);
                }
                let t = SpannedStr::from(&buf);
                printer.print_styled((0, x), t);
            } else {
                break;
            }
        }
    }

    fn layout(&mut self, size: Vec2) {
        // Always prefetch commits for one page from selected
        let end = self.selected + size.y;
        if end >= self.history.len() - 1 && end < self.length {
            let max = end + 1 - self.history.len();
            let skip = self.history.len();
            let range = self.range.as_str();
            let working_dir = self.working_dir.as_str();
            let above_commit = self.history.last();
            let mut tmp = commits_for_range(
                working_dir,
                range,
                0,
                above_commit,
                vec![],
                Some(skip),
                Some(max),
            )
            .unwrap();
            self.history.append(tmp.as_mut());
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        Vec2 {
            x: constraint.x,
            y: 200,
        }
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        true
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        Rect::from_size(
            XY {
                x: 0,
                y: self.selected,
            },
            XY {
                x: view_size.x,
                y: self.selected,
            },
        )
    }
}

impl ScrollableSelectable for History {
    fn length(&self) -> usize {
        self.length
    }

    fn move_focus(&mut self, n: usize, source: MoveDirection) -> bool {
        if source == MoveDirection::Up && self.selected == 0 {
            false
        } else if source == MoveDirection::Up {
            if self.selected < n {
                self.selected = 0;
            } else {
                self.selected -= n;
            }
            true
        } else if source == MoveDirection::Down && self.selected == self.length() - 1 {
            false
        } else if source == MoveDirection::Down {
            if self.selected + n >= self.length() {
                self.selected = self.length() - 1;
            } else {
                self.selected += n;
            }
            true
        } else {
            false
        }
    }
    fn selected_pos(&self) -> usize {
        self.selected
    }

    fn selected_item(&self) -> &Commit {
        self.history.get(self.selected).as_ref().unwrap()
    }
}

impl History {
    fn calc_max_name_date(&self, height: usize) -> (usize, usize) {
        let mut max_author = 5;
        let mut max_date = 5;
        {
            let mut iter = self.history.iter();
            for _ in 0..height {
                if let Some(commit) = iter.next() {
                    if commit.author_rel_date().len() > max_date {
                        let t = commit.author_rel_date().as_str();
                        max_date = UnicodeWidthStr::width(t);
                    }
                    if commit.author_name().len() > max_author {
                        let t = commit.author_name().as_str();
                        max_author = UnicodeWidthStr::width(t);
                    }
                } else {
                    break;
                }
            }
        }
        (max_author, max_date)
    }
}
