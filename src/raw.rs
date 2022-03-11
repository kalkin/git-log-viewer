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

use crate::ui::base::{StyledArea, StyledLine};
use crossterm::style::{Attribute, Color, ContentStyle, StyledContent};

struct Counter {
    style: ContentStyle,
    buf: String,
    lines: StyledArea<String>,
    cur_line: StyledLine<String>,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            style: ContentStyle::default(),
            buf: String::new(),
            lines: vec![],
            cur_line: vec![],
        }
    }

    fn save_cur_span(&mut self) {
        if !self.buf.is_empty() {
            self.cur_line
                .push(StyledContent::new(self.style, self.buf.clone()));
            self.buf = String::new();
        }
    }
}

impl vte::Perform for Counter {
    fn print(&mut self, c: char) {
        self.buf.push(c);
    }
    fn execute(&mut self, byte: u8) {
        if byte == 10 {
            self.save_cur_span();
            self.lines.push(self.cur_line.clone());
            self.cur_line = vec![];
        } else {
            self.buf.push(byte.try_into().expect("u8 to char"));
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _c: char) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _c: char,
    ) {
        self.save_cur_span();
        let mut iter = params.iter();
        while let Some(byte) = iter.next() {
            // TODO not sure if all colors match
            match byte[0] {
                0 => self.style = ContentStyle::new(),
                1 => self.style.attributes.set(Attribute::Bold),
                2 => self.style.attributes.set(Attribute::Italic),
                4 => self.style.attributes.set(Attribute::Underlined),
                5 => self.style.attributes.set(Attribute::SlowBlink),
                7 => self.style.attributes.set(Attribute::Reverse),

                30 => self.style.foreground_color = Some(Color::Black),
                31 => self.style.foreground_color = Some(Color::DarkRed),
                32 => self.style.foreground_color = Some(Color::DarkGreen),
                33 => self.style.foreground_color = Some(Color::DarkYellow),
                34 => self.style.foreground_color = Some(Color::DarkBlue),
                35 => self.style.foreground_color = Some(Color::DarkMagenta),
                36 => self.style.foreground_color = Some(Color::DarkCyan),
                37 => self.style.foreground_color = Some(Color::Grey),

                38 => {
                    let kind = iter.next().unwrap()[0];
                    match kind {
                        5 => {
                            #[allow(clippy::cast_possible_truncation)]
                            let color = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            self.style.foreground_color = Some(Color::AnsiValue(color));
                        }
                        2 => {
                            #[allow(clippy::cast_possible_truncation)]
                            let r = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            #[allow(clippy::cast_possible_truncation)]
                            let g = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            #[allow(clippy::cast_possible_truncation)]
                            let b = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            self.style.foreground_color = Some(Color::from((r, g, b)));
                        }
                        x => log::warn!("Unexpected CSI value 38;{:?}", x),
                    }
                }

                40 => self.style.background_color = Some(Color::Black),
                41 => self.style.background_color = Some(Color::DarkRed),
                42 => self.style.background_color = Some(Color::DarkGreen),
                43 => self.style.background_color = Some(Color::DarkYellow),
                44 => self.style.background_color = Some(Color::DarkBlue),
                45 => self.style.background_color = Some(Color::DarkMagenta),
                46 => self.style.background_color = Some(Color::DarkCyan),
                47 => self.style.background_color = Some(Color::Grey),

                48 => {
                    let kind = iter.next().unwrap()[0];
                    match kind {
                        5 => {
                            #[allow(clippy::cast_possible_truncation)]
                            let color = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            self.style.background_color = Some(Color::AnsiValue(color));
                        }
                        2 => {
                            #[allow(clippy::cast_possible_truncation)]
                            let r = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            #[allow(clippy::cast_possible_truncation)]
                            let g = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            #[allow(clippy::cast_possible_truncation)]
                            let b = iter.next().unwrap()[0].try_into().expect("usize to u8");
                            self.style.background_color = Some(Color::from((r, g, b)));
                        }
                        x => log::warn!("Unexpected CS value 48;{:?}", x),
                    }
                }

                90 => self.style.foreground_color = Some(Color::DarkGrey),
                91 => self.style.foreground_color = Some(Color::Red),
                92 => self.style.foreground_color = Some(Color::Green),
                93 => self.style.foreground_color = Some(Color::Yellow),
                94 => self.style.foreground_color = Some(Color::Blue),
                95 => self.style.foreground_color = Some(Color::Magenta),
                96 => self.style.foreground_color = Some(Color::Cyan),
                97 => self.style.foreground_color = Some(Color::White),

                100 => self.style.background_color = Some(Color::DarkGrey),
                101 => self.style.background_color = Some(Color::Red),
                102 => self.style.background_color = Some(Color::Green),
                103 => self.style.background_color = Some(Color::Yellow),
                104 => self.style.background_color = Some(Color::Blue),
                105 => self.style.background_color = Some(Color::Magenta),
                106 => self.style.background_color = Some(Color::Cyan),
                107 => self.style.background_color = Some(Color::White),

                x => log::warn!("Unexpected CSI value {:?}", x),
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

pub fn parse_spans(vec: Vec<u8>) -> Vec<StyledLine<String>> {
    let mut statemachine = vte::Parser::new();
    let mut performer = Counter::new();
    for u in vec {
        statemachine.advance(&mut performer, u);
    }
    performer.lines
}
