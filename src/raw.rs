use cursive::theme::{BaseColor, Color, ColorType, Effect, Style};
use cursive::utils::span::SpannedString;

struct Counter {
    style: Style,
    buf: String,
    lines: Vec<SpannedString<Style>>,
    cur_line: SpannedString<Style>,
}

impl Counter {
    pub fn new() -> Counter {
        Counter {
            style: cursive::theme::Style::none(),
            buf: String::new(),
            lines: Vec::new(),
            cur_line: SpannedString::new(),
        }
    }

    fn save_cur_span(&mut self) {
        if !self.buf.is_empty() {
            self.cur_line.append_styled(self.buf.clone(), self.style);
            self.buf = String::new();
        }
    }
}

impl vte::Perform for Counter {
    fn print(&mut self, c: char) {
        self.buf.push(c);
    }
    fn execute(&mut self, byte: u8) {
        self.buf.push(byte as char);
        if byte == 10 {
            self.save_cur_span();
            self.lines.push(self.cur_line.clone());
            self.cur_line = SpannedString::new();
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

            match cur {
                0 => self.style = Style::none(),
                1 => self.style.effects |= Effect::Bold,
                2 => self.style.effects |= Effect::Italic,
                4 => self.style.effects |= Effect::Underline,
                5 => self.style.effects |= Effect::Blink,
                7 => self.style.effects |= Effect::Reverse,

                30 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::Black)),
                31 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::Red)),
                32 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::Green)),
                33 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::Yellow)),
                34 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::Blue)),
                35 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::Magenta)),
                36 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::Cyan)),
                37 => self.style.color.front = ColorType::Color(Color::Dark(BaseColor::White)),

                38 => {
                    assert!(iter.next().unwrap()[0] == 5);
                    #[allow(clippy::cast_possible_truncation)]
                    let color = iter.next().unwrap()[0] as u8;
                    self.style.color.front = ColorType::Color(Color::from_256colors(color))
                }

                40 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::Black)),
                41 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::Red)),
                42 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::Green)),
                43 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::Yellow)),
                44 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::Blue)),
                45 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::Magenta)),
                46 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::Cyan)),
                47 => self.style.color.back = ColorType::Color(Color::Dark(BaseColor::White)),

                48 => {
                    assert!(iter.next().unwrap()[0] == 5);
                    #[allow(clippy::cast_possible_truncation)]
                    let color = iter.next().unwrap()[0] as u8;
                    self.style.color.back = ColorType::Color(Color::from_256colors(color))
                }

                90 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::Black)),
                91 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::Red)),
                92 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::Green)),
                93 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::Yellow)),
                94 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::Blue)),
                95 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::Magenta)),
                96 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::Cyan)),
                97 => self.style.color.front = ColorType::Color(Color::Light(BaseColor::White)),

                100 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::Black)),
                101 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::Red)),
                102 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::Green)),
                103 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::Yellow)),
                104 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::Blue)),
                105 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::Magenta)),
                106 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::Cyan)),
                107 => self.style.color.back = ColorType::Color(Color::Light(BaseColor::White)),

                _ => panic!("NIY handling for “{}”", cur),
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

pub fn parse_spans(vec: Vec<u8>) -> Vec<SpannedString<Style>> {
    let mut statemachine = vte::Parser::new();
    let mut performer = Counter::new();
    for u in vec {
        statemachine.advance(&mut performer, u);
    }
    performer.lines
}
