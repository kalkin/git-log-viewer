use cursive::direction::Direction;
use cursive::event::{Event, EventResult, Key};
use cursive::theme::*;
use cursive::utils::span::{SpannedStr, SpannedString};
use cursive::{Printer, Rect, Vec2, XY};
use unicode_width::UnicodeWidthStr;

use posix_errors::PosixError;

use crate::core::*;
use crate::history_entry::{HistoryEntry, WidthConfig};
use crate::scroll::{MoveDirection, ScrollableSelectable};
use crate::search::{search_recursive, SearchDirection, SearchState};
use crate::style::DEFAULT_STYLE;
use git_subtrees_improved::{subtrees, SubtreeConfig};

pub struct History {
    range: String,
    history: Vec<Commit>,
    selected: usize,
    length: usize,
    working_dir: String,
    subtree_modules: Vec<SubtreeConfig>,
    search_state: SearchState,
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
        let search_state = SearchState::new(DEFAULT_STYLE.to_owned());
        Ok(History {
            range: range.to_string(),
            history,
            selected: 0,
            length,
            working_dir: working_dir.to_string(),
            subtree_modules,
            search_state,
        })
    }

    fn render_commit(&self, commit: &Commit, render_config: RenderConfig) -> SpannedString<Style> {
        let mut style = *DEFAULT_STYLE;
        if render_config.highlight {
            style.effects |= Effect::Reverse;
        }
        let mut buf = SpannedString::new();
        let width_config = WidthConfig {
            max_author: render_config.max_author,
            max_date: render_config.max_date,
            max_modules: modules_width(),
        };

        let sc = HistoryEntry::new(style, commit, &self.search_state, width_config);

        {
            buf.append(sc.id_span());
            buf.append_styled(" ", style);
        }

        {
            // Author date
            buf.append(sc.date_span());
            buf.append_styled(" ", style);
        }

        {
            // Author name
            buf.append(sc.name_span());
            buf.append_styled(" ", style);
        }

        buf.append_styled(commit.icon(), style);

        buf.append(sc.graph_span());
        buf.append_styled(" ", style);

        if let Some(modules) = sc.modules_span() {
            buf.append(modules);
            buf.append_styled(" ", style);
        }

        {
            buf.append(sc.subject_span());
            buf.append_styled(" ", style);
        }
        buf.append(sc.references_span());

        buf
    }

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

    fn toggle_folding(&mut self) {
        let pos = self.selected + 1;
        if self.selected_item().is_folded() {
            let children: Vec<Commit> = child_history(
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
    }

    fn render_history(&self, printer: &Printer) {
        let start = printer.content_offset.y;
        let end = start + printer.size.y;
        let configured_max_author = author_name_width();
        let configured_max_date = author_rel_date_width();
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
                buf = self.render_commit(commit, render_config);
                let t = SpannedStr::from(&buf);
                printer.print_styled((0, x), t);
            } else {
                break;
            }
        }
    }

    fn search_backward(&mut self) {
        for i in (0..self.selected).rev() {
            let c = self.history.get(i).unwrap();
            if c.search_matches(&self.search_state.needle, true) {
                let delta = self.selected - i;
                if delta > 0 {
                    self.move_focus(delta, MoveDirection::Up);
                    break;
                }
            }
        }
    }

    fn search_forward(&mut self) {
        let start = self.selected;
        let end = self.length;
        for i in start..end {
            let mut commit_option = self.history.get_mut(i);
            // Check if we need to fill_up data
            if commit_option.is_none() {
                if !self.fill_up(50) {
                    panic!("WTF?: No data to fill up during search")
                } else {
                    commit_option = self.history.get_mut(i);
                }
            }
            let c = commit_option.unwrap();
            if c.search_matches(&self.search_state.needle, true) {
                let delta = i - self.selected;
                if delta > 0 {
                    self.move_focus(delta, MoveDirection::Down);
                    return;
                }
            } else if c.is_merge() && c.is_folded() {
                if let Some((pos, mut commits)) = search_recursive(
                    &self.working_dir,
                    c,
                    &self.subtree_modules,
                    &self.search_state,
                ) {
                    c.folded(false);
                    let needle_position = i + pos;
                    let mut insert_position = i;
                    for c in commits.iter_mut() {
                        insert_position += 1;
                        self.history.insert(insert_position, c.to_owned());
                    }
                    let delta = needle_position - self.selected + 1;
                    if delta > 0 {
                        self.move_focus(delta, MoveDirection::Down);
                    }
                    return;
                }
            }
        }
    }

    fn fill_up(&mut self, max: usize) -> bool {
        let skip = self.history.len();
        let range = self.range.as_str();
        let working_dir = self.working_dir.as_str();
        let above_commit = self.history.last();
        if let Ok(mut tmp) = commits_for_range(
            working_dir,
            range,
            0,
            above_commit,
            self.subtree_modules.as_ref(),
            vec![],
            Some(skip),
            Some(max),
        ) {
            let result = !tmp.is_empty();
            self.history.append(tmp.as_mut());
            return result;
        }
        false
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
        self.render_history(printer)
    }

    fn layout(&mut self, size: Vec2) {
        // Always prefetch commits for one page from selected
        let end = self.selected + size.y;
        if end >= self.history.len() - 1 && end < self.length {
            let max = end + 1 - self.history.len();
            self.fill_up(max);
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
            Event::Char('h') | Event::Key(Key::Left) => {
                if self.selected_item().is_merge() && !self.selected_item().is_folded() {
                    self.toggle_folding();
                } else if self.selected_item().level() > 0 {
                    // move to last parent node
                    let mut cur = self.selected;
                    let expected_level = self.selected_item().level() - 1;
                    for c in self.history[0..cur].iter().rev() {
                        if c.level() == expected_level {
                            break;
                        }
                        cur -= 1;
                    }
                    self.move_focus(self.selected - cur + 1, MoveDirection::Up);
                } else {
                    // move to last merge
                    let mut cur = self.selected;
                    for c in self.history[0..cur].iter().rev() {
                        if c.is_merge() {
                            break;
                        }
                        cur -= 1;
                    }
                    self.move_focus(self.selected - cur + 1, MoveDirection::Up);
                }
                EventResult::Consumed(None)
            }
            Event::Char('l') | Event::Key(Key::Right) => {
                if self.selected_item().is_merge() && self.selected_item().is_folded() {
                    self.toggle_folding()
                } else {
                    let mut cur = self.selected;
                    for c in self.history[cur + 1..].iter() {
                        if c.is_merge() {
                            break;
                        }
                        cur += 1;
                    }
                    self.move_focus(cur - self.selected + 1, MoveDirection::Down);
                }

                EventResult::Consumed(None)
            }
            Event::Char(' ') => {
                if self.selected_item().is_merge() {
                    self.toggle_folding();
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            _ => EventResult::Ignored,
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

    fn search(&mut self, search_state: SearchState) {
        self.search_state = search_state;
        match self.search_state.direction {
            SearchDirection::Forward => self.search_forward(),
            SearchDirection::Backward => self.search_backward(),
        }
    }

    fn selected_pos(&self) -> usize {
        self.selected
    }

    fn selected_item(&self) -> &Commit {
        self.history.get(self.selected).as_ref().unwrap()
    }
}
