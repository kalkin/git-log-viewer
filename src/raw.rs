use crate::ui::base::{StyledArea, StyledLine};
use crossterm::style::{Attribute, Color, ContentStyle, StyledContent};

struct Counter {
    style: ContentStyle,
    buf: String,
    lines: StyledArea<String>,
    cur_line: StyledLine<String>,
}

impl Counter {
    pub fn new() -> Counter {
        Counter {
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
            self.buf.push(byte as char);
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
        loop {
            let cur;
            match iter.next() {
                Some(byte) => cur = byte[0],
                None => break,
            }
            // TODO not sure if all colors match
            match cur {
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
                        5 =>  {
                            let color = iter.next().unwrap()[0] as u8;
                            self.style.foreground_color = Some(Color::AnsiValue(color));
                        }
                        2 =>  {
                            let r = iter.next().unwrap()[0] as u8;
                            let g = iter.next().unwrap()[0] as u8;
                            let b = iter.next().unwrap()[0] as u8;
                            self.style.foreground_color = Some(Color::from((r, g, b)));
                        }
                        x => panic!("Unexpected value {:?}", x),
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
                        5 =>  {
                            let color = iter.next().unwrap()[0] as u8;
                            self.style.background_color = Some(Color::AnsiValue(color));
                        }
                        2 =>  {
                            let r = iter.next().unwrap()[0] as u8;
                            let g = iter.next().unwrap()[0] as u8;
                            let b = iter.next().unwrap()[0] as u8;
                            self.style.background_color = Some(Color::from((r, g, b)));
                        }
                        x => panic!("Unexpected value {:?}", x),
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

                _ => panic!("NIY handling for “{}”", cur),
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
