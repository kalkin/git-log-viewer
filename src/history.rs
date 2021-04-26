use std::borrow::Borrow;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use cursive::direction::Direction;
use cursive::event::{Event, EventResult, Key};
use cursive::theme::*;
use cursive::utils::span::{SpannedStr, SpannedString};
use cursive::{Printer, Rect, Vec2, XY};
use unicode_width::UnicodeWidthStr;

use git_subtrees_improved::{subtrees, SubtreeConfig};
use git_wrapper::is_ancestor;
use posix_errors::PosixError;

use crate::core::*;
use crate::fork_point::{ForkPointRequest, ForkPointResponse, ForkPointThread};
use crate::history_entry::{HistoryEntry, WidthConfig};
use crate::scroll::{MoveDirection, ScrollableSelectable};
use crate::search::{search_link_recursive, SearchDirection, SearchState};
use crate::style::DEFAULT_STYLE;

pub struct History {
    range: String,
    history: Vec<HistoryEntry>,
    selected: usize,
    length: usize,
    working_dir: String,
    subtree_modules: Vec<SubtreeConfig>,
    search_state: SearchState,
    paths: Vec<String>,
    fork_point_thread: ForkPointThread,
}

struct RenderConfig {
    max_author: usize,
    max_date: usize,
    highlight: bool,
}

impl History {
    pub fn new(working_dir: &str, range: &str, paths: Vec<String>) -> Result<History, PosixError> {
        let subtree_modules = subtrees(working_dir)?;
        let length = history_length(working_dir, range, vec![])?;
        let search_state = SearchState::new(DEFAULT_STYLE.to_owned());
        let fork_point_thread = ForkPointThread::new();

        Ok(History {
            range: range.to_string(),
            history: vec![],
            selected: 0,
            length,
            working_dir: working_dir.to_string(),
            subtree_modules,
            search_state,
            paths,
            fork_point_thread,
        })
    }

    fn render_commit(
        &self,
        entry: &HistoryEntry,
        render_config: RenderConfig,
    ) -> SpannedString<Style> {
        let mut style = *DEFAULT_STYLE;
        if render_config.highlight {
            style.effects |= Effect::Reverse;
        }
        let search_state = if self.search_state.active {
            Some(&self.search_state)
        } else {
            None
        };

        let width_config = WidthConfig {
            max_author: render_config.max_author,
            max_date: render_config.max_date,
            max_modules: modules_width(),
        };

        entry.render(search_state, width_config)
    }

