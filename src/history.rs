use cursive::direction::Direction;
use cursive::event::{Event, EventResult};
use cursive::theme::*;
use cursive::utils::span::{SpannedStr, SpannedString};
use cursive::{Rect, Vec2, XY};
use unicode_width::UnicodeWidthStr;

use posix_errors::PosixError;

use crate::scroll::{MoveDirection, ScrollableSelectable};
use crate::style::{date_style, id_style, name_style, ref_style};
use crate::style::{mod_style, DEFAULT_STYLE};
use git_subtrees_improved::{subtrees, SubtreeConfig};
use glv_core::*;

pub struct History {
    range: String,
    history: Vec<Commit>,
    selected: usize,
    length: usize,
    working_dir: String,
    subtree_modules: Vec<SubtreeConfig>,
}

struct RenderConfig {
    max_author: usize,
    max_date: usize,
    highlight: bool,
}

impl History {
    pub fn new(working_dir: &str, range: &str) -> Result<History, PosixError> {
        let subtree_modules = subtrees(working_dir)?;
        let history = commits_for_range(
            working_dir,
            range,
            0,
            None,
            subtree_modules.as_ref(),
            vec![],
            Some(0),
            Some(25),
        )?;
        let length = history_length(working_dir, range, vec![])?;
        assert!(!history.is_empty());
        Ok(History {
            range: range.to_string(),
            history,
            selected: 0,
            length,
            working_dir: working_dir.to_string(),
            subtree_modules,
        })
    }

    fn render_commit(commit: &Commit, render_config: RenderConfig) -> SpannedString<Style> {
        let mut style = *DEFAULT_STYLE;
        if render_config.highlight {
            style.effects |= Effect::Reverse;
        }
        let mut buf = SpannedString::new();
        let id_style = id_style(&style);
        let name_style = name_style(&style);
        let date_style = date_style(&style);
        let mod_style = mod_style(&style);

        buf.append_styled(commit.short_id(), id_style);
        buf.append_styled(" ", style);

        {
            let date = glv_core::adjust_string(commit.author_rel_date(), render_config.max_date);
            buf.append_styled(date, date_style);
        }
        buf.append_styled(" ", style);

        {
            let name = glv_core::adjust_string(commit.author_name(), render_config.max_author);
            buf.append_styled(name, name_style);
        }
        buf.append_styled(" ", style);
        buf.append_styled(commit.icon(), style);

        for _ in 0..commit.level() {
            buf.append_styled("│ ", style)
        }
        if commit.bellow().is_none() {
            buf.append_styled("◉", style)
        } else if commit.is_commit_link() {
            buf.append_styled("⭞", style)
        } else {
            buf.append_styled("●", style)
        }

        if commit.is_merge() {
            if commit.subject().starts_with("Update :") || commit.subject().contains(" Import ") {
                if commit.is_fork_point() {
                    buf.append_styled("⇤┤", style);
                } else {
                    buf.append_styled("⇤╮", style);
                }
            } else if commit.is_fork_point() {
                buf.append_styled("─┤", style);
            } else {
                buf.append_styled("─┐", style)
            }
        } else if commit.is_fork_point() {
            buf.append_styled("─┘", style)
        }
        buf.append_styled(" ", style);

        if !commit.subtree_modules().is_empty() {
            let mut modules_text: String = ":".to_string();
            let subtree_modules = commit.subtree_modules();
            modules_text.push_str(&subtree_modules.join(" :"));
            if modules_text.width() > modules_width() {
                modules_text = format!("({} modules)", subtree_modules.len());
            }
            buf.append_styled(modules_text, mod_style);
            buf.append_styled(" ", style);
        } else if let Some(v) = commit.subject_module() {
            buf.append_styled(v, mod_style);
            buf.append_styled(" ", style);
        }

        if let Some(subject) = commit.short_subject() {
            buf.append_styled(subject, style);
        } else {
            buf.append_styled(commit.subject(), style);
        }
        buf.append_styled(" ", style);
        for r in commit.references() {
            buf.append_styled("«", ref_style(&style));
            buf.append_styled(r.to_string(), ref_style(&style));
            buf.append_styled("» ", ref_style(&style));
        }
        buf
    }
}

impl cursive::view::View for History {
    fn draw(&self, printer: &cursive::Printer) {
        assert!(
            printer.content_offset.y <= self.selected
                && self.selected < printer.content_offset.y + printer.size.y,
            "Wrong `draw()` call. Selected '{}' is not visible",
            self.selected
        );
        let start = printer.content_offset.y;
        let end = start + printer.size.y;
        let configured_max_author = glv_core::author_name_width();
        let configured_max_date = glv_core::author_rel_date_width();
        let (mut max_author, mut max_date) = self.calc_max_name_date(end);
        if configured_max_author != 0 && max_author > configured_max_author {
            max_author = configured_max_author;
        }

        if configured_max_date != 0 && max_date > configured_max_date {
            max_date = configured_max_date;
        }

        for x in start..end {
            if let Some(commit) = self.history.get(x) {
                let buf;
                let render_config = RenderConfig {
                    max_author,
                    max_date,
                    highlight: x == self.selected,
                };
                buf = History::render_commit(commit, render_config);
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
                self.subtree_modules.as_ref(),
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
                        let children: Vec<Commit> = glv_core::child_history(
                            &self.working_dir,
                            self.selected_item(),
                            self.subtree_modules.as_ref(),
                        );
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
