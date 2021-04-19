use cursive::event::{Event, EventResult, Key};
use cursive::{Printer, Rect, Vec2, View};

use crate::history_entry::HistoryEntry;
use crate::scroll::ScrollableSelectable;

enum FocusedView {
    MAIN,
    ASIDE,
}

pub struct DynamicSplitView<V1, V2> {
    main: V1,
    aside: V2,
    aside_visible: bool,
    focused: FocusedView,
}

impl<V1, V2> DynamicSplitView<V1, V2> {
    pub fn new(main: V1, aside: V2) -> Self {
        let aside_visible = false;
        DynamicSplitView {
            main,
            aside,
            aside_visible,
            focused: FocusedView::MAIN,
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
            let aside_size;
            let main_size;

            let horizontal = printer.size.x < 160;
            if horizontal {
                aside_size = Vec2 {
                    x: printer.size.x,
                    y: printer.size.y / 2,
                };
                main_size = Vec2 {
                    x: printer.size.x,
                    y: printer.size.y - printer.size.y / 2,
                };
            } else {
                aside_size = Vec2 {
                    x: printer.size.x / 2,
                    y: printer.size.y,
                };
                main_size = Vec2 {
                    x: printer.size.x - printer.size.x / 2,
                    y: printer.size.y,
                };
            }

            let main_rect = Rect::from_size(printer.offset, main_size);
            let main_printer = printer.windowed(main_rect);
            self.main.draw(&main_printer);

            let aside_rect;
            if horizontal {
                aside_rect = Rect::from_size(
                    Vec2 {
                        x: 0,
                        y: main_size.y,
                    },
                    aside_size,
                );
            } else {
                aside_rect = Rect::from_size(
                    Vec2 {
                        x: main_size.x,
                        y: 0,
                    },
                    aside_size,
                );
            }
            let aside_printer = printer.windowed(aside_rect);
            self.aside.draw(&aside_printer);
        }
    }

    fn layout(&mut self, size: Vec2) {
        if !self.aside_visible {
            self.main.layout(size);
        } else {
            let aside_size;
            let main_size;

            if size.x < 160 {
                aside_size = Vec2 {
                    x: size.x,
                    y: size.y / 2,
                };
                main_size = Vec2 {
                    x: size.x,
                    y: size.y - size.y / 2,
                };
            } else {
                aside_size = Vec2 {
                    x: size.x / 2,
                    y: size.y,
                };
                main_size = Vec2 {
                    x: size.x - size.x / 2,
                    y: size.y,
                };
            }
            self.main.layout(main_size);
            self.aside.layout(aside_size);
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        return constraint;
    }
    fn on_event(&mut self, e: Event) -> EventResult {
        return match self.focused {
            FocusedView::MAIN => match self.main.on_event(e.clone()) {
                EventResult::Ignored => match e {
                    Event::Key(Key::Enter) => {
                        self.aside.set_detail(self.main.selected_item());
                        self.focused = FocusedView::ASIDE;
                        self.aside_visible = true;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Tab) => {
                        self.focused = FocusedView::ASIDE;
                        EventResult::Consumed(None)
                    }
                    _ => {
                        log::warn!("MAIN: Unexpected key {:?}", e);
                        EventResult::Ignored
                    }
                },
                EventResult::Consumed(callback) => EventResult::Consumed(callback),
            },
            FocusedView::ASIDE => match self.aside.on_event(e.clone()) {
                EventResult::Ignored => match e {
                    Event::Char('q') => {
                        self.aside_visible = false;
                        self.focused = FocusedView::MAIN;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Tab) => {
                        self.focused = FocusedView::MAIN;
                        EventResult::Consumed(None)
                    }
                    _ => {
                        log::warn!("ASIDE: Unexpected key {:?}", e);
                        EventResult::Ignored
                    }
                },
                EventResult::Consumed(callback) => EventResult::Consumed(callback),
            },
        };
    }
}

pub trait DetailView {
    fn set_detail(&mut self, detail: &HistoryEntry);
}
