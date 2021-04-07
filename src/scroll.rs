use cursive::event::{Event, EventResult, Key};
use cursive::{Printer, Vec2, View};

struct ViewPort {
    top: usize,
    bottom: usize,
}

pub struct CustomScrollView<V> {
    inner: V,
    view_port: ViewPort,
}

impl<V> CustomScrollView<V> {
    pub fn new(inner: V) -> Self {
        CustomScrollView {
            inner,
            view_port: ViewPort { top: 0, bottom: 25 },
        }
    }
}

impl<V> View for CustomScrollView<V>
where
    V: View + Scrollable + Selectable,
{
    fn draw(&self, printer: &Printer) {
        let printer = &printer.content_offset(Vec2 {
            x: 0,
            y: self.view_port.top,
        });
        self.inner.draw(printer);
    }

    fn layout(&mut self, size: Vec2) {
        self.view_port.bottom = self.view_port.top + size.y - 1;
        self.inner.layout(size);
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // log::info!("Event: {:?}", event);
        match event {
            Event::Key(Key::Up) => {
                if self.inner.move_focus(1, MoveDirection::Up) {
                    let sel = self.inner.selected();
                    if sel < self.view_port.top {
                        let height = self.view_port.bottom - self.view_port.top;
                        self.view_port.top = sel;
                        self.view_port.bottom = self.view_port.top + height;
                    }
                }
                EventResult::Consumed(None)
            }
            Event::Key(Key::Down) => {
                if self.inner.move_focus(1, MoveDirection::Down) {
                    let sel = self.inner.selected();
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
                } else if self.inner.selected() < self.inner.length() - 1 {
                    self.inner.move_focus(
                        self.inner.length() - 1 - self.inner.selected(),
                        MoveDirection::Down,
                    );
                }

                EventResult::Consumed(None)
            }
            Event::Key(Key::PageUp) => {
                if self.view_port.top != 0 {
                    let n = self.view_port.bottom - self.view_port.top;
                    let top = self.view_port.top - n - 1;
                    let bottom = top + n;
                    self.view_port.top = top;
                    self.view_port.bottom = bottom;
                    self.inner.move_focus(n + 1, MoveDirection::Up);
                } else if self.inner.selected() != 0 {
                    self.inner.move_focus(
                        self.inner.selected() - self.view_port.top,
                        MoveDirection::Up,
                    );
                }
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored,
        }
    }
}

pub trait Scrollable {
    fn length(&self) -> usize;
}

#[derive(Eq, PartialEq)]
pub enum MoveDirection {
    Up,
    Down,
}

pub trait Selectable {
    fn selected(&self) -> usize;
    fn move_focus(&mut self, n: usize, direction: MoveDirection) -> bool;
}
