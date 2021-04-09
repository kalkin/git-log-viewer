use cursive::event::{Event, EventResult, Key};
use cursive::{Printer, Rect, Vec2, View};

use glv_core::Commit;

use crate::scroll::ScrollableSelectable;

pub struct DynamicSplitView<V1, V2> {
    main: V1,
    aside: V2,
    aside_visible: bool,
}

impl<V1, V2> DynamicSplitView<V1, V2> {
    pub fn new(main: V1, aside: V2) -> Self {
        let aside_visible = false;
        DynamicSplitView {
            main,
            aside,
            aside_visible,
        }
    }
}

impl<V1, V2> View for DynamicSplitView<V1, V2>
where
    V1: ScrollableSelectable + View,
    V2: View + DetailView,
{
    fn draw(&self, printer: &Printer) {
        if !self.aside_visible {
            self.main.draw(printer);
        } else {
            let aside_size = Vec2 {
                x: printer.size.x,
                y: printer.size.y / 2,
            };
            let main_size = Vec2 {
                x: printer.size.x,
                y: printer.size.y - printer.size.y / 2,
            };

            let main_rect = Rect::from_size(printer.offset, main_size);
            let main_printer = printer.windowed(main_rect);
            self.main.draw(&main_printer);

            let aside_rect = Rect::from_size(
                Vec2 {
                    x: 0,
                    y: main_size.y,
                },
                aside_size,
            );
            let aside_printer = printer.windowed(aside_rect);
            self.aside.draw(&aside_printer);
        }
    }

    fn layout(&mut self, size: Vec2) {
        if !self.aside_visible {
            self.main.layout(size);
        } else {
            let aside_size = Vec2 {
                x: size.x,
                y: size.y / 2,
            };
            let main_size = Vec2 {
                x: size.x,
                y: size.y - size.y / 2,
            };

            self.main.layout(main_size);
            self.aside.layout(aside_size);
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        return constraint;
    }
    fn on_event(&mut self, e: Event) -> EventResult {
        match (self.aside_visible, &e) {
            (false, Event::Key(Key::Enter)) => {
                self.aside.set_detail(self.main.selected_item());
                self.aside_visible = true;
                EventResult::Consumed(None)
            }
            (false, _) => self.main.on_event(e),
            (true, Event::Char('q')) => {
                self.aside_visible = false;
                return EventResult::Consumed(None);
            }
            (true, _) => self.aside.on_event(e),
        }
    }
}

pub trait DetailView {
    fn set_detail(&mut self, detail: &Commit);
}
