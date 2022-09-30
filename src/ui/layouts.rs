// Copyright (C) 2021  Bahtiar `kalkin-` Gadimov <bahtiar@gadimov.de>
//
// This file is part of git-log-viewer
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

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
    pub const fn new(main: Main, aside: Aside) -> Self {
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
            #[allow(clippy::arithmetic)]
            // arithmetic: division by 2 is safe
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
                debug_assert_eq!(result.len(), aside_result.len());
                for (i, row) in aside_result.iter_mut().enumerate() {
                    let right_row = result.get_mut(i).expect("row");
                    if line_length(right_row) < main_size.width() {
                        let max = main_size.width() - line_length(right_row);
                        let spaces: Vec<String> = (0..max).map(|_| " ".to_owned()).collect();
                        let content = spaces.join("");
                        right_row
                            .content
                            .push(StyledContent::new(ContentStyle::default(), content));
                    }
                    right_row.content.append(&mut row.content);
                }
                result
            }
        } else {
            self.main.render(area)
        }
    }

    fn on_event(&mut self, event: &Event) -> HandleEvent {
        if self.aside_visible {
            match self.aside.on_event(event) {
                HandleEvent::Handled => HandleEvent::Handled,
                HandleEvent::Ignored => match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::NONE,
                        ..
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
                        ..
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
