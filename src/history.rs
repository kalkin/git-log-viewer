use cursive::direction::Direction;
use cursive::event::{Event, EventResult};
use cursive::theme::*;
use cursive::utils::span::{SpannedStr, SpannedString};
use cursive::{Rect, Vec2, XY};
use lazy_static::lazy_static;
use regex::Regex;
use unicode_width::UnicodeWidthStr;

use glv_core::*;
use posix_errors::PosixError;

use monorepo::SubtreeConfig;

use crate::scroll::{MoveDirection, ScrollableSelectable};

// const icons: Vec<Regex> = vec![Regex::new(r"^Revert:?\s*").unwrap()];

lazy_static! {
    static ref REGEXES: Vec<(Regex, &'static str)> = vec![
        (Regex::new(r"(?i)^Revert:?\s*").unwrap(), " "),
        (Regex::new(r"(?i)^archive:?\s*").unwrap(), "\u{f53b} "),
        (Regex::new(r"(?i)^issue:?\s*").unwrap(), "\u{f145} "),
        (Regex::new(r"(?i)^BREAKING CHANGE:?\s*").unwrap(), "⚠ "),
        (Regex::new(r"(?i)^fixup!\s+").unwrap(), "\u{f0e3} "),
        (Regex::new(r"(?i)^ADD:\s?[a-z0-9]+").unwrap(), " "),
        (Regex::new(r"(?i)^ref(actor)?:?\s*").unwrap(), "↺ "),
        (Regex::new(r"(?i)^lang:?\s*").unwrap(), "\u{fac9}"),
        (Regex::new(r"(?i)^deps(\(.+\))?:?\s*").unwrap(), "\u{f487} "),
        (Regex::new(r"(?i)^config:?\s*").unwrap(), "\u{f462} "),
        (Regex::new(r"(?i)^test(\(.+\))?:?\s*").unwrap(), "\u{f45e} "),
        (Regex::new(r"(?i)^ci(\(.+\))?:?\s*").unwrap(), "\u{f085} "),
        (Regex::new(r"(?i)^perf(\(.+\))?:?\s*").unwrap(), "\u{f9c4}"),
        (
            Regex::new(r"(?i)^(bug)?fix(ing|ed)?(\(.+\))?[/:\s]+").unwrap(),
            "\u{f188} "
        ),
        (Regex::new(r"(?i)^doc(s|umentation)?:?\s*").unwrap(), "✎ "),
        (Regex::new(r"(?i)^improvement:?\s*").unwrap(), "\u{e370} "),
        (Regex::new(r"(?i)^CHANGE/?:?\s*").unwrap(), "\u{e370} "),
        (Regex::new(r"(?i)^hotfix:?\s*").unwrap(), "\u{f490} "),
        (Regex::new(r"(?i)^feat:?\s*").unwrap(), "➕"),
        (Regex::new(r"(?i)^add:?\s*").unwrap(), "➕"),
        (
            Regex::new(r"(?i)^(release|bump):?\s*").unwrap(),
            "\u{f412} "
        ),
        (Regex::new(r"(?i)^build:?\s*").unwrap(), "🔨"),
        (Regex::new(r"(?i).*\bchangelog\b.*").unwrap(), "✎ "),
        (Regex::new(r"(?i)^refactor:?\s*").unwrap(), "↺ "),
        (Regex::new(r"(?i)^.* Import .*").unwrap(), "⮈ "),
        (Regex::new(r"(?i)^Split .*").unwrap(), "\u{f403} "),
        (Regex::new(r"(?i)^Remove:?\s+.*").unwrap(), "\u{f48e} "),
        (Regex::new(r"(?i)^Update :\w+.*").unwrap(), "\u{f419} "),
        (Regex::new(r"(?i)^style:?\s*").unwrap(), "♥ "),
        (Regex::new(r"(?i)^DONE:?\s?[a-z0-9]+").unwrap(), "\u{f41d} "),
        (Regex::new(r"(?i)^rename?\s*").unwrap(), "\u{f044} "),
        (Regex::new(r"(?i).*").unwrap(), "  "),
    ];
}

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
        for (reg, icon) in REGEXES.iter() {
            if reg.is_match(commit.subject()) {
                buf.append_styled(icon.to_string(), default_style);
                break;
            }
        }
        buf.append_styled(" ", default_style);
        for _ in 0..commit.level() {
            buf.append_styled("│ ", default_style)
        }
        if commit.bellow().is_none() {
            buf.append_styled("◉", default_style)
        } else if commit.is_commit_link() {
            buf.append_styled("⭞", default_style)
        } else {
            buf.append_styled("●", default_style)
        }

        if commit.is_merge() {
            if commit.subject().starts_with("Update :") || commit.subject().contains(" Import ") {
                if commit.is_fork_point() {
                    buf.append_styled("⇤┤", default_style);
                } else {
                    buf.append_styled("⇤╮", default_style);
                }
            } else if commit.is_fork_point() {
                buf.append_styled("─┤", default_style);
            } else {
                buf.append_styled("─┐", default_style)
            }
        } else if commit.is_fork_point() {
            buf.append_styled("─┘", default_style)
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

    fn on_event(&mut self, e: Event) -> EventResult {
        match e {
            Event::Char(' ') => {
                if self.selected_item().is_merge() {
                    let pos = self.selected + 1;
                    if self.selected_item().is_folded() {
                        let children: Vec<Commit> =
                            glv_core::child_history(&self.working_dir, self.selected_item());
                        for (i, c) in children.iter().cloned().enumerate() {
                            self.history.insert(pos + i, c);
                        }
                    } else {
                        while let Some(c) = self.history.get(pos) {
                            if c.level() > self.selected_item().level() {
                                self.history.remove(pos);
                            } else {
                                break;
                            }
                        }
                    }
                    let cur = self.history.get_mut(self.selected).unwrap();
                    cur.folded(!cur.is_folded());

                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            _ => {
                log::warn!("History: Unexpected key {:?}", e);
                EventResult::Ignored
            }
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
