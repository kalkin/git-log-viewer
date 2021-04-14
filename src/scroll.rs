use cursive::direction::Direction;
use cursive::event::{Event, EventResult, Key};
use cursive::views::EditView;
use cursive::{Printer, Vec2, View};

use crate::core::Commit;

use crate::search::{SearchDirection, SearchState};
use crate::style::DEFAULT_STYLE;

struct ViewPort {
    top: usize,
    bottom: usize,
}

enum CustomScrollFocus {
    CONTENT,
    SEARCH,
}

pub struct CustomScrollView<V> {
    inner: V,
    search_state: SearchState,
    search_input: Option<EditView>,
    view_port: ViewPort,
    focus: CustomScrollFocus,
}

impl<V> CustomScrollView<V> {
    pub fn new(inner: V) -> Self {
        let search_state = SearchState::new(DEFAULT_STYLE.to_owned());
        CustomScrollView {
            inner,
            search_state,
            search_input: None,
            view_port: ViewPort { top: 0, bottom: 25 },
            focus: CustomScrollFocus::CONTENT,
        }
    }
}

impl<V> ScrollableSelectable for CustomScrollView<V>
where
    V: View + ScrollableSelectable,
{
    fn length(&self) -> usize {
        self.inner.length()
    }

    fn move_focus(&mut self, n: usize, direction: MoveDirection) -> bool {
        self.inner.move_focus(n, direction)
    }

    fn search(&mut self, search_state: SearchState) {
        self.inner.search(search_state);
    }

    fn selected_pos(&self) -> usize {
        self.inner.selected_pos()
    }

    fn selected_item(&self) -> &Commit {
        self.inner.selected_item()
    }
}

