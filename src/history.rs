use cursive::direction::Direction;
use cursive::event::{Event, EventResult, Key};
use cursive::theme::*;
use cursive::utils::span::{SpannedStr, SpannedString};
use cursive::views::EditView;
use cursive::{Printer, Rect, Vec2, XY};
use unicode_width::UnicodeWidthStr;

use posix_errors::PosixError;

use crate::scroll::{MoveDirection, ScrollableSelectable};
use crate::search::{SearchState, SearchableCommit, WidthConfig};
use crate::style::DEFAULT_STYLE;
use git_subtrees_improved::{subtrees, SubtreeConfig};
use glv_core::*;

enum HistoryFocus {
    History,
    Search,
}

pub struct History {
    range: String,
    history: Vec<Commit>,
    selected: usize,
    length: usize,
    working_dir: String,
    subtree_modules: Vec<SubtreeConfig>,
    search_state: SearchState,
    search_input: Option<EditView>,
    focused: HistoryFocus,
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
            search_input: None,
            focused: HistoryFocus::History,
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

        let sc = SearchableCommit::new(style, commit, &self.search_state, width_config);

        {
            buf.append(sc.short_id());
            buf.append_styled(" ", style);
        }

        {
            // Author date
            buf.append(sc.author_rel_date());
            buf.append_styled(" ", style);
        }

        {
            // Author name
            buf.append(sc.author_name());
            buf.append_styled(" ", style);
        }

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

        if let Some(modules) = sc.modules() {
            buf.append(modules);
            buf.append_styled(" ", style);
        }

        {
            buf.append(sc.subject());
            buf.append_styled(" ", style);
        }
        buf.append(sc.references());

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
    }

    fn render_history(&self, printer: &Printer) {
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
                buf = self.render_commit(commit, render_config);
                let t = SpannedStr::from(&buf);
                printer.print_styled((0, x), t);
            } else {
                break;
            }
        }
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
        if let Some(input) = &self.search_input {
            let history_printer = printer.inner_size(Vec2 {
                x: printer.size.x,
                y: printer.size.y - 1,
            });
            let search_printer = printer
                .offset(Vec2 {
                    x: printer.content_offset.x,
                    y: printer.content_offset.y + history_printer.size.y,
                })
                .inner_size(Vec2 {
                    x: printer.size.x,
                    y: 1,
                });
            self.render_history(&history_printer);
            input.draw(&search_printer);
        } else {
            self.render_history(printer)
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
        if let Some(input) = self.search_input.as_mut() {
            input.layout(Vec2 { x: size.x, y: 1 });
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        Vec2 {
            x: constraint.x,
            y: 200,
        }
    }

    fn on_event(&mut self, e: Event) -> EventResult {
        match &self.focused {
            HistoryFocus::History => match e {
                Event::Key(Key::Esc) => {
                    if self.search_input.is_some() {
                        self.search_input = None;
                        self.search_state.active = false;
                        EventResult::Consumed(None)
                    } else {
                        EventResult::Ignored
                    }
                }
                Event::Char('/') => {
                    let mut t = EditView::new();
                    t.set_enabled(true);
                    self.search_input = Some(t);
                    self.focused = HistoryFocus::Search;
                    self.search_input
                        .as_mut()
                        .unwrap()
                        .take_focus(Direction::down());
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
            },
            HistoryFocus::Search => match e {
                Event::Key(Key::Esc) => {
                    self.focused = HistoryFocus::History;
                    self.search_state.active = false;
                    self.search_input = None;

                    EventResult::Consumed(None)
                }
                Event::Key(Key::Enter) => {
                    self.focused = HistoryFocus::History;
                    self.search_input.as_mut().unwrap().disable();
                    let needle = self
                        .search_input
                        .as_ref()
                        .unwrap()
                        .get_content()
                        .to_string();
                    self.search_state.active = true;
                    self.search_state.needle = needle;
                    EventResult::Consumed(None)
                }
                _ => self.search_input.as_mut().unwrap().on_event(e),
            },
        }
    }

    fn take_focus(&mut self, d: Direction) -> bool {
        match self.focused {
            HistoryFocus::History => true,
            HistoryFocus::Search => self.search_input.as_mut().unwrap().take_focus(d),
        }
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