    fn calc_max_name_date(&self, height: usize) -> (usize, usize) {
        let mut max_author = 5;
        let mut max_date = 5;
        {
            let mut iter = self.history.iter();
            for _ in 0..height {
                if let Some(entry) = iter.next() {
                    if entry.commit().author_rel_date().len() > max_date {
                        let t = entry.commit().author_rel_date().as_str();
                        max_date = UnicodeWidthStr::width(t);
                    }
                    if entry.commit().author_name().len() > max_author {
                        let t = entry.commit().author_name().as_str();
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
        if self.selected_entry().is_folded() {
            let children: Vec<Commit> = child_history(
                &self.working_dir,
                self.selected_commit(),
                self.subtree_modules.as_ref(),
            );
            let mut above_commit = Some(self.selected_commit());
            for (i, c) in children.iter().cloned().enumerate() {
                if above_commit.is_some()
                    && above_commit.unwrap().is_merge()
                    && c.fork_points_calculation_needed()
                {
                    self.fork_point_thread.send(ForkPointRequest {
                        first: c.id().clone(),
                        second: above_commit.unwrap().children().first().unwrap().clone(),
                        working_dir: self.working_dir.clone(),
                    });
                }
                let entry = HistoryEntry::new(
                    self.working_dir.clone(),
                    c,
                    self.selected_entry().level() + 1,
                    &self.subtree_modules,
                );
                self.history.insert(pos + i, entry);
                above_commit = Some(self.history.get(pos + i).unwrap().commit());
            }
        } else {
            while let Some(e) = self.history.get(pos) {
                if e.level() > self.selected_entry().level() {
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
            if let Some(entry) = self.history.get(x) {
                let buf;
                let render_config = RenderConfig {
                    max_author,
                    max_date,
                    highlight: x == self.selected,
                };
                buf = self.render_commit(entry, render_config);
                let t = SpannedStr::from(&buf);
                printer.print_styled((0, x), t);
            } else {
                break;
            }
        }
    }

    fn search_backward(&mut self) {
        for i in (0..self.selected).rev() {
            let e = self.history.get(i).unwrap();
            if e.search_matches(&self.search_state.needle, true) {
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
            let mut commit_option = self.history.get(i);
            // Check if we need to fill_up data
            if commit_option.is_none() {
                if !self.fill_up(50) {
                    panic!("WTF?: No data to fill up during search")
                } else {
                    commit_option = self.history.get(i);
                }
            }
            let e = commit_option.unwrap();
            if e.search_matches(&self.search_state.needle, true) {
                let delta = i - self.selected;
                if delta > 0 {
                    self.move_focus(delta, MoveDirection::Down);
                    return;
                }
            } else if e.is_merge() && e.is_folded() {
                let x = self.search_recursive(e);
                if let Some((pos, mut entries)) = x {
                    self.history.get_mut(i).unwrap().folded(false);
                    let needle_position = i + pos;
                    let mut insert_position = i;
                    for entry in entries.into_iter() {
                        insert_position += 1;
                        self.history.insert(insert_position, entry);
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

    fn search_recursive(&self, entry: &HistoryEntry) -> Option<(usize, Vec<HistoryEntry>)> {
        assert!(entry.is_merge(), "Expected a merge commit");
        let level = entry.level() + 1;
        let children = child_history(
            &self.working_dir,
            entry.commit(),
            self.subtree_modules.borrow(),
        );

        let mut above_commit = Some(entry.commit());
        let mut entries: Vec<HistoryEntry> = vec![];
        for c in children.into_iter() {
            if above_commit.is_some()
                && above_commit.unwrap().is_merge()
                && c.fork_points_calculation_needed()
            {
                self.fork_point_thread.send(ForkPointRequest {
                    first: c.id().clone(),
                    second: above_commit.unwrap().children().first().unwrap().clone(),
                    working_dir: self.working_dir.clone(),
                });
            }
            let e = HistoryEntry::new(self.working_dir.clone(), c, level, &self.subtree_modules);
            entries.push(e);
            above_commit = Some(entries.last().unwrap().commit());
        }
        for (i, e) in entries.iter_mut().enumerate() {
            if e.search_matches(&self.search_state.needle, true) {
                return Some((i, entries));
            } else if e.is_merge() {
                if let Some((pos, children)) = self.search_recursive(e) {
                    let needle_position = i + pos;
                    let mut insert_position = i;
                    for child in children.into_iter() {
                        insert_position += 1;
                        entries.insert(insert_position, child);
                    }
                    return Some((needle_position, entries));
                }
            }
        }
        None
    }

    pub(crate) fn search_link_target(&mut self) {
        let link = &self.selected_commit().id().clone();
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
            let e = commit_option.unwrap();
            if e.is_commit_link() {
                continue;
            }

            if e.commit().id() == link {
                let delta = i - self.selected;
                if delta > 0 {
                    self.move_focus(delta, MoveDirection::Down);
                    return;
                }
            } else if e.is_merge() && e.is_folded() {
                let bellow = &e.commit().bellow().expect("Expected Merge").to_string();
                let link_id = &link.to_string();
                // Heuristic skip examining merge if link is ancestor of the first child
                if is_ancestor(self.working_dir.as_str(), link_id, bellow).unwrap() {
                    continue;
                }
                if let Some((pos, mut commits)) = search_link_recursive(
                    &self.working_dir,
                    e.commit(),
                    &self.subtree_modules,
                    &link,
                ) {
                    e.folded(false);
                    let level = e.level() + 1;
                    let needle_position = i + pos;
                    let mut insert_position = i;
                    let mut above_commit = Some(e.commit());
                    for c in commits.iter_mut() {
                        insert_position += 1;
                        let entry = HistoryEntry::new(
                            self.working_dir.clone(),
                            c.to_owned(),
                            level,
                            &self.subtree_modules,
                        );
                        self.history.insert(insert_position, entry);
                        above_commit = Some(self.history.get(insert_position).unwrap().commit());
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

        let mut above_commit = None;
        if let Some(v) = self.history.last() {
            above_commit = Some(v.commit());
        }

        if let Ok(tmp) = commits_for_range(
            working_dir,
            range,
            self.paths.as_ref(),
            Some(skip),
            Some(max),
        ) {
            let result = !tmp.is_empty();
            let working_dir = self.working_dir.clone();
            let subtrees = &self.subtree_modules;
            let mut above_commit = if self.history.is_empty() {
                None
            } else {
                Some(self.history.last().unwrap().commit())
            };
            for c in tmp.into_iter() {
                if above_commit.is_some()
                    && above_commit.unwrap().is_merge()
                    && c.fork_points_calculation_needed()
                {
                    self.fork_point_thread.send(ForkPointRequest {
                        first: c.id().clone(),
                        second: above_commit.unwrap().children().first().unwrap().clone(),
                        working_dir: self.working_dir.clone(),
                    });
                }
                let entry = HistoryEntry::new(working_dir.clone(), c, 0, subtrees);
                self.history.push(entry);
                above_commit = Some(self.history.last().unwrap().commit());
            }
            return result;
        }
        false
    }
    fn selected_entry(&self) -> &HistoryEntry {
        self.history.get(self.selected).unwrap()
    }

    fn selected_commit(&self) -> &Commit {
        self.history.get(self.selected).as_ref().unwrap().commit()
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
        if self.history.is_empty() || (end >= self.history.len() - 1 && end < self.length) {
            let max = end + 1 - self.history.len();
            self.fill_up(max);
        }

        for (i, c) in self.history.iter_mut().enumerate() {
            c.selected(i == self.selected);
        }

        while let Ok(v) = self.fork_point_thread.try_recv() {
            for e in self.history.iter_mut() {
                if e.commit().id() == &v.oid {
                    e.commit_mut().fork_point(v.value);
                    break;
                }
            }
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
                if self.selected_entry().is_merge() && !self.selected_entry().is_folded() {
                    self.toggle_folding();
                } else if self.selected_entry().level() > 0 {
                    // move to last parent node
                    let mut cur = self.selected;
                    let expected_level = self.selected_entry().level() - 1;
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
                if self.selected_item().is_commit_link() {
                    self.search_link_target();
                } else if self.selected_item().is_merge() && self.selected_entry().is_folded() {
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
                if self.selected_item().is_commit_link() {
                    self.search_link_target();
                } else if self.selected_item().is_merge() {
                    self.toggle_folding();
                }
                EventResult::Consumed(None)
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

    fn selected_item(&self) -> &HistoryEntry {
        self.history.get(self.selected).as_ref().unwrap()
    }
}