impl<V> View for CustomScrollView<V>
where
    V: View + ScrollableSelectable,
{
    fn draw(&self, printer: &Printer) {
        let printer = &printer.content_offset(Vec2 {
            x: 0,
            y: self.view_port.top,
        });
        if let Some(input) = &self.search_input {
            log::info!(
                "Original printer [{:?}] ({:?}) {:?}",
                printer.content_offset,
                printer.offset,
                printer.size
            );
            let history_printer = printer.inner_size(Vec2 {
                x: printer.size.x,
                y: printer.size.y - 1,
            });
            log::info!(
                "History printer [{:?}] ({:?}) {:?}",
                history_printer.content_offset,
                history_printer.offset,
                history_printer.size
            );
            let search_printer = printer
                .offset(Vec2 {
                    x: printer.content_offset.x,
                    y: printer.content_offset.y + history_printer.size.y,
                })
                .inner_size(Vec2 {
                    x: printer.size.x,
                    y: 1,
                });
            log::info!(
                "Search printer [{:?}] ({:?}) {:?}",
                search_printer.content_offset,
                search_printer.offset,
                search_printer.size
            );
            self.inner.draw(&history_printer);
            input.draw(&search_printer);
        } else {
            self.inner.draw(printer)
        }
    }

    fn layout(&mut self, size: Vec2) {
        let new_size;
        if let Some(search_input) = self.search_input.as_mut() {
            new_size = Vec2 {
                x: size.x,
                y: size.y - 1,
            };
            search_input.layout(Vec2 { x: size.x, y: 1 });
        } else {
            new_size = size;
        }
        self.view_port.bottom = self.view_port.top + new_size.y - 1;
        let range = self.view_port.top..self.view_port.bottom;
        let selected = self.inner.selected_pos();
        if !range.contains(&selected) {
            if selected > self.view_port.bottom {
                let height = self.view_port.bottom - self.view_port.top;
                self.view_port.bottom = self.selected_pos();
                self.view_port.top = self.view_port.bottom - height;
            } else if selected < self.view_port.top {
                let height = self.view_port.bottom - self.view_port.top;
                self.view_port.top = self.inner.selected_pos();
                self.view_port.bottom = self.view_port.top + height;
            }
        }
        self.inner.layout(new_size);
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match &self.focus {
            CustomScrollFocus::CONTENT => match event {
                Event::Char('?') => {
                    let mut t = EditView::new();
                    t.set_enabled(true);
                    self.search_input = Some(t);
                    self.search_state.direction = SearchDirection::Backward;
                    self.focus = CustomScrollFocus::SEARCH;
                    self.search_input
                        .as_mut()
                        .unwrap()
                        .take_focus(Direction::down());

                    EventResult::Consumed(None)
                }
                Event::Char('n') => {
                    if self.search_state.active {
                        self.search(self.search_state.clone());
                        return EventResult::Consumed(None);
                    }
                    EventResult::Ignored
                }
                Event::Char('N') => {
                    if self.search_state.active {
                        let mut new_state = self.search_state.clone();
                        match new_state.direction {
                            SearchDirection::Forward => {
                                new_state.direction = SearchDirection::Backward
                            }
                            SearchDirection::Backward => {
                                new_state.direction = SearchDirection::Forward
                            }
                        }
                        self.search(new_state);
                        return EventResult::Consumed(None);
                    }
                    EventResult::Ignored
                }
                Event::Char('/') => {
                    let mut t = EditView::new();
                    t.set_enabled(true);
                    self.search_input = Some(t);
                    self.search_state.direction = SearchDirection::Forward;
                    self.focus = CustomScrollFocus::SEARCH;
                    self.search_input
                        .as_mut()
                        .unwrap()
                        .take_focus(Direction::down());

                    EventResult::Consumed(None)
                }
                Event::Key(Key::Up) | Event::Char('k') => {
                    if self.inner.move_focus(1, MoveDirection::Up) {
                        let sel = self.inner.selected_pos();
                        if sel < self.view_port.top {
                            let height = self.view_port.bottom - self.view_port.top;
                            self.view_port.top = sel;
                            self.view_port.bottom = self.view_port.top + height;
                        }
                    }
                    EventResult::Consumed(None)
                }
                Event::Key(Key::Down) | Event::Char('j') => {
                    if self.inner.move_focus(1, MoveDirection::Down) {
                        let sel = self.inner.selected_pos();
                        if sel >= self.view_port.bottom {
                            let height = self.view_port.bottom - self.view_port.top;
                            self.view_port.top = sel - height;
                            self.view_port.bottom = sel;
                        }
                    }
                    EventResult::Consumed(None)
                }
                Event::Key(Key::PageDown) => {
                    if self.view_port.bottom < self.inner.length() {
                        let n = self.view_port.bottom - self.view_port.top;
                        let top = self.view_port.top + n + 1;
                        let bottom = top + n;
                        self.view_port.top = top;
                        self.view_port.bottom = bottom;
                        self.inner.move_focus(n + 1, MoveDirection::Down);
                    } else if self.inner.selected_pos() < self.inner.length() - 1 {
                        self.inner.move_focus(
                            self.inner.length() - 1 - self.inner.selected_pos(),
                            MoveDirection::Down,
                        );
                    }

                    EventResult::Consumed(None)
                }
                Event::Key(Key::PageUp) => {
                    if self.view_port.top != 0 {
                        let n = self.view_port.bottom - self.view_port.top;
                        let top;
                        if n + 1 > self.view_port.top {
                            top = 0;
                        } else {
                            top = self.view_port.top - n - 1;
                        }
                        let bottom = top + n;
                        self.view_port.top = top;
                        self.view_port.bottom = bottom;
                        self.inner.move_focus(n + 1, MoveDirection::Up);
                    } else if self.inner.selected_pos() != 0 {
                        self.inner.move_focus(
                            self.inner.selected_pos() - self.view_port.top,
                            MoveDirection::Up,
                        );
                    }
                    EventResult::Consumed(None)
                }
                _ => self.inner.on_event(event),
            },
            CustomScrollFocus::SEARCH => match event {
                Event::Key(Key::Esc) => {
                    self.focus = CustomScrollFocus::CONTENT;
                    self.search_state.active = false;
                    self.search_input = None;

                    EventResult::Consumed(None)
                }
                Event::Key(Key::Enter) => {
                    self.focus = CustomScrollFocus::CONTENT;
                    self.search_input.as_mut().unwrap().disable();
                    let needle = self
                        .search_input
                        .as_ref()
                        .unwrap()
                        .get_content()
                        .to_string();
                    self.search_state.active = true;
                    self.search_state.needle = needle;
                    self.search(self.search_state.clone());
                    EventResult::Consumed(None)
                }
                _ => self.search_input.as_mut().unwrap().on_event(event),
            },
        }
    }
}

pub trait ScrollableSelectable {
    fn length(&self) -> usize;
    fn move_focus(&mut self, n: usize, direction: MoveDirection) -> bool;
    fn search(&mut self, search_state: SearchState);
    fn selected_pos(&self) -> usize;
    fn selected_item(&self) -> &Commit;
}

#[derive(Eq, PartialEq)]
pub enum MoveDirection {
    Up,
    Down,
}
