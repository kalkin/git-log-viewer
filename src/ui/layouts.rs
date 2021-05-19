use std::marker::PhantomData;

use crate::ui::base::{line_length, Area, Drawable, HandleEvent, Selectable, StyledArea};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{ContentStyle, StyledContent};

pub trait DetailsWidget<T>: Drawable {
    fn set_content(&mut self, content: &T);
}

pub struct SplitLayout<Main, Aside, T>
where
    Main: Drawable + Selectable<T>,
    Aside: DetailsWidget<T>,
{
    main: Main,
    aside: Aside,
    aside_visible: bool,
    _selected: PhantomData<T>,
}

impl<Main, Aside, T> SplitLayout<Main, Aside, T>
where
    Main: Drawable + Selectable<T>,
    Aside: DetailsWidget<T>,
{
    pub fn new(main: Main, aside: Aside) -> Self {
        Self {
            main,
            aside,
            aside_visible: false,
            _selected: PhantomData,
        }
    }
}

impl<Main, Aside, T> Drawable for SplitLayout<Main, Aside, T>
where
    Main: Drawable + Selectable<T>,
    Aside: DetailsWidget<T>,
{
    fn render(&mut self, area: &Area) -> StyledArea<String> {
        if self.aside_visible {
            let aside_size;
            let main_size;

            let horizontal_split = area.width() < 160;
            if horizontal_split {
                aside_size = Area::new(area.width(), area.height() / 2);
                main_size = Area::new(area.width(), area.height() - area.height() / 2);
                let mut result = self.main.render(&main_size);
                for s in self.aside.render(&aside_size) {
                    result.push(s);
                }
                result
            } else {
                aside_size = Area::new(area.width() / 2, area.height());
                main_size = Area::new(area.width() - area.width() / 2, area.height());
                let mut result = self.main.render(&main_size);
                let mut aside_result = self.aside.render(&aside_size);
                assert_eq!(result.len(), aside_result.len());
                for (i, row) in aside_result.iter_mut().enumerate() {
                    let right_row = result.get_mut(i).expect("row");
                    if line_length(right_row) < main_size.width() {
                        let max = main_size.width() - line_length(right_row);
                        let spaces: Vec<String> = (0..max).map(|_| " ".to_string()).collect();
                        let content = spaces.join("");
                        right_row.push(StyledContent::new(ContentStyle::default(), content));
                    }
                    right_row.append(row);
                }
                result
            }
        } else {
            self.main.render(area)
        }
    }

    fn on_event(&mut self, event: Event) -> HandleEvent {
        if self.aside_visible {
            match self.aside.on_event(event) {
                HandleEvent::Handled => HandleEvent::Handled,
                HandleEvent::Ignored => match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::NONE,
                    }) => {
                        self.aside_visible = false;
                        HandleEvent::Handled
                    }
                    _ => HandleEvent::Ignored,
                },
            }
        } else {
            match self.main.on_event(event) {
                HandleEvent::Handled => HandleEvent::Handled,
                HandleEvent::Ignored => match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                    }) => {
                        self.aside_visible = true;
                        self.aside.set_content(self.main.selected_item());
                        HandleEvent::Handled
                    }
                    _ => HandleEvent::Ignored,
                },
            }
        }
    }
}
